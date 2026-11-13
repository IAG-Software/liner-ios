//! Offline "saved tracks" library.
//!
//! Layout (identical in spirit to `electron/saved-tracks/library.ts`):
//!
//! ```text
//! saved/
//!   library.json          [{ key, trackId, title, …, audioFile, coverFile, lyrics }]
//!   audio/<key>.<ext>
//!   covers/<key>.<ext>
//!   lyrics/<key>.json
//! ```
//!
//! Unlike the audio *cache*, this data is user-owned: it lives in
//! `Application Support`, is never evicted by iOS, and is included in backups.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{AppError, AppResult};
use crate::jsonstore::{extension_for_mime, write_atomic, JsonFile};
use crate::local_library::mime_for_extension;
use crate::paths::AppPaths;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LyricsState {
    Found,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedTrackMeta {
    pub track_id: String,
    pub title: String,
    pub artists: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist_id: Option<String>,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explicit: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedTrackRecord {
    #[serde(flatten)]
    pub meta: SavedTrackMeta,
    pub key: String,
    pub audio_file: String,
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_file: Option<String>,
    pub byte_size: u64,
    pub saved_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lyrics: Option<LyricsState>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedTrackInput {
    pub meta: SavedTrackMeta,
    /// Raw audio bytes arrive as a JSON array of numbers from the WebView
    /// (`invoke` cannot carry a `Uint8Array` over the postMessage bridge).
    pub audio: Vec<u8>,
    #[serde(default)]
    pub mime_type: String,
    #[serde(default)]
    pub cover: Option<SavedCoverInput>,
    #[serde(default)]
    pub lyrics_state: Option<LyricsState>,
    #[serde(default)]
    pub lyrics: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedCoverInput {
    pub data: Vec<u8>,
    pub mime_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedLibraryUsage {
    pub count: usize,
    pub bytes: u64,
    pub free_bytes: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedTracksPartUsage {
    pub count: usize,
    pub bytes: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SavedIndex {
    tracks: Vec<SavedTrackRecord>,
}

pub struct SavedLibrary {
    paths: AppPaths,
    index: JsonFile,
}

impl SavedLibrary {
    pub fn new(paths: &AppPaths) -> Self {
        Self {
            paths: paths.clone(),
            index: JsonFile::new(paths.saved_index_file()),
        }
    }

    fn records(&self) -> Vec<SavedTrackRecord> {
        self.index.read::<SavedIndex>().unwrap_or_default().tracks
    }

    pub fn list(&self) -> Vec<SavedTrackRecord> {
        self.records()
    }

    pub fn save(&self, input: SavedTrackInput) -> AppResult<Option<SavedTrackRecord>> {
        if input.audio.is_empty() {
            return Ok(None);
        }

        let key = sanitize_key(&input.meta.track_id);
        if key.is_empty() {
            return Err(AppError::Invalid("trackId required".into()));
        }

        let extension = extension_for_mime(&input.mime_type).unwrap_or("mp3");
        let audio_file = format!("{key}.{extension}");
        write_atomic(
            &self.paths.saved_audio_dir().join(&audio_file),
            &input.audio,
        )?;

        let cover_file = match &input.cover {
            Some(cover) if !cover.data.is_empty() => {
                let ext = extension_for_mime(&cover.mime_type).unwrap_or("jpg");
                let name = format!("{key}.{ext}");
                write_atomic(&self.paths.saved_cover_dir().join(&name), &cover.data)?;
                Some(name)
            }
            _ => self
                .records()
                .into_iter()
                .find(|r| r.key == key)
                .and_then(|r| r.cover_file),
        };

        if let Some(lyrics) = &input.lyrics {
            write_atomic(
                &self.paths.saved_lyrics_dir().join(format!("{key}.json")),
                &serde_json::to_vec(lyrics)?,
            )?;
        }

        let record = SavedTrackRecord {
            meta: input.meta,
            key: key.clone(),
            audio_file,
            mime_type: if input.mime_type.is_empty() {
                mime_for_extension(extension).to_string()
            } else {
                input.mime_type
            },
            cover_file,
            byte_size: input.audio.len() as u64,
            saved_at: now_ms(),
            lyrics: input.lyrics_state,
        };

        let mut records = self.records();
        match records.iter().position(|r| r.key == key) {
            Some(index) => records[index] = record.clone(),
            None => records.push(record.clone()),
        }
        self.index.write(&SavedIndex { tracks: records })?;

        Ok(Some(record))
    }

    pub fn remove(&self, track_ids: &[String]) -> AppResult<usize> {
        let keys: Vec<String> = track_ids.iter().map(|id| sanitize_key(id)).collect();
        let mut records = self.records();
        let before = records.len();

        records.retain(|record| {
            let drop_it = keys.contains(&record.key);
            if drop_it {
                self.unlink(record);
            }
            !drop_it
        });

        let removed = before - records.len();
        self.index.write(&SavedIndex { tracks: records })?;
        Ok(removed)
    }

    fn unlink(&self, record: &SavedTrackRecord) {
        let _ = std::fs::remove_file(self.paths.saved_audio_dir().join(&record.audio_file));
        if let Some(cover) = &record.cover_file {
            let _ = std::fs::remove_file(self.paths.saved_cover_dir().join(cover));
        }
        let _ = std::fs::remove_file(
            self.paths
                .saved_lyrics_dir()
                .join(format!("{}.json", record.key)),
        );
    }

    pub fn remove_all(&self) -> AppResult<usize> {
        let records = self.records();
        for record in &records {
            self.unlink(record);
        }
        let count = records.len();
        self.index.write(&SavedIndex::default())?;
        Ok(count)
    }

    pub fn usage(&self) -> SavedLibraryUsage {
        let records = self.records();
        SavedLibraryUsage {
            count: records.len(),
            bytes: records.iter().map(|r| r.byte_size).sum(),
            free_bytes: free_bytes(&self.paths.data),
        }
    }

    pub fn usage_of(&self, track_ids: &[String]) -> SavedTracksPartUsage {
        let keys: Vec<String> = track_ids.iter().map(|id| sanitize_key(id)).collect();
        let records: Vec<_> = self
            .records()
            .into_iter()
            .filter(|r| keys.contains(&r.key))
            .collect();
        SavedTracksPartUsage {
            count: records.len(),
            bytes: records.iter().map(|r| r.byte_size).sum(),
        }
    }

    pub fn get_lyrics(&self, track_id: &str) -> Option<Value> {
        let key = sanitize_key(track_id);
        let path = self.paths.saved_lyrics_dir().join(format!("{key}.json"));
        let raw = std::fs::read(path).ok()?;
        serde_json::from_slice(&raw).ok()
    }

    pub fn set_lyrics(
        &self,
        track_id: &str,
        state: LyricsState,
        lyrics: Option<Value>,
    ) -> AppResult<bool> {
        let key = sanitize_key(track_id);
        if let Some(lyrics) = lyrics {
            write_atomic(
                &self.paths.saved_lyrics_dir().join(format!("{key}.json")),
                &serde_json::to_vec(&lyrics)?,
            )?;
        }

        let mut records = self.records();
        let Some(record) = records.iter_mut().find(|r| r.key == key) else {
            return Ok(false);
        };
        record.lyrics = Some(state);
        self.index.write(&SavedIndex { tracks: records })?;
        Ok(true)
    }

    /// Absolute path of a saved audio file, for handing to the native decoder.
    pub fn audio_path(&self, track_id: &str) -> Option<PathBuf> {
        let key = sanitize_key(track_id);
        self.records()
            .into_iter()
            .find(|r| r.key == key)
            .map(|r| self.paths.saved_audio_dir().join(r.audio_file))
            .filter(|p| p.exists())
    }
}

/// Track ids are URL-ish (`yt:abc/def`); keep them filesystem safe without
/// losing uniqueness.
pub fn sanitize_key(track_id: &str) -> String {
    track_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('.')
        .to_string()
}

fn free_bytes(path: &std::path::Path) -> Option<u64> {
    // `statvfs` is not exposed by std; on iOS the sandbox reports a quota, so
    // this is best-effort and only used for the settings screen estimate.
    #[cfg(target_os = "ios")]
    {
        let _ = path;
        None
    }
    #[cfg(not(target_os = "ios"))]
    {
        let _ = path;
        None
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
