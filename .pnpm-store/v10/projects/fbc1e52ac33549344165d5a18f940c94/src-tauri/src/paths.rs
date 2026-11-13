//! Sandbox directory layout.
//!
//! On iOS every writable location is inside the app container:
//!
//! ```text
//! <container>/Library/Caches/<bundle-id>/      -> AppCache    (purgeable)
//! <container>/Library/Application Support/      -> AppData     (persistent)
//! ```
//!
//! Anything in `Caches` may be evicted by iOS when the device is low on
//! storage, so only re-downloadable data (audio cache, album art cache) lives
//! there. User data (saved tracks, local imports, auth sessions) lives in
//! `Application Support`.

use std::path::{Path, PathBuf};

use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

use crate::error::AppResult;

#[derive(Debug, Clone)]
pub struct AppPaths {
    /// `Library/Caches/<bundle-id>` on iOS, `%LOCALAPPDATA%` on desktop.
    pub cache: PathBuf,
    /// `Application Support` on iOS.
    pub data: PathBuf,
}

impl AppPaths {
    pub fn resolve(app: &AppHandle) -> AppResult<Self> {
        let resolver = app.path();

        let cache = resolver
            .resolve("", BaseDirectory::AppCache)
            .unwrap_or_else(|_| std::env::temp_dir().join("liner-cache"));
        let data = resolver
            .resolve("", BaseDirectory::AppData)
            .unwrap_or_else(|_| std::env::temp_dir().join("liner-data"));

        let paths = Self { cache, data };

        // Eagerly create the layout: every later `write` can then assume the
        // parent exists, which removes a whole class of ENOENT races.
        for dir in [
            paths.audio_cache_dir(),
            paths.cover_cache_dir(),
            paths.data.join("auth"),
            paths.data.join("youtube-auth"),
            paths.local_dir(),
            paths.local_audio_dir(),
            paths.saved_audio_dir(),
            paths.saved_cover_dir(),
            paths.saved_lyrics_dir(),
        ] {
            std::fs::create_dir_all(&dir)?;
        }

        Ok(paths)
    }

    pub fn audio_cache_dir(&self) -> PathBuf {
        self.cache.join("audio")
    }

    pub fn cover_cache_dir(&self) -> PathBuf {
        self.cache.join("covers")
    }

    pub fn http_cache_dir(&self) -> PathBuf {
        self.cache.join("http")
    }

    pub fn session_file(&self) -> PathBuf {
        self.data.join("auth").join("session.enc")
    }

    pub fn session_key_file(&self) -> PathBuf {
        self.data.join("auth").join("device.key")
    }

    pub fn youtube_session_file(&self) -> PathBuf {
        self.data.join("youtube-auth").join("session.enc")
    }

    pub fn local_dir(&self) -> PathBuf {
        self.data.join("local")
    }

    pub fn local_audio_dir(&self) -> PathBuf {
        self.local_dir().join("audio")
    }

    pub fn local_index_file(&self) -> PathBuf {
        self.local_dir().join("library.json")
    }

    pub fn saved_dir(&self) -> PathBuf {
        self.data.join("saved")
    }

    pub fn saved_audio_dir(&self) -> PathBuf {
        self.saved_dir().join("audio")
    }

    pub fn saved_cover_dir(&self) -> PathBuf {
        self.saved_dir().join("covers")
    }

    pub fn saved_lyrics_dir(&self) -> PathBuf {
        self.saved_dir().join("lyrics")
    }

    pub fn saved_index_file(&self) -> PathBuf {
        self.saved_dir().join("library.json")
    }
}

/// Recursively sum the size of every regular file below `dir`.
pub fn dir_size(dir: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                stack.push(entry.path());
            } else {
                total += meta.len();
            }
        }
    }
    total
}

/// Remove every entry inside `dir`, keeping `dir` itself.
pub fn clear_dir(dir: &Path) -> AppResult<u64> {
    let mut removed = 0u64;
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(0);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            removed += dir_size(&path);
            let _ = std::fs::remove_dir_all(&path);
        } else {
            removed += meta.len();
            let _ = std::fs::remove_file(&path);
        }
    }
    Ok(removed)
}
