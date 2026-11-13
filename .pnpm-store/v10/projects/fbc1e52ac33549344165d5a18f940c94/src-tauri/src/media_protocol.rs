//! `liner-local://` URI scheme handler.
//!
//! WKWebView cannot load `file://` from a custom scheme document, and the
//! `asset:` protocol only exposes allow-listed paths. A dedicated scheme keeps
//! the media origin separate from the app document origin and lets us implement
//! HTTP byte ranges — which is what `<audio>` needs to seek without downloading
//! the whole file.
//!
//! URL shape: `liner-local://localhost/<root>/<kind>/<file>`
//! where `<root>` is `cache` | `saved` | `local` and `<kind>` is `audio` |
//! `covers`.

use std::path::{Path, PathBuf};

use tauri::http::{header, Request, Response, StatusCode};
use tauri::{Manager, UriSchemeContext};

use crate::state::AppState;

pub fn handle<R: tauri::Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let state = ctx.app_handle().state::<AppState>();

    let path = request.uri().path().trim_start_matches('/').to_string();
    let resolved = resolve(&state, &path);

    let Some(file) = resolved else {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(b"not found".to_vec())
            .unwrap_or_default();
    };

    let range = request
        .headers()
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_range);

    match build_response(&file, range) {
        Ok(response) => response,
        Err(err) => {
            log::error!("liner-local: {err}");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Vec::new())
                .unwrap_or_default()
        }
    }
}

fn resolve(state: &AppState, path: &str) -> Option<PathBuf> {
    let mut parts = path.splitn(3, '/');
    let root = parts.next()?;
    let kind = parts.next()?;
    let name = parts.next()?;

    // Path traversal guard: the handler only ever serves flat file names.
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return None;
    }

    let base = match (root, kind) {
        ("cache", "audio") => state.paths.audio_cache_dir(),
        ("cache", "covers") => state.paths.cover_cache_dir(),
        ("saved", "audio") => state.paths.saved_audio_dir(),
        ("saved", "covers") => state.paths.saved_cover_dir(),
        ("local", "audio") => state.paths.local_audio_dir(),
        _ => return None,
    };

    let candidate = base.join(name);
    candidate.is_file().then_some(candidate)
}

fn content_type(file: &Path) -> &'static str {
    match file
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("mp3") => "audio/mpeg",
        Some("m4a") | Some("mp4") | Some("aac") => "audio/mp4",
        Some("flac") => "audio/flac",
        Some("ogg") => "audio/ogg",
        Some("opus") => "audio/opus",
        Some("wav") => "audio/wav",
        Some("webm") => "audio/webm",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}

#[derive(Clone, Copy)]
struct ByteRange {
    start: u64,
    end: u64,
}

/// Accepts a single `bytes=start-end` / `bytes=start-` / `bytes=-suffix` range.
/// Multi-range requests fall back to a full 200 response, which every WebKit
/// version handles.
fn parse_range(value: &str) -> Option<ByteRange> {
    let spec = value.strip_prefix("bytes=")?;
    if spec.contains(',') {
        return None;
    }
    let (start, end) = spec.split_once('-')?;
    let start = start.trim();
    let end = end.trim();

    if start.is_empty() {
        let suffix: u64 = end.parse().ok()?;
        return Some(ByteRange {
            start: u64::MAX,
            end: suffix,
        });
    }

    let start: u64 = start.parse().ok()?;
    let end: u64 = if end.is_empty() {
        u64::MAX
    } else {
        end.parse().ok()?
    };
    (start <= end).then_some(ByteRange { start, end })
}

fn build_response(
    file: &Path,
    range: Option<ByteRange>,
) -> Result<Response<Vec<u8>>, std::io::Error> {
    let bytes = std::fs::read(file)?;
    let total = bytes.len() as u64;
    let mime = content_type(file);

    let Some(range) = range else {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .header(header::CONTENT_LENGTH, total)
            .header(header::ACCEPT_RANGES, "bytes")
            .header(header::CACHE_CONTROL, "private, max-age=31536000, immutable")
            .body(bytes)
            .unwrap_or_default());
    };

    // `start == u64::MAX` encodes a suffix request (`bytes=-N`).
    let (start, end) = if range.start == u64::MAX {
        let suffix = range.end.min(total);
        (total.saturating_sub(suffix), total.saturating_sub(1))
    } else {
        (range.start.min(total.saturating_sub(1)), range.end.min(total.saturating_sub(1)))
    };

    if start > end || total == 0 {
        return Ok(Response::builder()
            .status(StatusCode::RANGE_NOT_SATISFIABLE)
            .header(header::CONTENT_RANGE, format!("bytes */{total}"))
            .body(Vec::new())
            .unwrap_or_default());
    }

    let slice = bytes[start as usize..=end as usize].to_vec();
    Ok(Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, slice.len())
        .header(
            header::CONTENT_RANGE,
            format!("bytes {start}-{end}/{total}"),
        )
        .header(header::ACCEPT_RANGES, "bytes")
        .body(slice)
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_open_ended_range() {
        let range = parse_range("bytes=100-").unwrap();
        assert_eq!(range.start, 100);
        assert_eq!(range.end, u64::MAX);
    }

    #[test]
    fn rejects_multi_range() {
        assert!(parse_range("bytes=0-10,20-30").is_none());
    }

    #[test]
    fn parses_suffix_range() {
        let range = parse_range("bytes=-500").unwrap();
        assert_eq!(range.start, u64::MAX);
        assert_eq!(range.end, 500);
    }
}
