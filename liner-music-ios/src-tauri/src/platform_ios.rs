//! iOS platform integration.
//!
//! Apple's media stack is reached through a thin Objective-C shim
//! (`ios/LinerMusic/LinerPlatform.m`) rather than through Rust `objc2`
//! bindings. Reasons, recorded here because it is a deliberate deviation:
//!
//! 1. `MPRemoteCommandCenter` handlers are block-capturing Objective-C objects;
//!    owning their lifetime from Rust means hand-rolling `retain`/`release` on
//!    objects that outlive the call, which is exactly the class of bug that is
//!    impossible to test off-device.
//! 2. No Swift toolchain is involved either way (Tauri's own iOS plugin path
//!    would need `swiftc`); plain `.m` compiles with the same `clang` that
//!    links the Rust archive.
//! 3. The C ABI below is 9 functions. That is a surface small enough to audit
//!    in one screen, and it keeps the audio thread free of ObjC message sends.
//!
//! Every function is `unsafe extern "C"` and every caller is on the audio
//! worker thread except `open_url`/`picker`, which are on the main thread by
//! construction (Tauri commands run there).

use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::sync::atomic::{AtomicU8, Ordering};

use parking_lot::Mutex;

use crate::error::{AppError, AppResult};

// Frameworks the ObjC shim references. Rust only passes `-framework` flags for
// crates/link attributes it knows about, so they must be declared here or the
// `cdylib`/`bin` link fails with undefined ObjC class symbols.
#[link(name = "AVFoundation", kind = "framework")]
#[link(name = "MediaPlayer", kind = "framework")]
#[link(name = "UniformTypeIdentifiers", kind = "framework")]
#[link(name = "UIKit", kind = "framework")]
#[link(name = "WebKit", kind = "framework")]
#[link(name = "AVKit", kind = "framework")]
extern "C" {}

extern "C" {
    fn liner_audio_session_activate() -> c_int;
    fn liner_audio_session_deactivate();

    fn liner_now_playing_update(
        title: *const c_char,
        artist: *const c_char,
        album: *const c_char,
        duration_s: f64,
        elapsed_s: f64,
        rate: f64,
        artwork_path: *const c_char,
    );
    fn liner_now_playing_elapsed(elapsed_s: f64, rate: f64);
    fn liner_now_playing_rate(rate: f64);
    fn liner_now_playing_playing(playing: c_int);
    fn liner_now_playing_clear();

    fn liner_remote_commands_install(callback: extern "C" fn(c_int));
    fn liner_take_remote_seek() -> f64;
    fn liner_pick_audio_files(callback: extern "C" fn(*const c_char));
    fn liner_show_route_picker() -> c_int;
    fn liner_url_install(callback: extern "C" fn(*const c_char));
    fn liner_open_url(url: *const c_char) -> c_int;
    fn liner_idle_timer_disabled(disabled: c_int);
}

/// Remote command identifiers shared with `LinerPlatform.m`.
const REMOTE_PLAY: c_int = 0;
const REMOTE_PAUSE: c_int = 1;
const REMOTE_TOGGLE: c_int = 2;
const REMOTE_NEXT: c_int = 3;
const REMOTE_PREVIOUS: c_int = 4;
const REMOTE_SKIP_FORWARD: c_int = 5;
const REMOTE_SKIP_BACKWARD: c_int = 6;

/// Last remote command, published here so the command layer can react without
/// the ObjC shim knowing anything about Tauri.
static LAST_REMOTE: AtomicU8 = AtomicU8::new(255);
static PICK_SLOT: Mutex<Option<tokio::sync::oneshot::Sender<Vec<String>>>> = Mutex::new(None);

pub fn install(app: &tauri::AppHandle) -> AppResult<()> {
    unsafe {
        liner_remote_commands_install(remote_callback);
        liner_audio_session_activate();
        liner_idle_timer_disabled(1);
    }

    spawn_remote_forwarder(app.clone());
    install_deep_links(app.clone());
    Ok(())
}

/// Bridge `MPRemoteCommandCenter` to the WebView.
///
/// The latch is drained here — not in the audio worker — because the *web*
/// player is the one that owns playback in this build. Emitting an event keeps
/// exactly one consumer of the latch, which is what previously made remote
/// presses land on an idle native engine.
fn spawn_remote_forwarder(app: tauri::AppHandle) {
    use tauri::Emitter;

    std::thread::Builder::new()
        .name("liner-remote".into())
        .spawn(move || {
            loop {
                while let Some(code) = take_remote_command() {
                    let command = match code {
                        0 => "play",
                        1 => "pause",
                        2 => "toggle",
                        3 => "next",
                        4 => "previous",
                        5 => "seek-forward",
                        6 => "seek-backward",
                        _ => continue,
                    };
                    if let Err(err) = app.emit("media:remote", command) {
                        log::warn!("remote forward failed: {err}");
                    }
                }

                if let Some(seconds) = take_remote_seek() {
                    let _ = app.emit("media:remote-seek", seconds);
                }

                std::thread::sleep(std::time::Duration::from_millis(80));
            }
        })
        .ok();
}

extern "C" fn remote_callback(command: c_int) {
    // The bridging function only records the intent; the audio worker polls it
    // on its next tick, which keeps ObjC off the audio thread.
    let value = match command {
        REMOTE_PLAY => 0u8,
        REMOTE_PAUSE => 1,
        REMOTE_TOGGLE => 2,
        REMOTE_NEXT => 3,
        REMOTE_PREVIOUS => 4,
        REMOTE_SKIP_FORWARD => 5,
        REMOTE_SKIP_BACKWARD => 6,
        _ => return,
    };
    LAST_REMOTE.store(value, Ordering::Release);
}

