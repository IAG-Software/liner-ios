#![allow(clippy::too_many_arguments)]

//! User-imported audio files.
//!
//! On iOS a "file" only becomes readable after the user grants access through
//! `UIDocumentPickerViewController`. The picker copies the chosen item into our
//! container (see `ios/LinerMusic/LinerPlatform.m`), and this module takes it
//! from there: probe tags, transcode nothing, keep the original bytes.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::jsonstore::{extension_for_mime, JsonFile};
use crate::paths::{dir_size, AppPaths};

pub const LOCAL_AUDIO_EXTENSIONS: [&str; 8] =
    ["mp3", "flac", "m4a", "aac", "ogg", "opus", "wav", "webm"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalTrackRecord {
    pub id: String,
    pub title: String,
    pub artist: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    pub duration_ms: u64,
    pub file_name: String,
    pub audio_file: String,
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_file: Option<String>,
    pub byte_size: u64,
    pub added_at: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalLibraryUsage {
    pub count: usize,
    pub bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalImportResult {
    pub added: Vec<LocalTrackRecord>,
    pub duplicates: usize,
    pub failed: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct LocalIndex {
    tracks: Vec<LocalTrackRecord>,
}

pub struct LocalLibrary {
    paths: AppPaths,
    index: JsonFile,
}

impl LocalLibrary {
    pub fn new(paths: &AppPaths) -> Arc<Self> {
        Arc::new(Self {
            paths: paths.clone(),
            index: JsonFile::new(paths.local_index_file()),
        })
    }

    fn records(&self) -> Vec<LocalTrackRecord> {
        self.index.read::<LocalIndex>().unwrap_or_default().tracks
    }

    pub fn list(&self) -> Vec<LocalTrackRecord> {
        self.records()
    }

    pub fn usage(&self) -> LocalLibraryUsage {
        let records = self.records();
        LocalLibraryUsage {
            count: records.len(),
            bytes: records.iter().map(|r| r.byte_size).sum(),
        }
    }

    /// Import files that already exist in a readable location (already copied
    /// into the container by the document picker).
    pub fn import_paths(&self, sources: &[PathBuf]) -> LocalImportResult {
        let mut added = Vec::new();
        let mut duplicates = 0usize;
        let mut failed = Vec::new();
        let mut records = self.records();

        for source in sources {
            match self.import_one(source, &records) {
                Ok(Some(record)) => {
                    records.push(record.clone());
                    added.push(record);
                }
                Ok(None) => duplicates += 1,
                Err(err) => failed.push(format!("{}: {err}", source.display())),
            }
        }

        if !added.is_empty() {
            let _ = self
                .index
                .write(&LocalIndex { tracks: records });
        }

        LocalImportResult {
            added,
            duplicates,
            failed,
        }
    }

    fn import_one(
        &self,
        source: &Path,
        existing: &[LocalTrackRecord],
    ) -> AppResult<Option<LocalTrackRecord>> {
        let meta = std::fs::metadata(source)?;
        if !meta.is_file() || meta.len() == 0 {
            return Err(AppError::Invalid("not a readable audio file".into()));
        }

        let file_name = source
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("track")
            .to_string();

        let extension = source
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();

        if !LOCAL_AUDIO_EXTENSIONS.contains(&extension.as_str()) {
            return Err(AppError::Unsupported(format!("unsupported audio type .{extension}")));
        }

        // De-duplicate on (size, file name) — cheap, and stable across imports
        // of the same file from different picker paths.
        if existing
            .iter()
            .any(|r| r.byte_size == meta.len() && r.file_name == file_name)
        {
            return Ok(None);
        }

        let id = uuid::Uuid::new_v4().to_string();
        let audio_file = format!("{id}.{extension}");
        let dest = self.paths.local_audio_dir().join(&audio_file);
        std::fs::copy(source, &dest)?;

        let probed = probe_tags(&dest).unwrap_or_default();

        Ok(Some(LocalTrackRecord {
            id,
            title: probed
                .title
                .unwrap_or_else(|| strip_extension(&file_name)),
            artist: probed.artist.unwrap_or_else(|| "Unknown Artist".into()),
            album: probed.album,
            duration_ms: probed.duration_ms,
            file_name,
            audio_file,
            mime_type: mime_for_extension(&extension).to_string(),
            cover_file: None,
            byte_size: meta.len(),
            added_at: now_ms(),
        }))
    }

    pub fn remove(&self, ids: &[String]) -> AppResult<()> {
        let mut records = self.records();
        let mut kept = Vec::with_capacity(records.len());
        for record in records.drain(..) {
            if ids.iter().any(|id| id == &record.id) {
                let _ = std::fs::remove_file(self.paths.local_audio_dir().join(&record.audio_file));
            } else {
                kept.push(record);
            }
        }
        self.index.write(&LocalIndex { tracks: kept })
    }

    pub fn remove_all(&self) -> AppResult<usize> {
        let records = self.records();
        for record in &records {
            let _ = std::fs::remove_file(self.paths.local_audio_dir().join(&record.audio_file));
        }
        self.index.write(&LocalIndex::default())?;
        Ok(records.len())
    }

    pub fn locate(&self, id: &str) -> Option<PathBuf> {
        self.records()
            .into_iter()
            .find(|r| r.id == id)
            .map(|r| self.paths.local_audio_dir().join(r.audio_file))
            .filter(|p| p.exists())
    }

    pub fn bytes(&self) -> u64 {
        dir_size(&self.paths.local_audio_dir())
    }
}

#[derive(Default, Debug)]
struct ProbedTags {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    duration_ms: u64,
}

/// Read tags + duration with symphonia (already linked for playback, so this
/// adds no new decoder dependency).
fn probe_tags(path: &Path) -> Option<ProbedTags> {
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = std::fs::File::open(path).ok()?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;

    let mut out = ProbedTags::default();
    let mut format = probed.format;

    if let Some(track) = format.default_track() {
        let params = &track.codec_params;
        if let (Some(frames), Some(rate)) = (params.n_frames, params.sample_rate) {
            if rate > 0 {
                out.duration_ms = frames.saturating_mul(1000) / rate as u64;
            }
        }
    }

    if let Some(revision) = format.metadata().current() {
        for tag in revision.tags() {
            let value = tag.value.to_string();
            match tag.key.to_ascii_lowercase().as_str() {
                "title" => out.title = Some(value),
                "artist" => out.artist = Some(value),
                "album" => out.album = Some(value),
                _ => {}
            }
        }
    }

    Some(out)
}

pub fn mime_for_extension(extension: &str) -> &'static str {
    match extension {
        "mp3" => "audio/mpeg",
        "flac" => "audio/flac",
        "m4a" | "aac" => "audio/mp4",
        "ogg" => "audio/ogg",
        "opus" => "audio/opus",
        "wav" => "audio/wav",
        "webm" => "audio/webm",
        _ => "application/octet-stream",
    }
}

fn strip_extension(file_name: &str) -> String {
    match file_name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem.to_string(),
        _ => file_name.to_string(),
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Extension of a saved file, used when the caller only knows a MIME type.
pub fn saved_extension(mime: &str, fallback: &str) -> String {
    extension_for_mime(mime)
        .map(str::to_string)
        .unwrap_or_else(|| fallback.to_string())
}
