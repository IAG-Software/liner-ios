//! Liner Music — Tauri v2 native host.
//!
//! The React frontend that ships inside the iOS bundle is the *same* renderer
//! the Electron build uses. Everything that used to be a Node/Chromium
//! capability (main-process IPC) is re-implemented here as `#[tauri::command]`
//! handlers, and the renderer talks to them through a single adapter object
//! (`window.linerElectron`, installed by `src/tauri/bridge.ts`).
//!
//! Module map (mirrors the brief):
//! * [`audio_engine`]  — decode/output state machine + gapless queue + DSP
//! * [`lyrics_sync`]   — timestamp + syllable parser driven by the audio clock
//! * [`cache_manager`] — sandboxed audio/cover cache under `Library/Caches`
//! * [`commands`]      — the IPC surface the renderer actually calls

pub mod audio;
pub mod audio_engine;
pub mod auth;
pub mod boot_log;
pub mod cache_manager;
pub mod commands;
pub mod error;
pub mod http_client;
pub mod jsonstore;
pub mod local_library;
pub mod lyrics_sync;
pub mod media_protocol;
pub mod net;
pub mod paths;
pub mod saved_library;
pub mod signer;
pub mod state;

#[cfg(target_os = "ios")]
pub mod platform_ios;

use state::AppState;
use tauri::Manager;

/// Mobile entry point.
///
/// On iOS `tauri::mobile_entry_point` emits the exported
/// `#[no_mangle] extern "C" fn start_app()` symbol that the UIKit host
/// (`ios/LinerMusic/main.mm`) calls from `main()`. That symbol is the only
/// contract between the ObjC shell and this crate.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        // Media origin for `<audio>`/`<img>` fallbacks; see media_protocol.rs.
        .register_uri_scheme_protocol("liner-local", media_protocol::handle::<tauri::Wry>)
        .setup(|app| {
            // Diagnostics first: everything below must be observable on a device
            // with no console attached (Documents/liner-boot.log).
            boot_log::init(app.handle());
            log::info!("setup: begin");

            // A panic inside a Tauri command must not take down the WebContent
            // process; record it where the user can retrieve it.
            std::panic::set_hook(Box::new(|info| {
                log::error!("panic: {info}");
            }));

            let handle = app.handle().clone();

            let state = AppState::initialize(&handle).map_err(|err| {
                log::error!("setup: AppState::initialize failed: {err}");
                Box::new(err) as Box<dyn std::error::Error>
            })?;
            log::info!("setup: state ready at {:?}", state.paths.data);

            // The audio engine owns its own OS thread; it only needs an
            // `AppHandle` to emit events to the WebView.
            state.audio.attach_handle(handle.clone());
            app.manage(state);
            log::info!("setup: state managed");

            #[cfg(target_os = "ios")]
            platform_ios::install(app.handle()).map_err(|err| {
                log::error!("setup: platform install failed: {err}");
                Box::new(err) as Box<dyn std::error::Error>
            })?;

            log::info!("setup: complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // ---- app ----
            commands::app_cmds::app_get_version,
            commands::app_cmds::app_set_icon,
            commands::app_cmds::app_open_external,
            commands::app_cmds::app_platform,
            commands::app_cmds::boot_log,
            commands::app_cmds::boot_log_path,
            // ---- window (no-ops on iOS, kept for renderer parity) ----
            commands::window_cmds::window_minimize,
            commands::window_cmds::window_toggle_maximize,
            commands::window_cmds::window_toggle_fullscreen,
            commands::window_cmds::window_close,
            commands::window_cmds::window_is_maximized,
            commands::window_cmds::window_is_fullscreen,
            commands::window_cmds::window_get_remember_bounds,
            commands::window_cmds::window_set_remember_bounds,
            commands::window_cmds::window_presence,
            commands::window_cmds::window_drag_start,
            commands::window_cmds::window_drag_move,
            commands::window_cmds::window_drag_end,
            // ---- signing ----
            commands::signer_cmds::signer_sign_request,
            commands::signer_cmds::signer_sign_monitor_request,
            commands::signer_cmds::signer_sync_time,
            commands::signer_cmds::signer_get_time_offset,
            commands::signer_cmds::signer_sign_cover_url,
            commands::signer_cmds::signer_sign_raw_payload,
            commands::signer_cmds::edge_set_key,
            // ---- auth ----
            commands::auth_cmds::auth_get_session,
            commands::auth_cmds::auth_set_session,
            commands::auth_cmds::auth_refresh,
            commands::auth_cmds::youtube_auth_get_session,
            commands::auth_cmds::youtube_auth_set_session,
            // ---- storage / cache ----
            commands::storage_cmds::storage_get_cache_stats,
            commands::storage_cmds::storage_clear_covers_cache,
            commands::storage_cmds::storage_clear_audio_cache,
            commands::storage_cmds::storage_clear_http_cache,
            commands::storage_cmds::storage_open_cache_folder,
            commands::storage_cmds::shell_open_downloads,
            commands::storage_cmds::shell_open_export_folder,
            // ---- local library ----
            commands::local_cmds::local_tracks_list,
            commands::local_cmds::local_tracks_import,
            commands::local_cmds::local_tracks_pick,
            commands::local_cmds::local_tracks_remove,
            commands::local_cmds::local_tracks_remove_all,
            commands::local_cmds::local_tracks_usage,
            // ---- saved library ----
            commands::saved_cmds::saved_tracks_list,
            commands::saved_cmds::saved_tracks_save,
            commands::saved_cmds::saved_tracks_remove,
            commands::saved_cmds::saved_tracks_remove_all,
            commands::saved_cmds::saved_tracks_usage,
            commands::saved_cmds::saved_tracks_usage_of,
            commands::saved_cmds::saved_tracks_open_folder,
            commands::saved_cmds::saved_tracks_get_lyrics,
            commands::saved_cmds::saved_tracks_set_lyrics,
            // ---- network diagnostics ----
            commands::net_cmds::net_diagnose,
            commands::net_cmds::net_probe,
            // ---- native HTTP proxy (UA + CORS escape hatch) ----
            commands::http_cmds::http_request,
            // ---- native audio engine ----
            commands::audio_cmds::audio_load,
            commands::audio_cmds::audio_play,
            commands::audio_cmds::audio_pause,
            commands::audio_cmds::audio_seek,
            commands::audio_cmds::audio_stop,
            commands::audio_cmds::audio_set_volume,
            commands::audio_cmds::audio_set_rate,
            commands::audio_cmds::audio_enqueue,
            commands::audio_cmds::audio_set_queue,
            commands::audio_cmds::audio_clear_queue,
            commands::audio_cmds::audio_set_equalizer,
            commands::audio_cmds::audio_state,
            commands::audio_cmds::audio_playback_url,
            // ---- lyrics ----
            commands::lyrics_cmds::lyrics_load,
            commands::lyrics_cmds::lyrics_clear,
            commands::lyrics_cmds::lyrics_current,
            // ---- native Now Playing (lock screen, Dynamic Island, Siri) ----
            commands::now_playing_cmds::now_playing_update,
            commands::now_playing_cmds::now_playing_elapsed,
            commands::now_playing_cmds::now_playing_clear,
            commands::media_cmds::show_route_picker,
        ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running Liner Music");
}
