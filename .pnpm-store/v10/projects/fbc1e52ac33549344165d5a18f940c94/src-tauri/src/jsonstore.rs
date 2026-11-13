//! Crash-safe JSON persistence.
//!
//! The Electron build wrote library indices with `writeFile` + `rename`; iOS
//! can kill the process at any moment (background jetsam), so every write here
//! goes to `<file>.tmp` and is `fsync`ed before the rename. A partially written
//! index therefore never becomes visible to the renderer.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{AppError, AppResult};

pub struct JsonFile {
    path: PathBuf,
    /// Serialises read-modify-write cycles so two concurrent commands cannot
    /// clobber each other's index.
    lock: Mutex<()>,
}

impl JsonFile {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            lock: Mutex::new(()),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read the file, returning `T::default()` when it is absent or corrupt.
    pub fn read<T: DeserializeOwned + Default>(&self) -> AppResult<T> {
        let raw = match fs::read_to_string(&self.path) {
            Ok(raw) => raw,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(T::default()),
            Err(err) => return Err(err.into()),
        };
        match serde_json::from_str(&raw) {
            Ok(value) => Ok(value),
            Err(err) => {
                // Keep the corrupt payload for diagnosis instead of silently
                // destroying the user's library.
                let backup = self.path.with_extension("json.corrupt");
                let _ = fs::copy(&self.path, &backup);
                log::error!("index {} is corrupt ({err}); moved to {backup:?}", self.path.display());
                Ok(T::default())
            }
        }
    }

    pub fn write<T: Serialize>(&self, value: &T) -> AppResult<()> {
        write_atomic(&self.path, &serde_json::to_vec_pretty(value)?)
    }

    /// Read → mutate → write, holding the lock for the whole cycle.
    pub fn mutate<T, F>(&self, f: F) -> AppResult<T>
    where
        T: DeserializeOwned + Default + Serialize,
        F: FnOnce(&mut T) -> AppResult<()>,
    {
        let _guard = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut value: T = self.read()?;
        f(&mut value)?;
        self.write(&value)?;
        Ok(value)
    }

    pub fn lock(&self) -> std::sync::MutexGuard<'_, ()> {
        self.lock.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Write `bytes` to `path` atomically (tmp file + fsync + rename).
pub fn write_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin")
    ));

    {
        let mut file = File::create(&tmp)?;
        file.write_all(bytes)?;
        // `sync_all` maps to F_FULLFSYNC on APFS, which is what makes the
        // rename below survive an abrupt power loss.
        file.sync_all()?;
    }

    match fs::rename(&tmp, path) {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = fs::remove_file(&tmp);
            Err(AppError::Io(err))
        }
    }
}

/// Extension helper shared by the cache and the two libraries.
pub fn extension_for_mime(mime: &str) -> Option<&'static str> {
    let base = mime
        .split(';')
        .next()
        .unwrap_or(mime)
        .trim()
        .to_ascii_lowercase();
    Some(match base.as_str() {
        "audio/mpeg" | "audio/mp3" => "mp3",
        "audio/mp4" | "audio/m4a" | "audio/x-m4a" | "audio/aac" => "m4a",
        "audio/flac" | "audio/x-flac" => "flac",
        "audio/ogg" => "ogg",
        "audio/opus" => "opus",
        "audio/wav" | "audio/x-wav" => "wav",
        "audio/webm" => "webm",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        _ => return None,
    })
}

/// Best-effort audio extension when the server did not tell us the MIME type.
pub fn extension_for_url(url: &str) -> &'static str {
    let lower = url.to_ascii_lowercase();
    let path = lower.split('?').next().unwrap_or(&lower);
    if path.ends_with(".mp3") {
        "mp3"
    } else if path.ends_with(".m4a") || path.ends_with(".mp4") || path.ends_with(".aac") {
        "m4a"
    } else if path.ends_with(".flac") {
        "flac"
    } else if path.ends_with(".ogg") {
        "ogg"
    } else if path.ends_with(".opus") {
        "opus"
    } else if path.ends_with(".wav") {
        "wav"
    } else {
        "m4a"
    }
}
