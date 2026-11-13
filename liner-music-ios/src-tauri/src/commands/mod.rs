//! Tauri command surface.
//!
//! Naming rule: every Electron channel `foo:bar-baz` becomes `foo_bar_baz`.
//! The renderer never sees these names — `src/tauri/bridge.ts` is the only
//! place that maps `window.linerElectron` methods onto them.

pub mod app_cmds;
pub mod audio_cmds;
pub mod auth_cmds;
pub mod http_cmds;
pub mod local_cmds;
pub mod lyrics_cmds;
pub mod media_cmds;
pub mod net_cmds;
pub mod now_playing_cmds;
pub mod saved_cmds;
pub mod signer_cmds;
pub mod storage_cmds;
pub mod window_cmds;
