//! `window:*` channels.
//!
//! iOS has a single, OS-managed window: no minimise, no maximise, no custom
//! title bar, no drag-to-move. The renderer still calls these on boot (it runs
//! the same code path as desktop), so every command resolves successfully with
//! the "nothing to do" value instead of throwing.
//!
//! These are deliberately pure stubs rather than `#[cfg(desktop)]` branches:
//! this crate targets iOS, and the desktop window integration in the Tauri v2
//! builder is not part of the port.

use crate::error::AppResult;

#[tauri::command]
pub fn window_minimize() -> AppResult<()> {
    Ok(())
}

#[tauri::command]
pub fn window_toggle_maximize() -> AppResult<()> {
    Ok(())
}

#[tauri::command]
pub fn window_toggle_fullscreen() -> AppResult<()> {
    Ok(())
}

#[tauri::command]
pub fn window_close() -> AppResult<()> {
    Ok(())
}

#[tauri::command]
pub fn window_is_maximized() -> bool {
    false
}

#[tauri::command]
pub fn window_is_fullscreen() -> bool {
    false
}

#[tauri::command]
pub fn window_get_remember_bounds() -> bool {
    true
}

#[tauri::command]
pub fn window_set_remember_bounds(_enabled: bool) -> bool {
    true
}

#[tauri::command]
pub fn window_presence() {}

#[tauri::command]
pub fn window_drag_start() {}

#[tauri::command]
pub fn window_drag_move(_delta_x: f64, _delta_y: f64) {}

#[tauri::command]
pub fn window_drag_end() {}
