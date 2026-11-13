//! Native audio transport commands.

use std::path::PathBuf;

use serde::Deserialize;
use tauri::{Emitter, State};

use crate::audio_engine::{Command, QueueItemPublic, Snapshot, TrackMeta};
use crate::audio::dsp::EqualizerSettings;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadRequest {
    /// Stable track id (`yt:…`, `sc:…`, `local:…`).
    pub id: String,
    /// Remote stream URL. Cached before playback.
    #[serde(default)]
    pub url: Option<String>,
    /// Already-local file (saved track / imported file).
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub local_id: Option<String>,
    #[serde(default)]
    pub saved_id: Option<String>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default)]
    pub cover_url: Option<String>,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub autoplay: Option<bool>,
}

async fn resolve_cover(state: &AppState, cover_url: Option<String>) -> Option<String> {
    let url = cover_url?;
    match state.cache.ensure_cover(&url).await {
        Ok(path) => Some(path.to_string_lossy().to_string()),
        Err(err) => {
            log::warn!("cover cache failed: {err}");
            None
        }
    }
}

/// Resolve the request to a decodable local file, downloading if needed.
async fn resolve_path(state: &AppState, request: &LoadRequest) -> AppResult<PathBuf> {
    if let Some(path) = &request.path {
        let path = PathBuf::from(path);
        if !path.exists() {
            return Err(AppError::NotFound(path.display().to_string()));
        }
        return Ok(path);
    }

    if let Some(saved_id) = &request.saved_id {
        return state
            .saved
            .audio_path(saved_id)
            .ok_or_else(|| AppError::NotFound(format!("saved track {saved_id}")));
    }

    if let Some(local_id) = &request.local_id {
        return state
            .local
            .locate(local_id)
            .ok_or_else(|| AppError::NotFound(format!("local track {local_id}")));
    }

    let url = request
        .url
        .as_ref()
        .ok_or_else(|| AppError::Invalid("load requires url, path, localId or savedId".into()))?;

    state.cache.ensure_audio(url, None).await
}

/// `audio:load` — resolve, cache, and (optionally) start playback.
#[tauri::command]
pub async fn audio_load(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request: LoadRequest,
) -> AppResult<Snapshot> {
    let _ = app.emit("audio:loading", &request.id);

    let path = resolve_path(&state, &request).await?;
    let cover_path = resolve_cover(&state, request.cover_url.clone()).await;

    let meta = TrackMeta {
        id: request.id.clone(),
        title: request.title.clone(),
        artist: request.artist.clone(),
        album: request.album.clone(),
        cover_path,
        duration_ms: request.duration_ms,
    };

    state.audio.send(Command::PlayFile {
        path,
        meta,
        autoplay: request.autoplay.unwrap_or(true),
    })?;

    Ok(state.audio.snapshot())
}

#[tauri::command]
pub fn audio_play(state: State<'_, AppState>) -> AppResult<()> {
    state.audio.send(Command::Play)
}

#[tauri::command]
pub fn audio_pause(state: State<'_, AppState>) -> AppResult<()> {
    state.audio.send(Command::Pause)
}

#[tauri::command]
pub fn audio_seek(state: State<'_, AppState>, position_ms: u64) -> AppResult<()> {
    state.audio.send(Command::Seek(position_ms))
}

#[tauri::command]
pub fn audio_stop(state: State<'_, AppState>) -> AppResult<()> {
    state.audio.send(Command::Stop)
}

#[tauri::command]
pub fn audio_set_volume(state: State<'_, AppState>, volume: f32) -> AppResult<()> {
    state.audio.set_volume(volume)
}

#[tauri::command]
pub fn audio_set_rate(state: State<'_, AppState>, rate: f32) -> AppResult<()> {
    state.audio.send(Command::SetRate(rate))
}

#[tauri::command]
pub fn audio_enqueue(state: State<'_, AppState>, item: QueueItemPublic) -> AppResult<()> {
    state.audio.send(Command::Enqueue(item))
}

#[tauri::command]
pub fn audio_set_queue(state: State<'_, AppState>, items: Vec<QueueItemPublic>) -> AppResult<()> {
    state.audio.send(Command::SetQueue(items))
}

#[tauri::command]
pub fn audio_clear_queue(state: State<'_, AppState>) -> AppResult<()> {
    state.audio.send(Command::ClearQueue)
}

#[tauri::command]
pub fn audio_set_equalizer(
    state: State<'_, AppState>,
    settings: EqualizerSettings,
) -> AppResult<()> {
    state.audio.send(Command::SetEqualizer(settings))
}

#[tauri::command]
pub fn audio_state(state: State<'_, AppState>) -> Snapshot {
    state.audio.snapshot()
}

/// Local URL the WebView can hand to `<audio>` / `<img>` when it wants a media
/// element instead of the native engine (e.g. the ambient/visualiser graphs).
///
/// Served by the `liner-local` URI scheme with HTTP range support, so seeking
/// and artwork cropping behave like a normal origin.
#[tauri::command]
pub async fn audio_playback_url(
    state: State<'_, AppState>,
    url: Option<String>,
    track_id: Option<String>,
) -> AppResult<Option<String>> {
    // Already-cached content is served straight from disk.
    if let Some(id) = track_id.as_deref() {
        if let Some(path) = state.saved.audio_path(id) {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                return Ok(Some(format!("liner-local://localhost/saved/audio/{name}")));
            }
        }
    }

    let Some(url) = url else {
        return Ok(None);
    };

    let path = state.cache.ensure_audio(&url, None).await?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| AppError::Invalid("cached file has no name".into()))?;

    Ok(Some(format!("liner-local://localhost/cache/audio/{name}")))
}
