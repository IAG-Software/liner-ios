//! `auth:*` and `youtube-auth:*` channels.

use serde::Deserialize;
use serde_json::Value;
use tauri::{Emitter, State};

use crate::auth::AuthRefreshResult;
use crate::error::AppResult;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshInput {
    pub api_base_url: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
}

#[tauri::command]
pub fn auth_get_session(state: State<'_, AppState>) -> Option<Value> {
    state.auth.liner.get_value()
}

#[tauri::command]
pub fn auth_set_session(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    next: Option<Value>,
) -> AppResult<Option<Value>> {
    let value = state.auth.liner.set(next)?;
    // Broadcast so every `onChange` subscriber (auth store, player, overlays)
    // sees a rotation exactly once.
    let _ = app.emit("auth:session-changed", &value);
    Ok(value)
}

#[tauri::command]
pub async fn auth_refresh(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input: RefreshInput,
) -> AppResult<AuthRefreshResult> {
    let result = state
        .auth
        .liner
        .refresh(
            &state.http,
            &input.api_base_url,
            input.refresh_token,
            &state.signer,
        )
        .await;

    let _ = app.emit("auth:session-changed", &result.session);
    Ok(result)
}

#[tauri::command]
pub fn youtube_auth_get_session(state: State<'_, AppState>) -> Option<Value> {
    state.auth.youtube.get_value()
}

#[tauri::command]
pub fn youtube_auth_set_session(
    state: State<'_, AppState>,
    next: Option<Value>,
) -> AppResult<Option<Value>> {
    state.auth.youtube.set(next)
}
