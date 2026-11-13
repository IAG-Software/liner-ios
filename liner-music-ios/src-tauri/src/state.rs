//! Process-wide application state.
//!
//! One `AppState` is created in `Builder::setup` and handed to Tauri's
//! `manage`; commands reach it through `State<'_, AppState>`.

use std::sync::{Arc, OnceLock};

use tauri::AppHandle;

use crate::audio_engine::AudioEngine;
use crate::auth::AuthStore;
use crate::cache_manager::CacheManager;
use crate::error::{AppError, AppResult};
use crate::local_library::LocalLibrary;
use crate::lyrics_sync::LyricsEngine;
use crate::paths::AppPaths;
use crate::saved_library::SavedLibrary;
use crate::signer::SignerState;

/// The audio worker and the iOS platform layer are not tied to a specific
/// command, so they need a handle they can emit from. Storing it once avoids
/// cloning an `AppHandle` into every thread.
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

pub fn current_handle() -> Option<&'static AppHandle> {
    APP_HANDLE.get()
}

pub struct AppState {
    pub paths: AppPaths,
    pub signer: SignerState,
    pub auth: AuthStore,
    pub cache: Arc<CacheManager>,
    pub local: Arc<LocalLibrary>,
    pub saved: Arc<SavedLibrary>,
    pub audio: Arc<AudioEngine>,
    pub lyrics: Arc<LyricsEngine>,
    pub http: reqwest::Client,
}

impl AppState {
    pub fn initialize(app: &AppHandle) -> AppResult<Self> {
        let _ = APP_HANDLE.set(app.clone());

        let paths = AppPaths::resolve(app)?;

        // One client for the whole process: connection pooling matters on
        // cellular, and rustls avoids any OpenSSL dependency on device.
        // The User-Agent is part of the contract with the API's WAF — see
        // `http_client.rs`.
        let http = crate::http_client::build_client()
            .map_err(|e| AppError::Network(format!("http client: {e}")))?;

        let lyrics = LyricsEngine::new();
        let audio = AudioEngine::new(lyrics.clone())?;

        Ok(Self {
            cache: Arc::new(CacheManager::new(paths.clone(), http.clone())),
            local: LocalLibrary::new(&paths),
            saved: Arc::new(SavedLibrary::new(&paths)),
            auth: AuthStore::new(&paths),
            signer: SignerState::default(),
            lyrics,
            audio,
            http,
            paths,
        })
    }
}
