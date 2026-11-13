//! `local-tracks:*` channels.

use std::path::PathBuf;

use tauri::State;

use crate::error::AppResult;
use crate::local_library::{LocalImportResult, LocalLibraryUsage, LocalTrackRecord};
use crate::state::AppState;

#[tauri::command]
pub fn local_tracks_list(state: State<'_, AppState>) -> Vec<LocalTrackRecord> {
    state.local.list()
}

#[tauri::command]
pub fn local_tracks_import(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> LocalImportResult {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    state.local.import_paths(&paths)
}

/// Presents `UIDocumentPickerViewController`, then imports whatever the user
/// selected. Returns `None` when the sheet is dismissed without a selection —
/// the renderer treats that as "no-op", not as an error.
#[tauri::command]
pub async fn local_tracks_pick(state: State<'_, AppState>) -> AppResult<Option<LocalImportResult>> {
    #[cfg(target_os = "ios")]
    {
        let paths = crate::platform_ios::picker::pick_audio_files().await?;
        if paths.is_empty() {
            return Ok(None);
        }
        let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        return Ok(Some(state.local.import_paths(&paths)));
    }
    #[cfg(not(target_os = "ios"))]
    {
        let _ = state;
        Ok(None)
    }
}

#[tauri::command]
pub fn local_tracks_remove(state: State<'_, AppState>, ids: Vec<String>) -> AppResult<()> {
    state.local.remove(&ids)
}

#[tauri::command]
pub fn local_tracks_remove_all(state: State<'_, AppState>) -> AppResult<usize> {
    state.local.remove_all()
}

#[tauri::command]
pub fn local_tracks_usage(state: State<'_, AppState>) -> LocalLibraryUsage {
    state.local.usage()
}
