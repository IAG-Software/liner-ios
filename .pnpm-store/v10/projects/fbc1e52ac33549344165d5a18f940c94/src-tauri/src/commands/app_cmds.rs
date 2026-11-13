//! `app:*` channels.

use tauri::AppHandle;
use tauri::State;

use crate::error::AppResult;
use crate::state::AppState;

#[tauri::command]
pub fn app_get_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

/// Electron swapped the window/dock icon at runtime. iOS icons are baked into
/// the bundle by the asset catalogue, so this is intentionally a no-op rather
/// than an error — the renderer calls it on every theme change.
#[tauri::command]
pub fn app_set_icon(data_url: String, is_default: Option<bool>) -> AppResult<()> {
    let _ = (data_url, is_default);
    Ok(())
}

#[tauri::command]
pub fn app_open_external(url: String) -> AppResult<bool> {
    #[cfg(target_os = "ios")]
    {
        return crate::platform_ios::open_url(&url);
    }
    #[cfg(not(target_os = "ios"))]
    {
        let _ = url;
        Ok(false)
    }
}

/// The renderer gates desktop-only chrome on `platform === "darwin" | "linux"`.
/// Reporting `ios` is what switches it into the touch layout.
#[tauri::command]
pub fn app_platform(state: State<'_, AppState>) -> String {
    let _ = &state;
    if cfg!(target_os = "ios") {
        "ios".into()
    } else if cfg!(target_os = "macos") {
        "darwin".into()
    } else if cfg!(target_os = "windows") {
        "win32".into()
    } else {
        "linux".into()
    }
}

/// Append a line to `Documents/liner-boot.log`.
///
/// The WebView has no console on a sideloaded device; this is how renderer-side
/// failures (module load errors, panics during boot) become readable in
/// Files.app.
#[tauri::command]
pub fn boot_log(message: String) {
    crate::boot_log::append_line(&message);
}

/// Absolute path of the boot log, so the UI can tell the user where to look.
#[tauri::command]
pub fn boot_log_path() -> Option<String> {
    crate::boot_log::path().map(|p| p.to_string_lossy().to_string())
}
