//! Encrypted-at-rest session storage + refresh, for both the Liner API and
//! the YouTube TV session.
//!
//! The Electron implementation used `safeStorage` (OS keyring). iOS has no
//! equivalent reachable without extra entitlement work, so this port writes an
//! AES-256-GCM envelope whose key lives in a separate `0600` file inside
//! `Application Support`.
//!
//! Threat model, stated plainly: this protects a stolen *backup* and casual
//! filesystem inspection of a jailbroken device. It does **not** protect
//! against an attacker who can already read the app container, because the key
//! sits next to the ciphertext. Moving the key into the iOS Keychain is the
//! documented upgrade path (see `platform_ios::keychain`) and does not change
//! the on-disk envelope format.

use std::sync::Mutex;

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, Nonce};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{AppError, AppResult};
use crate::jsonstore::write_atomic;
use crate::paths::AppPaths;
use crate::signer::SignerState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredAuthSession {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    /// Everything else the server returned (`user`, `expiresAt`, …) is kept
    /// verbatim so logging the session back out is lossless.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl StoredAuthSession {
    pub fn parse(value: &Value) -> Option<Self> {
        let obj = value.as_object()?;
        let access = obj.get("accessToken")?.as_str()?;
        let refresh = obj.get("refreshToken")?.as_str()?;
        if access.is_empty() || refresh.is_empty() {
            return None;
        }
        Some(Self {
            access_token: access.to_string(),
            refresh_token: refresh.to_string(),
            extra: obj.clone(),
        })
    }

    fn to_value(&self) -> Value {
        let mut map = self.extra.clone();
        map.insert("accessToken".into(), Value::String(self.access_token.clone()));
        map.insert(
            "refreshToken".into(),
            Value::String(self.refresh_token.clone()),
        );
        Value::Object(map)
    }
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RefreshOutcome {
    Refreshed,
    Rotated,
    SignedOut,
    Transient,
    NoSession,
}

#[derive(Debug, Serialize)]
pub struct AuthRefreshResult {
    pub outcome: RefreshOutcome,
    pub session: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// One store instance per session kind (liner + youtube).
pub struct SessionStore {
    path: std::path::PathBuf,
    key_path: std::path::PathBuf,
    inner: Mutex<Option<StoredAuthSession>>,
    refresh_guard: tokio::sync::Mutex<()>,
}

impl SessionStore {
    pub fn new(path: std::path::PathBuf, key_path: std::path::PathBuf) -> Self {
        Self {
            path,
            key_path,
            inner: Mutex::new(None),
            refresh_guard: tokio::sync::Mutex::new(()),
        }
    }

    fn cipher(&self) -> AppResult<Aes256Gcm> {
        let key_bytes = load_or_create_key(&self.key_path)?;
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        Ok(Aes256Gcm::new(key))
    }

    /// Synchronous read used by `auth_get_session` (the renderer calls it
    /// during boot and cannot await).
    pub fn get(&self) -> Option<StoredAuthSession> {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if guard.is_some() {
            return guard.clone();
        }
        let loaded = self.read_from_disk();
        *guard = loaded.clone();
        loaded
    }

    pub fn get_value(&self) -> Option<Value> {
        self.get().map(|s| s.to_value())
    }

    pub fn set(&self, value: Option<Value>) -> AppResult<Option<Value>> {
        let next = match value {
            None => None,
            Some(v) if v.is_null() => None,
            Some(v) => match StoredAuthSession::parse(&v) {
                Some(session) => Some(session),
                None => {
                    return Err(AppError::Invalid(
                        "session must include non-empty accessToken and refreshToken".into(),
                    ))
                }
            },
        };

        {
            let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            *guard = next.clone();
        }

        match next {
            None => {
                let _ = std::fs::remove_file(&self.path);
                Ok(None)
            }
            Some(session) => {
                self.write_to_disk(&session)?;
                Ok(Some(session.to_value()))
            }
        }
    }

    fn read_from_disk(&self) -> Option<StoredAuthSession> {
        let raw = std::fs::read_to_string(&self.path).ok()?;
        let envelope: Value = serde_json::from_str(&raw).ok()?;
        let data = envelope.get("data")?.as_str()?;
        let bytes = base64_decode(data)?;

        let encrypted = envelope
            .get("encrypted")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let plain = if encrypted {
            let cipher = self.cipher().ok()?;
            let (nonce_bytes, ciphertext) = bytes.split_at_checked(12)?;
            let nonce = Nonce::from_slice(nonce_bytes);
            cipher.decrypt(nonce, ciphertext).ok()?
        } else {
            bytes
        };

        let value: Value = serde_json::from_slice(&plain).ok()?;
        StoredAuthSession::parse(&value)
    }

    fn write_to_disk(&self, session: &StoredAuthSession) -> AppResult<()> {
        let plain = serde_json::to_vec(&session.to_value())?;
        let cipher = self.cipher()?;
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let mut payload = nonce.to_vec();
        payload.extend_from_slice(
            &cipher
                .encrypt(&nonce, plain.as_slice())
                .map_err(|_| AppError::Other("session encryption failed".into()))?,
        );

        let envelope = serde_json::json!({
            "version": 1,
            "encrypted": true,
            "data": base64_encode(&payload),
        });
        write_atomic(&self.path, envelope.to_string().as_bytes())
    }

    /// Faithful port of `AuthSessionStore::runRefresh`.
    pub async fn refresh(
        &self,
        http: &reqwest::Client,
        api_base_url: &str,
        used_refresh_token: Option<String>,
        signer: &SignerState,
    ) -> AuthRefreshResult {
        let Some(current) = self.get() else {
            return AuthRefreshResult {
                outcome: RefreshOutcome::NoSession,
                session: None,
                reason: None,
            };
        };

        // A concurrent caller rotated the token before we got here.
        if let Some(used) = &used_refresh_token {
            if used != &current.refresh_token {
                return AuthRefreshResult {
                    outcome: RefreshOutcome::Rotated,
                    session: Some(current.to_value()),
                    reason: None,
                };
            }
        }

        if !is_trusted_api_base(api_base_url) {
            return AuthRefreshResult {
                outcome: RefreshOutcome::Transient,
                session: Some(current.to_value()),
                reason: Some("untrusted api base url".into()),
            };
        }

        // Single-flight: concurrent refreshes would burn the rotating token.
        let _guard = self.refresh_guard.lock().await;
        let current = match self.get() {
            Some(s) => s,
            None => {
                return AuthRefreshResult {
                    outcome: RefreshOutcome::NoSession,
                    session: None,
                    reason: None,
                }
            }
        };
        if let Some(used) = &used_refresh_token {
            if used != &current.refresh_token {
                return AuthRefreshResult {
                    outcome: RefreshOutcome::Rotated,
                    session: Some(current.to_value()),
                    reason: None,
                };
            }
        }

        const REFRESH_PATH: &str = "/v1/auth/refresh";
        const DEAD_CODES: [&str; 2] = ["INVALID_REFRESH_TOKEN", "REFRESH_SESSION_EXPIRED"];

        let base = api_base_url.trim_end_matches('/');
        let body = serde_json::json!({ "refreshToken": current.refresh_token }).to_string();

        let mut reason = String::new();
        for attempt in 0..2 {
            let signed = match signer.sign_request(&crate::signer::SignRequestInput {
                method: "POST".into(),
                path: REFRESH_PATH.into(),
                body: Some(body.clone()),
                timestamp: None,
            }) {
                Ok(s) => s,
                Err(err) => {
                    return AuthRefreshResult {
                        outcome: RefreshOutcome::Transient,
                        session: self.get_value(),
                        reason: Some(err.to_string()),
                    }
                }
            };

            let response = http
                .post(format!("{base}{REFRESH_PATH}"))
                .header("content-type", "application/json")
                .header("x-liner-signature", &signed.signature)
                .header("x-liner-timestamp", signed.timestamp.to_string())
                .header("x-liner-nonce", &signed.nonce)
                .body(body.clone())
                .send()
                .await;

            let response = match response {
                Ok(r) => r,
                Err(err) => {
                    return AuthRefreshResult {
                        outcome: RefreshOutcome::Transient,
                        session: self.get_value(),
                        reason: Some(err.to_string()),
                    }
                }
            };

            if let Some(date) = response.headers().get("date") {
                if let Some(parsed) = date.to_str().ok().and_then(parse_http_date_ms) {
                    signer.set_server_time_ms(parsed);
                }
            }

            let status = response.status();
            if status.is_success() {
                let value: Value = match response.json().await {
                    Ok(v) => v,
                    Err(err) => {
                        return AuthRefreshResult {
                            outcome: RefreshOutcome::Transient,
                            session: self.get_value(),
                            reason: Some(err.to_string()),
                        }
                    }
                };

                let mut merged = current.to_value();
                if let (Some(target), Some(source)) = (merged.as_object_mut(), value.as_object()) {
                    for (k, v) in source {
                        target.insert(k.clone(), v.clone());
                    }
                }
                let Some(next) = StoredAuthSession::parse(&merged) else {
                    return AuthRefreshResult {
                        outcome: RefreshOutcome::Transient,
                        session: self.get_value(),
                        reason: Some("malformed refresh response".into()),
                    };
                };

                // Someone else rotated while we were in flight.
                if self
                    .get()
                    .map(|s| s.refresh_token != current.refresh_token)
                    .unwrap_or(false)
                {
                    return AuthRefreshResult {
                        outcome: RefreshOutcome::Rotated,
                        session: self.get_value(),
                        reason: None,
                    };
                }

                let value = next.to_value();
                let _ = self.set(Some(value.clone()));
                return AuthRefreshResult {
                    outcome: RefreshOutcome::Refreshed,
                    session: Some(value),
                    reason: None,
                };
            }

            let code = response
                .json::<Value>()
                .await
                .ok()
                .and_then(|v| v.get("code").and_then(Value::as_str).map(str::to_string))
                .unwrap_or_default();
            reason = format!("{status} {code}").trim().to_string();

            if status.as_u16() != 401 && status.as_u16() != 403 {
                break;
            }
            if DEAD_CODES.contains(&code.as_str()) {
                let still_same = self
                    .get()
                    .map(|s| s.refresh_token == current.refresh_token)
                    .unwrap_or(false);
                if still_same {
                    let _ = self.set(None);
                }
                return AuthRefreshResult {
                    outcome: if self.get().is_some() {
                        RefreshOutcome::Rotated
                    } else {
                        RefreshOutcome::SignedOut
                    },
                    session: self.get_value(),
                    reason: Some(reason),
                };
            }
            let _ = attempt;
        }

        AuthRefreshResult {
            outcome: RefreshOutcome::Transient,
            session: self.get_value(),
            reason: Some(reason),
        }
    }
}

fn is_trusted_api_base(url: &str) -> bool {
    match url::Url::parse(url) {
        Ok(parsed) => match parsed.scheme() {
            "https" => true,
            "http" => matches!(
                parsed.host_str().unwrap_or_default(),
                "localhost" | "127.0.0.1" | "::1"
            ),
            _ => false,
        },
        Err(_) => false,
    }
}

fn load_or_create_key(path: &std::path::Path) -> AppResult<[u8; 32]> {
    if let Ok(bytes) = std::fs::read(path) {
        if bytes.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return Ok(key);
        }
    }

    let mut key = [0u8; 32];
    getrandom::getrandom(&mut key)
        .map_err(|e| AppError::Other(format!("rng unavailable: {e}")))?;
    write_atomic(path, &key)?;

    // Best effort: 0600 on POSIX. iOS honours this inside the container.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }

    Ok(key)
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

fn base64_decode(data: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.decode(data).ok()
}

/// Minimal RFC 1123 parser (`Sun, 06 Nov 1994 08:49:37 GMT`).
fn parse_http_date_ms(value: &str) -> Option<i64> {
    use chrono::{DateTime, Utc};
    DateTime::parse_from_rfc2822(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc).timestamp_millis())
}

pub struct AuthStore {
    pub liner: SessionStore,
    pub youtube: SessionStore,
}

impl AuthStore {
    pub fn new(paths: &AppPaths) -> Self {
        Self {
            liner: SessionStore::new(paths.session_file(), paths.session_key_file()),
            youtube: SessionStore::new(paths.youtube_session_file(), paths.session_key_file()),
        }
    }
}
