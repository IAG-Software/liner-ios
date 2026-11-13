//! `http_request` — the renderer's escape hatch around WebKit's fetch limits.
//!
//! Two things WKWebView cannot do that this port needs:
//!
//! 1. **Send the desktop User-Agent.** `User-Agent` is a forbidden header name
//!    for `fetch`, and the API's WAF only admits `Electron/` clients
//!    (see `http_client.rs` for the measurements).
//! 2. **Ignore CORS.** The document origin is `tauri://localhost`, which no
//!    server's `Access-Control-Allow-Origin` is written for.
//!
//! So requests to Liner hosts are replayed from Rust. The renderer's shim
//! (`src/tauri/httpProxy.ts`) keeps everything else — media CDNs, cover art —
//! on WebKit, which has the genuine Safari TLS fingerprint.

use base64::Engine;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

/// Bodies larger than this are refused: the caller should stream media through
/// the native engine or a range request instead of buffering it in JS.
const MAX_BODY_BYTES: usize = 32 * 1024 * 1024;

/// Headers that must not be forwarded: they are either computed by reqwest or
/// describe the WebView's own connection rather than the payload.
const STRIPPED_REQUEST_HEADERS: [&str; 14] = [
    "host",
    "content-length",
    "connection",
    "transfer-encoding",
    "accept-encoding",
    "user-agent",
    "origin",
    "referer",
    "cookie",
    "sec-fetch-mode",
    "sec-fetch-site",
    "sec-fetch-dest",
    "sec-fetch-user",
    "priority",
];

/// Hop-by-hop / framing headers the `Response` constructor would reject anyway.
const STRIPPED_RESPONSE_HEADERS: [&str; 6] = [
    "connection",
    "transfer-encoding",
    "content-encoding",
    "content-length",
    "keep-alive",
    "set-cookie",
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestInput {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    /// Base64. Absent for GET/HEAD.
    #[serde(default)]
    pub body: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponseOutput {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    /// Base64.
    pub body: String,
}

#[tauri::command]
pub async fn http_request(
    state: State<'_, AppState>,
    request: HttpRequestInput,
) -> AppResult<HttpResponseOutput> {
    let method = reqwest::Method::from_bytes(request.method.as_bytes())
        .map_err(|_| AppError::Invalid(format!("bad method: {}", request.method)))?;

    let mut builder = state.http.request(method.clone(), &request.url);

    for (name, value) in &request.headers {
        let lower = name.to_ascii_lowercase();
        if STRIPPED_REQUEST_HEADERS.contains(&lower.as_str()) {
            continue;
        }
        builder = builder.header(name, value);
    }

    // Always advertise the client identity the WAF expects.
    builder = builder.header(reqwest::header::USER_AGENT, crate::http_client::DESKTOP_UA);

    if let Some(body) = request.body {
        if !body.is_empty() {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&body)
                .map_err(|e| AppError::Invalid(format!("body base64: {e}")))?;
            builder = builder.body(bytes);
        }
    }

    let response = builder
        .send()
        .await
        .map_err(|e| AppError::Network(format!("{method} {} failed: {e}", request.url)))?;

    let status = response.status();
    let status_text = status.canonical_reason().unwrap_or("").to_string();

    let headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .filter(|(name, _)| !STRIPPED_RESPONSE_HEADERS.contains(&name.as_str()))
        .map(|(name, value)| {
            (
                name.as_str().to_string(),
                value.to_str().unwrap_or_default().to_string(),
            )
        })
        .collect();

    let bytes = response
        .bytes()
        .await
        .map_err(|e| AppError::Network(format!("read body: {e}")))?;

    if bytes.len() > MAX_BODY_BYTES {
        return Err(AppError::Network(format!(
            "response body {} bytes exceeds the {} byte proxy limit",
            bytes.len(),
            MAX_BODY_BYTES
        )));
    }

    Ok(HttpResponseOutput {
        status: status.as_u16(),
        status_text,
        headers,
        body: base64::engine::general_purpose::STANDARD.encode(&bytes),
    })
}
