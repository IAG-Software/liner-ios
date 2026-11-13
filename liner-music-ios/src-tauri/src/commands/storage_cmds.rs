//! `storage:*` and `shell:*` channels.

use tauri::State;

use crate::cache_manager::CacheStats;
use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub fn storage_get_cache_stats(state: State<'_, AppState>) -> CacheStats {
    state.cache.stats()
}

#[tauri::command]
pub fn storage_clear_covers_cache(state: State<'_, AppState>) -> AppResult<bool> {
    state.cache.clear_covers()?;
    Ok(true)
}

/// `clear_entire_db` is accepted for parity with the desktop menu; the cache is
/// already a flat directory so both variants do the same thing.
#[tauri::command]
pub fn storage_clear_audio_cache(
    state: State<'_, AppState>,
    clear_entire_db: Option<bool>,
) -> AppResult<bool> {
    let _ = clear_entire_db;
    state.cache.clear_audio()?;
    Ok(true)
}

#[tauri::command]
pub fn storage_clear_http_cache(state: State<'_, AppState>) -> AppResult<bool> {
    state.cache.clear_http()?;
    Ok(true)
}

/// iOS has no file manager to open. The renderer expects `false` so it can
/// hide the "Open folder" affordance.
#[tauri::command]
pub fn storage_open_cache_folder() -> bool {
    false
}

#[tauri::command]
pub fn shell_open_downloads(custom_path: Option<String>) -> bool {
    let _ = custom_path;
    false
}

#[tauri::command]
pub fn shell_open_export_folder(custom_path: Option<String>) -> bool {
    let _ = custom_path;
    false
}
