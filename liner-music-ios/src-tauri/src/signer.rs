//! Request / cover / telemetry signing.
//!
//! The desktop build ships `signer/` compiled to WASM and runs it in the
//! renderer. That is a poor fit for a phone: the WASM blob is a script asset
//! that can be lifted straight out of the bundle, and every signed request
//! pays an async boundary. Here the same crate is linked natively, so the
//! obfuscated keys never exist as a separate file and signing is a synchronous
//! HMAC over a few bytes of canonical string.
//!
//! Canonical form (identical to `signer/src/ffi.rs`):
//!
//! ```text
//! METHOD \n PATH \n TIMESTAMP \n NONCE \n SHA256_HEX(BODY)
//! ```

use std::sync::atomic::{AtomicI64, Ordering};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

pub struct SignerState {
    /// `server_time - device_time` in milliseconds. Kept on the native side so
    /// the timestamp we sign with matches the clock the server will validate
    /// against, even if the WebView's `Date.now()` has drifted.
    offset_ms: AtomicI64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignRequestInput {
    pub method: String,
    pub path: String,
    pub body: Option<String>,
    pub timestamp: Option<u64>,
}

#[derive(Debug, Serialize, Default)]
pub struct SignRequestResult {
    pub signature: String,
    pub timestamp: u64,
    pub nonce: String,
}

impl Default for SignerState {
    fn default() -> Self {
        Self {
            offset_ms: AtomicI64::new(0),
        }
    }
}

impl SignerState {
    pub fn set_server_time_ms(&self, server_ms: i64) -> i64 {
        let device_ms = now_ms();
        let offset = server_ms - device_ms;
        self.offset_ms.store(offset, Ordering::Relaxed);
        offset
    }

    pub fn offset_ms(&self) -> i64 {
        self.offset_ms.load(Ordering::Relaxed)
    }

    /// Epoch seconds adjusted by the last observed server offset.
    pub fn adjusted_timestamp(&self) -> u64 {
        let ms = now_ms() + self.offset_ms();
        if ms <= 0 {
            0
        } else {
            (ms / 1000) as u64
        }
    }

    pub fn sign_request(&self, input: &SignRequestInput) -> AppResult<SignRequestResult> {
        let method = input.method.trim().to_ascii_uppercase();
        let path = input.path.trim().to_string();
        let timestamp = input.timestamp.unwrap_or_else(|| self.adjusted_timestamp());
        let nonce = random_nonce();
        let body_hash = liner_signer::sha256_hex(input.body.as_deref().unwrap_or("").as_bytes());

        let canonical = format!("{method}\n{path}\n{timestamp}\n{nonce}\n{body_hash}");
        let signature = liner_signer::sign_api_base64url(canonical.as_bytes())
            .map_err(AppError::Other)?;

        Ok(SignRequestResult {
            signature,
            timestamp,
            nonce,
        })
    }

    pub fn sign_monitor_request(&self, input: &SignRequestInput) -> AppResult<SignRequestResult> {
        let method = input.method.trim().to_ascii_uppercase();
        let path = input.path.trim().to_string();
        let timestamp = input.timestamp.unwrap_or_else(|| self.adjusted_timestamp());
        let nonce = random_nonce();
        let body_hash = liner_signer::sha256_hex(input.body.as_deref().unwrap_or("").as_bytes());

        let canonical = format!("{method}\n{path}\n{timestamp}\n{nonce}\n{body_hash}");
        let signature = liner_signer::sign_monitor_base64url(canonical.as_bytes())
            .map_err(AppError::Other)?;

        Ok(SignRequestResult {
            signature,
            timestamp,
            nonce,
        })
    }

    pub fn sign_cover(&self, payload: &str) -> AppResult<String> {
        liner_signer::sign_cover_base64url(payload.as_bytes()).map_err(AppError::Other)
    }

    pub fn sign_raw(&self, payload: &[u8]) -> AppResult<String> {
        liner_signer::sign_api_base64url(payload).map_err(AppError::Other)
    }
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn random_nonce() -> String {
    let mut bytes = [0u8; 8];
    if getrandom::getrandom(&mut bytes).is_err() {
        // Fall back to a time-derived nonce rather than panicking: a weak
        // nonce only weakens replay protection, it never forges a signature.
        let now = now_ms() as u64;
        bytes.copy_from_slice(&now.to_le_bytes());
    }
    hex::encode(bytes)
}
