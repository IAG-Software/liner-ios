//! `saved-tracks:*` channels.

use serde::Deserialize;
use serde_json::Value;
use tauri::State;

use crate::error::{AppError, AppResult};
use crate::saved_library::{
    LyricsState, SavedLibraryUsage, SavedTrackInput, SavedTrackRecord, SavedTracksPartUsage,
};
use crate::state::AppState;

#[tauri::command]
pub fn saved_tracks_list(state: State<'_, AppState>) -> Vec<SavedTrackRecord> {
    state.saved.list()
}

/// The renderer sends `audio` as a `Uint8Array`; JSON cannot carry binary, so it
/// arrives either as an array of numbers or as a base64 string (the bridge
/// prefers base64 because it is ~4x smaller on the wire).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveInputWire {
    pub meta: crate::saved_library::SavedTrackMeta,
    pub audio: Value,
    #[serde(default)]
    pub mime_type: String,
    #[serde(default)]
    pub cover: Option<CoverWire>,
    #[serde(default)]
    pub lyrics_state: Option<LyricsState>,
    #[serde(default)]
    pub lyrics: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverWire {
    pub data: Value,
    #[serde(default)]
    pub mime_type: String,
}

fn bytes_from_value(value: &Value) -> AppResult<Vec<u8>> {
    match value {
        Value::String(text) => {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD
                .decode(text)
                .map_err(|e| AppError::Invalid(format!("audio base64: {e}")))
        }
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                let byte = item
                    .as_u64()
                    .ok_or_else(|| AppError::Invalid("audio byte is not a number".into()))?;
                out.push(byte as u8);
            }
            Ok(out)
        }
        _ => Err(AppError::Invalid("audio must be base64 or a byte array".into())),
    }
}

#[tauri::command]
pub fn saved_tracks_save(
    state: State<'_, AppState>,
    input: SaveInputWire,
) -> AppResult<Option<SavedTrackRecord>> {
    let audio = bytes_from_value(&input.audio)?;

    let cover = match input.cover {
        Some(cover) => {
            let data = bytes_from_value(&cover.data)?;
            Some(crate::saved_library::SavedCoverInput {
                data,
                mime_type: cover.mime_type,
            })
        }
        None => None,
    };

    state.saved.save(SavedTrackInput {
        meta: input.meta,
        audio,
        mime_type: input.mime_type,
        cover,
        lyrics_state: input.lyrics_state,
        lyrics: input.lyrics,
    })
}

#[tauri::command]
pub fn saved_tracks_remove(state: State<'_, AppState>, track_ids: Vec<String>) -> AppResult<usize> {
    state.saved.remove(&track_ids)
}

#[tauri::command]
pub fn saved_tracks_remove_all(state: State<'_, AppState>) -> AppResult<usize> {
    state.saved.remove_all()
}

#[tauri::command]
pub fn saved_tracks_usage(state: State<'_, AppState>) -> SavedLibraryUsage {
    state.saved.usage()
}

#[tauri::command]
pub fn saved_tracks_usage_of(
    state: State<'_, AppState>,
    track_ids: Vec<String>,
) -> SavedTracksPartUsage {
    state.saved.usage_of(&track_ids)
}

#[tauri::command]
pub fn saved_tracks_open_folder() -> bool {
    false
}

#[tauri::command]
pub fn saved_tracks_get_lyrics(state: State<'_, AppState>, track_id: String) -> Option<Value> {
    state.saved.get_lyrics(&track_id)
}

#[tauri::command]
pub fn saved_tracks_set_lyrics(
    state: State<'_, AppState>,
    track_id: String,
    lyrics_state: LyricsState,
    lyrics: Option<Value>,
) -> AppResult<bool> {
    state.saved.set_lyrics(&track_id, lyrics_state, lyrics)
}
