//! `lyrics:*` channels.

use serde_json::Value;
use tauri::State;

use crate::error::AppResult;
use crate::lyrics_sync::ProgressEvent;
use crate::state::AppState;

/// Compile the renderer's `SyncedLine[]` into the native timeline.
/// Returns the number of lines that are now schedulable.
#[tauri::command]
pub fn lyrics_load(state: State<'_, AppState>, payload: Value) -> AppResult<usize> {
    state.lyrics.load(payload)
}

#[tauri::command]
pub fn lyrics_clear(state: State<'_, AppState>) {
    state.lyrics.clear();
}

/// Snapshot of the active line/word for the audio position, for renderers that
/// prefer polling over events (and for re-anchoring after a WebView hitch).
#[tauri::command]
pub fn lyrics_current(state: State<'_, AppState>, position_ms: Option<u64>) -> ProgressEvent {
    let position = position_ms.unwrap_or_else(|| state.audio.snapshot().position_ms);
    state.lyrics.current(position)
}
