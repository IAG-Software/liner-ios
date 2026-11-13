//! Syllable-accurate lyric scheduling.
//!
//! The renderer already parses Musixmatch richsync / TTML / LRC into a
//! `SyncedLine[]` (see `src/features/lyrics/engine/lyricsParser.ts`). That parse
//! is *not* repeated here — the same structure is handed to Rust once, compiled
//! into a flat, binary-searchable timeline, and then driven by the audio
//! engine's audible sample position.
//!
//! Why native rather than a `setInterval` in JS: a WKWebView timer stalls when
//! the app is backgrounded and drifts against CoreAudio under load. The audio
//! worker owns a monotonic frame counter, so every event carries an exact
//! `frame` index and the UI can re-anchor on it after any jank.

use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::audio::PIPELINE_RATE;
use crate::error::{AppError, AppResult};

/// One syllable / word token.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordInput {
    #[serde(default)]
    pub time_ms: i64,
    #[serde(default)]
    pub end_ms: i64,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub is_background: bool,
}

/// One lyric line, exactly as the renderer parsed it.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LineInput {
    #[serde(default)]
    pub time_ms: i64,
    #[serde(default)]
    pub duration_ms: Option<i64>,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub is_instrumental: bool,
    #[serde(default)]
    pub words: Option<Vec<WordInput>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsPayload {
    pub lines: Vec<LineInput>,
    /// User-applied global offset (settings → lyrics offset).
    #[serde(default)]
    pub offset_ms: i64,
}

#[derive(Debug, Clone)]
pub struct CompiledWord {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct CompiledLine {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub instrumental: bool,
    pub words: Vec<CompiledWord>,
}

#[derive(Debug, Clone)]
pub struct CompiledTrack {
    pub lines: Vec<CompiledLine>,
}

impl CompiledTrack {
    fn from_input(payload: LyricsPayload) -> Self {
        let offset = payload.offset_ms;
        let mut lines: Vec<CompiledLine> = payload
            .lines
            .into_iter()
            .filter(|line| !line.text.trim().is_empty() || line.is_instrumental)
            .map(|line| {
                let start = (line.time_ms + offset).max(0);
                let explicit_end = line
                    .duration_ms
                    .filter(|d| *d > 0)
                    .map(|d| start + d);
                let words: Vec<CompiledWord> = line
                    .words
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|w| !w.text.is_empty())
                    .map(|w| {
                        let w_start = (w.time_ms + offset).max(0);
                        // A token with no end inherits the next token's start;
                        // that is fixed up below.
                        CompiledWord {
                            start_ms: w_start,
                            end_ms: if w.end_ms > w.time_ms {
                                (w.end_ms + offset).max(w_start)
                            } else {
                                w_start
                            },
                            text: w.text,
                        }
                    })
                    .collect();

                let end = explicit_end
                    .or_else(|| words.last().map(|w| w.end_ms.max(w.start_ms + 400)))
                    .unwrap_or(start + 2_400);

                CompiledLine {
                    start_ms: start,
                    end_ms: end.max(start),
                    text: line.text,
                    instrumental: line.is_instrumental,
                    words,
                }
            })
            .collect();

        // A token whose `endMs` was missing ends where the next token begins.
        for line in &mut lines {
            for index in 0..line.words.len() {
                if line.words[index].end_ms <= line.words[index].start_ms {
                    let next = line
                        .words
                        .get(index + 1)
                        .map(|w| w.start_ms)
                        .unwrap_or(line.end_ms);
                    line.words[index].end_ms = next.max(line.words[index].start_ms + 80);
                }
            }
        }

        lines.sort_by_key(|line| line.start_ms);
        Self { lines }
    }

