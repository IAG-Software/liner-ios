//! `signer:*` and `edge:*` channels.

use std::sync::Mutex;

use serde::Serialize;
use serde_json::Value;
use tauri::State;

use crate::error::AppResult;
use crate::signer::SignRequestInput;
use crate::state::AppState;

/// CDN edge key set by the renderer after login. Held natively so the renderer
/// does not have to thread it through every request builder.
static EDGE_KEY: Mutex<Option<String>> = Mutex::new(None);

#[derive(Debug, Serialize)]
pub struct SignResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl SignResult {
    fn ok(signature: String, timestamp: u64, nonce: String) -> Self {
        Self {
            signature: Some(signature),
            timestamp: Some(timestamp),
            nonce: Some(nonce),
            error: None,
        }
    }

    fn failed(message: String) -> Self {
        Self {
            signature: None,
            timestamp: None,
            nonce: None,
            error: Some(message),
        }
    }
}

#[tauri::command]
pub fn signer_sign_request(state: State<'_, AppState>, input: SignRequestInput) -> SignResult {
    match state.signer.sign_request(&input) {
        Ok(result) => SignResult::ok(result.signature, result.timestamp, result.nonce),
        Err(err) => SignResult::failed(err.to_string()),
    }
}

#[tauri::command]
pub fn signer_sign_monitor_request(
    state: State<'_, AppState>,
    input: SignRequestInput,
) -> SignResult {
    match state.signer.sign_monitor_request(&input) {
        Ok(result) => SignResult::ok(result.signature, result.timestamp, result.nonce),
        Err(err) => SignResult::failed(err.to_string()),
    }
}

/// Accepts either a `Date` header string or an epoch-millis number.
#[tauri::command]
pub fn signer_sync_time(state: State<'_, AppState>, date_header_or_ms: Value) -> i64 {
    let server_ms = match &date_header_or_ms {
        Value::Number(number) => number.as_i64().unwrap_or(0),
        Value::String(text) => chrono::DateTime::parse_from_rfc2822(text)
            .map(|dt| dt.timestamp_millis())
            .or_else(|_| {
                chrono::DateTime::parse_from_rfc3339(text).map(|dt| dt.timestamp_millis())
            })
            .unwrap_or(0),
        _ => 0,
    };

    if server_ms > 0 {
        state.signer.set_server_time_ms(server_ms)
    } else {
        state.signer.offset_ms()
    }
}

#[tauri::command]
pub fn signer_get_time_offset(state: State<'_, AppState>) -> i64 {
    state.signer.offset_ms()
}

#[tauri::command]
pub fn signer_sign_cover_url(state: State<'_, AppState>, payload: String) -> AppResult<String> {
    state.signer.sign_cover(&payload)
}

#[tauri::command]
pub fn signer_sign_raw_payload(
    state: State<'_, AppState>,
    payload: Vec<u8>,
) -> AppResult<String> {
    state.signer.sign_raw(&payload)
}

#[tauri::command]
pub fn edge_set_key(key: Option<String>) {
    *EDGE_KEY.lock().unwrap_or_else(|e| e.into_inner()) = key;
}

pub fn edge_key() -> Option<String> {
    EDGE_KEY
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}
