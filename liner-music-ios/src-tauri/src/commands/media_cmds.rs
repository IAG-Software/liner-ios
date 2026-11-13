//! `media:*` — outputs and playback-surface controls.

use tauri::State;

use crate::state::AppState;

/// Show the system output-route sheet (AirPlay, Bluetooth, speakers).
#[tauri::command]
pub fn show_route_picker(_state: State<'_, AppState>) -> bool {
    #[cfg(target_os = "ios")]
    {
        return crate::platform_ios::route_picker::show();
    }
    #[cfg(not(target_os = "ios"))]
    {
        false
    }
}
