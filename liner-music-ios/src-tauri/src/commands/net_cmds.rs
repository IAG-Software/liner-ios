//! `net:*` channels.

use tauri::State;

use crate::error::AppResult;
use crate::net::{MainNetResult, ProbeResult};
use crate::state::AppState;

#[tauri::command]
pub async fn net_diagnose(
    state: State<'_, AppState>,
    hosts: Vec<String>,
) -> AppResult<MainNetResult> {
    crate::net::diagnose(&state.http, hosts).await
}

#[tauri::command]
pub async fn net_probe(state: State<'_, AppState>) -> AppResult<ProbeResult> {
    Ok(crate::net::probe(&state.http).await)
}