/// Take the pending remote command, if any.
pub fn take_remote_command() -> Option<u8> {
    let value = LAST_REMOTE.swap(255, Ordering::AcqRel);
    if value == 255 {
        None
    } else {
        Some(value)
    }
}

/// Lock-screen scrubbing carries a position, so it travels on its own latch
/// instead of the integer command channel. Returns seconds.
pub fn take_remote_seek() -> Option<f64> {
    let seconds = unsafe { liner_take_remote_seek() };
    if seconds < 0.0 {
        None
    } else {
        Some(seconds)
    }
}

pub mod audio_session {
    use super::*;

    /// Activate `AVAudioSessionCategoryPlayback`.
    ///
    /// Must be called before the `AudioUnit` is created: activating afterwards
    /// is legal but the first render callback can be dropped while the route
    /// changes.
    pub fn activate() -> AppResult<()> {
        let status = unsafe { liner_audio_session_activate() };
        if status == 0 {
            Ok(())
        } else {
            Err(AppError::Audio(format!(
                "AVAudioSession activation failed (code {status})"
            )))
        }
    }

    pub fn deactivate() {
        unsafe { liner_audio_session_deactivate() }
    }
}

pub mod now_playing {
    //! Lock-screen / Control-Centre metadata.

    use super::*;

    fn c_string(value: &str) -> Option<CString> {
        // Embedded NULs would truncate the string; strip them rather than fail
        // playback over a stray tag character.
        CString::new(value.replace('\0', "")).ok()
    }

    pub fn update(
        title: &str,
        artist: &str,
        album: Option<&str>,
        duration_ms: u64,
        elapsed_s: f64,
        rate: f64,
        artwork_path: Option<&str>,
    ) {
        let title = c_string(title);
        let artist = c_string(artist);
        let album = album.and_then(c_string);
        let artwork = artwork_path.and_then(c_string);

        unsafe {
            liner_now_playing_update(
                title.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                artist.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                album.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                duration_ms as f64 / 1000.0,
                elapsed_s,
                rate,
                artwork.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
            );
        }
    }

    pub fn update_elapsed(elapsed_s: f64, rate: f64) {
        unsafe { liner_now_playing_elapsed(elapsed_s, rate) }
    }

    pub fn set_rate(rate: f64) {
        unsafe { liner_now_playing_rate(rate) }
    }

    pub fn set_playing(playing: bool) {
        unsafe { liner_now_playing_playing(if playing { 1 } else { 0 }) }
    }

    pub fn clear() {
        unsafe { liner_now_playing_clear() }
    }
}

pub mod picker {
    //! `UIDocumentPickerViewController` bridge.
    //!
    //! The picker copies the chosen files into our container before calling
    //! back, so the returned paths are always readable — no security-scoped
    //! bookmarks to manage.

    use super::*;
    use std::time::Duration;

    pub async fn pick_audio_files() -> AppResult<Vec<String>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let mut slot = PICK_SLOT.lock();
            *slot = Some(tx);
        }

        unsafe { liner_pick_audio_files(pick_callback) }

        match tokio::time::timeout(Duration::from_secs(300), rx).await {
            Ok(Ok(paths)) => Ok(paths),
            Ok(Err(_)) => Err(AppError::Other("picker cancelled".into())),
            Err(_) => Err(AppError::Other("picker timed out".into())),
        }
    }

    extern "C" fn pick_callback(json_paths: *const c_char) {
        let paths = if json_paths.is_null() {
            Vec::new()
        } else {
            // SAFETY: the shim passes a NUL-terminated string it owns for the
            // duration of this call.
            let raw = unsafe { std::ffi::CStr::from_ptr(json_paths) }
                .to_string_lossy()
                .to_string();
            serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default()
        };

        if let Some(tx) = PICK_SLOT.lock().take() {
            let _ = tx.send(paths);
        }
    }
}

/// Deep links: `liner://…` → `deep-link` event for the renderer.
pub fn install_deep_links(app: tauri::AppHandle) {
    use tauri::Emitter;

    extern "C" fn on_url(url: *const c_char) {
        if url.is_null() {
            return;
        }
        let Ok(value) = unsafe { std::ffi::CStr::from_ptr(url) }.to_str() else {
            return;
        };
        if let Some(handle) = crate::state::current_handle() {
            let _ = handle.emit("deep-link", value.to_string());
        }
    }

    let _ = &app;
    unsafe { liner_url_install(on_url) };
}

pub mod route_picker {
    //! System AirPlay / Bluetooth route sheet.

    use super::*;

    pub fn show() -> bool {
        unsafe { liner_show_route_picker() == 1 }
    }
}

/// Open a URL in the system browser / handler app.
pub fn open_url(url: &str) -> AppResult<bool> {
    let Some(c_url) = CString::new(url.replace('\0', "")).ok() else {
        return Err(AppError::Invalid("url contains NUL".into()));
    };
    let status = unsafe { liner_open_url(c_url.as_ptr()) };
    Ok(status == 1)
}

/// Keep the screen awake while a track plays (optional UX nicety).
pub fn set_idle_timer_disabled(disabled: bool) {
    unsafe { liner_idle_timer_disabled(if disabled { 1 } else { 0 }) }
}
