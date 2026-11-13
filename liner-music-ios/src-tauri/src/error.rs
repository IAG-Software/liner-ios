//! Error type shared by every command.
//!
//! Tauri serialises `Err` values back to the renderer, so this implements
//! `Serialize` as a plain string — the renderer only ever logs it.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),

    #[error("tauri: {0}")]
    Tauri(#[from] tauri::Error),

    #[error("network: {0}")]
    Network(String),

    #[error("audio: {0}")]
    Audio(String),

    #[error("decode: {0}")]
    Decode(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid input: {0}")]
    Invalid(String),

    #[error("unsupported on this platform: {0}")]
    Unsupported(String),

    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(value: reqwest::Error) -> Self {
        AppError::Network(value.to_string())
    }
}

impl From<String> for AppError {
    fn from(value: String) -> Self {
        AppError::Other(value)
    }
}

impl From<&str> for AppError {
    fn from(value: &str) -> Self {
        AppError::Other(value.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
