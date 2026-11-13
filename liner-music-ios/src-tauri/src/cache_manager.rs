//! Audio and album-art cache.
//!
//! Everything here is *disposable*: iOS may purge `Library/Caches` while the
//! app is suspended, so the cache is written in a way that a missing or
//! truncated entry is always recoverable — a download goes to `<key>.part`
//! and is renamed only after the byte count matches what we expect.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;
use serde::Serialize;
use sha2::Digest;
use tokio::io::AsyncWriteExt;

use crate::error::{AppError, AppResult};
use crate::jsonstore::{extension_for_mime, extension_for_url};
use crate::paths::{clear_dir, dir_size, AppPaths};

#[derive(Debug, Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
#[allow(non_snake_case)]
pub struct CacheStats {
    pub cache_size: u64,
    pub http_cache_size: u64,
    pub code_cache_size: u64,
    pub service_worker_size: u64,
    pub indexed_db_size: u64,
    #[serde(rename = "localStorageSize")]
    pub localStorage_size: u64,
    pub blob_storage_size: u64,
    pub cache_path: String,
}

pub struct CacheManager {
    paths: AppPaths,
    http: reqwest::Client,
    /// In-flight downloads, so two callers asking for the same track share one
    /// HTTP request instead of racing each other into the same `.part` file.
    inflight: Mutex<HashMap<String, Arc<tokio::sync::Notify>>>,
}

impl CacheManager {
    pub fn new(paths: AppPaths, http: reqwest::Client) -> Self {
        Self {
            paths,
            http,
            inflight: Mutex::new(HashMap::new()),
        }
    }

    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    pub fn stats(&self) -> CacheStats {
        let cache_path = self.paths.cache.clone();
        CacheStats {
            cache_size: dir_size(&cache_path),
            http_cache_size: dir_size(&self.paths.http_cache_dir()),
            code_cache_size: 0,
            service_worker_size: 0,
            indexed_db_size: 0,
            localStorage_size: 0,
            blob_storage_size: 0,
            cache_path: cache_path.to_string_lossy().to_string(),
        }
    }

    /// Stable, filesystem-safe cache key: `sha256(url)` truncated to 32 hex
    /// chars. The URL (not the track id) is the key because signed media URLs
    /// are what actually get re-requested.
    pub fn key_for(url: &str) -> String {
        let digest = sha2::Sha256::digest(url.as_bytes());
        hex::encode(&digest[..16])
    }

    pub fn audio_path(&self, url: &str, mime: Option<&str>) -> PathBuf {
        let ext = mime
            .and_then(extension_for_mime)
            .unwrap_or_else(|| extension_for_url(url));
        self.paths
            .audio_cache_dir()
            .join(format!("{}.{ext}", Self::key_for(url)))
    }

    /// Returns a local path for `url`, downloading it if necessary.
    ///
    /// `min_bytes` lets the caller wait until enough data exists to start
    /// decoding without stalling (gapless pre-buffer).
    pub async fn ensure_audio(
        &self,
        url: &str,
        mime: Option<&str>,
    ) -> AppResult<PathBuf> {
        let final_path = self.audio_path(url, mime);
        if let Ok(meta) = std::fs::metadata(&final_path) {
            if meta.len() > 0 {
                return Ok(final_path);
            }
        }

        // Coalesce concurrent downloads of the same URL.
        let notify = {
            let mut guard = self.inflight.lock();
            if let Some(existing) = guard.get(url) {
                Some(existing.clone())
            } else {
                let notify = Arc::new(tokio::sync::Notify::new());
                guard.insert(url.to_string(), notify.clone());
                None
            }
        };

        if let Some(notify) = notify {
            notify.notified().await;
            if final_path.exists() {
                return Ok(final_path);
            }
        }

        let result = self.download(url, &final_path).await;

        if let Some(notify) = self.inflight.lock().remove(url) {
            notify.notify_waiters();
        }

        result
    }

    async fn download(&self, url: &str, final_path: &Path) -> AppResult<PathBuf> {
        let response = self
            .http
            .get(url)
            .header("accept", "*/*")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AppError::Network(format!(
                "media download failed: {} for {url}",
                response.status()
            )));
        }

        let part = final_path.with_extension(format!(
            "{}.part",
            final_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bin")
        ));

        let mut file = tokio::fs::File::create(&part).await?;
        let mut written = 0u64;
        let mut stream = response.bytes_stream();

        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| AppError::Network(e.to_string()))?;
            file.write_all(&chunk).await?;
            written += chunk.len() as u64;
        }
        file.flush().await?;
        file.sync_all().await?;
        drop(file);

        if written == 0 {
            let _ = tokio::fs::remove_file(&part).await;
            return Err(AppError::Network("empty media body".into()));
        }

        tokio::fs::rename(&part, final_path).await?;
        Ok(final_path.to_path_buf())
    }

    /// Copy an already-downloaded blob into the cache (used by the saved-track
    /// path so playback never re-downloads).
    pub fn adopt(&self, url: &str, mime: Option<&str>, from: &Path) -> AppResult<PathBuf> {
        let target = self.audio_path(url, mime);
        if target.exists() {
            return Ok(target);
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(from, &target)?;
        Ok(target)
    }

    pub fn cover_path(&self, url: &str) -> PathBuf {
        let ext = extension_for_url(url);
        let ext = if ext == "m4a" { "jpg" } else { ext };
        self.paths
            .cover_cache_dir()
            .join(format!("{}.{ext}", Self::key_for(url)))
    }

    pub async fn ensure_cover(&self, url: &str) -> AppResult<PathBuf> {
        let path = self.cover_path(url);
        if path.exists() {
            return Ok(path);
        }
        let response = self.http.get(url).send().await?;
        if !response.status().is_success() {
            return Err(AppError::Network(format!(
                "cover download failed: {}",
                response.status()
            )));
        }
        let bytes = response.bytes().await?;
        if bytes.is_empty() {
            return Err(AppError::Network("empty cover body".into()));
        }
        crate::jsonstore::write_atomic(&path, &bytes)?;
        Ok(path)
    }

    pub fn clear_audio(&self) -> AppResult<u64> {
        clear_dir(&self.paths.audio_cache_dir())
    }

    pub fn clear_covers(&self) -> AppResult<u64> {
        clear_dir(&self.paths.cover_cache_dir())
    }

    pub fn clear_http(&self) -> AppResult<u64> {
        clear_dir(&self.paths.http_cache_dir())
    }
}
