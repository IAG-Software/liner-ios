//! `now-playing:*` — publish transport state to the iOS Now Playing centre.
//!
//! Playback happens in an `<audio>` element inside WKWebView. WebKit does *not*
//! register a WKWebView as the system Now Playing app, so without this bridge
//! iOS has nothing to show on the lock screen, in the Dynamic Island, or for
//! Siri to control.
//!
//! This publishes the same state natively, and `platform_ios` forwards
//! `MPRemoteCommandCenter` events back to the renderer as `media:remote`, which
//! is what makes "Hey Siri, pause" actually pause the web player.

use tauri::State;

use crate::state::AppState;

/// Push the current track + transport state to `MPNowPlayingInfoCenter`.
#[tauri::command]
pub fn now_playing_update(
    _state: State<'_, AppState>,
    title: String,
    artist: String,
    album: Option<String>,
    duration_ms: f64,
    position_ms: f64,
    playing: bool,
    artwork_path: Option<String>,
) {
    #[cfg(target_os = "ios")]
    crate::platform_ios::now_playing::update(
        &title,
        &artist,
        album.as_deref(),
        duration_ms.max(0.0) as u64,
        position_ms.max(0.0) / 1000.0,
        if playing { 1.0 } else { 0.0 },
        artwork_path.as_deref(),
    );
    #[cfg(not(target_os = "ios"))]
    {
        let _ = (title, artist, album, duration_ms, position_ms, playing, artwork_path);
    }
}

/// Cheap position-only update for the 1 Hz transport tick.
#[tauri::command]
pub fn now_playing_elapsed(
    _state: State<'_, AppState>,
    position_ms: f64,
    rate: f64,
) {
    #[cfg(target_os = "ios")]
    crate::platform_ios::now_playing::update_elapsed(position_ms.max(0.0) / 1000.0, rate);
    #[cfg(not(target_os = "ios"))]
    {
        let _ = (position_ms, rate);
    }
}

#[tauri::command]
pub fn now_playing_clear(_state: State<'_, AppState>) {
    #[cfg(target_os = "ios")]
    crate::platform_ios::now_playing::clear();
}
