from pathlib import Path

# ---- Rust: register the commands -------------------------------------------
p = Path(r"D:\ios2\liner-music-ios\src-tauri\src\commands\mod.rs")
s = p.read_text(encoding="utf-8")
s = s.replace("pub mod net_cmds;", "pub mod net_cmds;\npub mod now_playing_cmds;")
p.write_text(s, encoding="utf-8")

p = Path(r"D:\ios2\liner-music-ios\src-tauri\src\lib.rs")
s = p.read_text(encoding="utf-8")
s = s.replace(
    "            commands::lyrics_cmds::lyrics_current,",
    "            commands::lyrics_cmds::lyrics_current,\n"
    "            // ---- native Now Playing (lock screen, Dynamic Island, Siri) ----\n"
    "            commands::now_playing_cmds::now_playing_update,\n"
    "            commands::now_playing_cmds::now_playing_elapsed,\n"
    "            commands::now_playing_cmds::now_playing_clear,",
)
p.write_text(s, encoding="utf-8")

# ---- platform_ios: forward remote commands as events ------------------------
p = Path(r"D:\ios2\liner-music-ios\src-tauri\src\platform_ios.rs")
s = p.read_text(encoding="utf-8")
s = s.replace(
    """pub fn install(app: &tauri::AppHandle) -> AppResult<()> {
    let _ = app;

    unsafe {
        liner_remote_commands_install(remote_callback);
        liner_audio_session_activate();
        liner_idle_timer_disabled(1);
    }

    Ok(())
}""",
    """pub fn install(app: &tauri::AppHandle) -> AppResult<()> {
    unsafe {
        liner_remote_commands_install(remote_callback);
        liner_audio_session_activate();
        liner_idle_timer_disabled(1);
    }

    spawn_remote_forwarder(app.clone());
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
}""",
)
p.write_text(s, encoding="utf-8")

# ---- audio worker: stop stealing the latch ---------------------------------
p = Path(r"D:\ios2\liner-music-ios\src-tauri\src\audio_engine.rs")
s = p.read_text(encoding="utf-8")
s = s.replace(
    """            #[cfg(target_os = "ios")]
            self.poll_remote();

""",
    "",
)
p.write_text(s, encoding="utf-8")

print("rust wiring done")