    /// Index of the line that should be on screen at `position_ms`.
    ///
    /// A line stays active until the *next* line starts (that is what makes the
    /// scroll feel continuous), falling back to the line's own end when it is
    /// the last one.
    pub fn line_index_at(&self, position_ms: i64) -> Option<usize> {
        if self.lines.is_empty() || position_ms < self.lines[0].start_ms {
            return None;
        }
        let mut low = 0usize;
        let mut high = self.lines.len() - 1;
        while low < high {
            let mid = (low + high + 1) / 2;
            if self.lines[mid].start_ms <= position_ms {
                low = mid;
            } else {
                high = mid - 1;
            }
        }
        Some(low)
    }

    fn word_index_at(line: &CompiledLine, position_ms: i64) -> Option<usize> {
        if line.words.is_empty() || position_ms < line.words[0].start_ms {
            return None;
        }
        let mut index = None;
        for (i, word) in line.words.iter().enumerate() {
            if position_ms >= word.start_ms {
                index = Some(i);
            } else {
                break;
            }
        }
        index
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineEvent {
    pub index: usize,
    pub text: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub instrumental: bool,
    pub frame: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WordEvent {
    pub line_index: usize,
    pub word_index: usize,
    pub text: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub progress: f32,
    pub frame: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub position_ms: u64,
    pub frame: u64,
    pub line_index: Option<usize>,
    pub word_index: Option<usize>,
    pub line_progress: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Cursor {
    line: Option<usize>,
    word: Option<usize>,
}

#[derive(Default)]
struct Pending {
    line: Option<LineEvent>,
    word: Option<WordEvent>,
    progress: Option<ProgressEvent>,
}

pub struct LyricsEngine {
    track: Mutex<Option<Arc<CompiledTrack>>>,
    cursor: Mutex<Cursor>,
    pending: Mutex<Pending>,
}

impl LyricsEngine {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            track: Mutex::new(None),
            cursor: Mutex::new(Cursor {
                line: None,
                word: None,
            }),
            pending: Mutex::new(Pending::default()),
        })
    }

    pub fn load(&self, payload: Value) -> AppResult<usize> {
        let parsed: LyricsPayload = serde_json::from_value(payload).map_err(|e| {
            AppError::Invalid(format!("lyrics payload does not match SyncedLine[]: {e}"))
        })?;

        let compiled = Arc::new(CompiledTrack::from_input(parsed));
        let count = compiled.lines.len();

        *self.track.lock() = if count == 0 { None } else { Some(compiled) };
        *self.cursor.lock() = Cursor {
            line: None,
            word: None,
        };
        *self.pending.lock() = Pending::default();

        Ok(count)
    }

    pub fn clear(&self) {
        *self.track.lock() = None;
        *self.cursor.lock() = Cursor {
            line: None,
            word: None,
        };
        *self.pending.lock() = Pending::default();
    }

    pub fn is_loaded(&self) -> bool {
        self.track.lock().is_some()
    }

    /// Current view of the timeline at `position_ms` (used by `lyrics_current`).
    pub fn current(&self, position_ms: u64) -> ProgressEvent {
        let (line_index, word_index, line_progress) = self.resolve(position_ms as i64);
        ProgressEvent {
            position_ms,
            frame: frames_for(position_ms),
            line_index,
            word_index,
            line_progress,
        }
    }

    fn resolve(&self, position_ms: i64) -> (Option<usize>, Option<usize>, f32) {
        let guard = self.track.lock();
        let Some(track) = guard.as_ref() else {
            return (None, None, 0.0);
        };
        let Some(line_index) = track.line_index_at(position_ms) else {
            return (None, None, 0.0);
        };
        let line = &track.lines[line_index];
        let word_index = CompiledTrack::word_index_at(line, position_ms);
        let span = (line.end_ms - line.start_ms).max(1) as f32;
        let progress = ((position_ms - line.start_ms) as f32 / span).clamp(0.0, 1.0);
        (Some(line_index), word_index, progress)
    }

    /// Called by the audio worker on every publish tick.
    pub fn on_position(&self, position_ms: u64) {
        let (line_index, word_index, line_progress) = self.resolve(position_ms as i64);
        let frame = frames_for(position_ms);

        let mut cursor = self.cursor.lock();
        let mut pending = self.pending.lock();
        let guard = self.track.lock();

        if let (Some(track), Some(index)) = (guard.as_ref(), line_index) {
            if cursor.line != Some(index) {
                cursor.line = Some(index);
                cursor.word = None;
                let line = &track.lines[index];
                pending.line = Some(LineEvent {
                    index,
                    text: line.text.clone(),
                    start_ms: line.start_ms,
                    end_ms: line.end_ms,
                    instrumental: line.instrumental,
                    frame,
                });
            }

            if cursor.word != word_index {
                cursor.word = word_index;
                if let (Some(w), Some(line_idx)) = (word_index, line_index) {
                    if let Some(word) = track.lines[line_idx].words.get(w) {
                        let span = (word.end_ms - word.start_ms).max(1) as f32;
                        let progress =
                            ((position_ms as i64 - word.start_ms) as f32 / span).clamp(0.0, 1.0);
                        pending.word = Some(WordEvent {
                            line_index: line_idx,
                            word_index: w,
                            text: word.text.clone(),
                            start_ms: word.start_ms,
                            end_ms: word.end_ms,
                            progress,
                            frame,
                        });
                    }
                }
            }
        } else if line_index.is_none() {
            cursor.line = None;
            cursor.word = None;
        }

        pending.progress = Some(ProgressEvent {
            position_ms,
            frame,
            line_index,
            word_index,
            line_progress,
        });
    }

    /// Emit whatever became true since the last tick.
    pub fn flush_events(&self, app: &tauri::AppHandle) {
        use tauri::Emitter;
        let mut pending = self.pending.lock();
        if let Some(line) = pending.line.take() {
            let _ = app.emit("lyrics:line", line);
        }
        if let Some(word) = pending.word.take() {
            let _ = app.emit("lyrics:word", word);
        }
        if let Some(progress) = pending.progress.take() {
            let _ = app.emit("lyrics:progress", progress);
        }
    }
}

fn frames_for(position_ms: u64) -> u64 {
    position_ms.saturating_mul(PIPELINE_RATE as u64) / 1000
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn payload() -> Value {
        json!({
            "lines": [
                {
                    "timeMs": 1000,
                    "durationMs": 2000,
                    "text": "hello world",
                    "words": [
                        { "timeMs": 1000, "endMs": 1500, "text": "hello " },
                        { "timeMs": 1500, "endMs": 3000, "text": "world" }
                    ]
                },
                { "timeMs": 3000, "durationMs": 1000, "text": "next line" }
            ]
        })
    }

    #[test]
    fn compiles_and_finds_active_line() {
        let engine = LyricsEngine::new();
        assert_eq!(engine.load(payload()).unwrap(), 2);

        let early = engine.current(0);
        assert_eq!(early.line_index, None);

        let first = engine.current(1_200);
        assert_eq!(first.line_index, Some(0));
        assert_eq!(first.word_index, Some(0));

        let second_word = engine.current(1_600);
        assert_eq!(second_word.word_index, Some(1));

        let second_line = engine.current(3_100);
        assert_eq!(second_line.line_index, Some(1));
    }

    #[test]
    fn offset_shifts_timeline() {
        let engine = LyricsEngine::new();
        let mut value = payload();
        value["offsetMs"] = json!(500);
        engine.load(value).unwrap();
        // Original 1000 ms line now starts at 1500 ms.
        assert_eq!(engine.current(1_200).line_index, None);
        assert_eq!(engine.current(1_600).line_index, Some(0));
    }

    #[test]
    fn missing_word_end_is_inferred() {
        let engine = LyricsEngine::new();
        engine
            .load(json!({
                "lines": [{
                    "timeMs": 0,
                    "durationMs": 2000,
                    "text": "ab",
                    "words": [
                        { "timeMs": 0, "text": "a" },
                        { "timeMs": 500, "endMs": 1000, "text": "b" }
                    ]
                }]
            }))
            .unwrap();
        assert_eq!(engine.current(200).word_index, Some(0));
        assert_eq!(engine.current(600).word_index, Some(1));
    }
}
