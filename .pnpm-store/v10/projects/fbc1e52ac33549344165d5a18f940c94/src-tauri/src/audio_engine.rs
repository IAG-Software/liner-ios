//! Asynchronous playback state machine.
//!
//! Threading model (deliberately boring, because audio does not tolerate
//! cleverness):
//!
//! ```text
//!   Tauri command (tokio)          audio worker (std::thread)        CoreAudio
//!   ─────────────────────          ──────────────────────────        ─────────
//!   cache download  ──Command──▶   decode → resample → DSP ──ring──▶  render cb
//!                                  ▲                                   │
//!                                  └──── position / underrun ◀─────────┘
//! ```
//!
//! * the worker owns the `cpal::Stream` (it is `!Send` on Apple platforms),
//!   the `symphonia` decoder, and the gapless look-ahead;
//! * the render callback only copies from a lock-free ring and applies a
//!   smoothed gain — no allocation, no locks;
//! * every user action is a message, so `seek` during a decode cannot tear
//!   state: it is applied between packets.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::audio::decode::DecoderState;
use crate::audio::dsp::{DspChain, EqualizerSettings};
use crate::audio::output::{self, OutputShared};
use crate::audio::resample::Resampler;
use crate::audio::ring::SpscRing;
use crate::audio::{PIPELINE_RATE, RING_FRAMES};
use crate::error::{AppError, AppResult};
use crate::lyrics_sync::LyricsEngine;

/// Metadata the UI needs about whatever is playing.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TrackMeta {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub album: Option<String>,
    #[serde(default)]
    pub cover_path: Option<String>,
    #[serde(default)]
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
struct QueueItem {
    path: PathBuf,
    meta: TrackMeta,
}

pub enum Command {
    PlayFile {
        path: PathBuf,
        meta: TrackMeta,
        autoplay: bool,
    },
    Enqueue(QueueItemPublic),
    SetQueue(Vec<QueueItemPublic>),
    ClearQueue,
    Play,
    Pause,
    Stop,
    Next,
    Previous,
    Seek(u64),
    SetVolume(f32),
    SetRate(f32),
    SetEqualizer(EqualizerSettings),
    Snapshot(Sender<Snapshot>),
    Shutdown,
}

/// Serializable form of a queued track (the renderer owns the queue order).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueItemPublic {
    pub path: String,
    #[serde(flatten)]
    pub meta: TrackMeta,
}

impl QueueItemPublic {
    fn into_internal(self) -> QueueItem {
        QueueItem {
            path: PathBuf::from(self.path),
            meta: self.meta,
        }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub track_id: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub cover_path: Option<String>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub playing: bool,
    pub buffering: bool,
    pub volume: f32,
    pub rate: f32,
    pub queue_len: usize,
    pub queue_index: usize,
    pub underruns: u32,
}

/// Shared transport state for cheap synchronous reads.
#[derive(Debug, Clone, Default)]
struct Transport {
    position_ms: u64,
    duration_ms: u64,
    playing: bool,
    buffering: bool,
    volume: f32,
    rate: f32,
    track: Option<TrackMeta>,
    queue_len: usize,
    queue_index: usize,
}

pub struct AudioEngine {
    tx: Sender<Command>,
    shared: Arc<OutputShared>,
    state: Arc<Mutex<Transport>>,
    handle: Mutex<Option<AppHandle>>,
    lyrics: Arc<LyricsEngine>,
}

impl AudioEngine {
    pub fn new(lyrics: Arc<LyricsEngine>) -> AppResult<Arc<Self>> {
        let ring = Arc::new(SpscRing::new(RING_FRAMES));
        let shared = Arc::new(OutputShared::new(ring.clone()));
        let state = Arc::new(Mutex::new(Transport {
            volume: 1.0,
            rate: 1.0,
            ..Default::default()
        }));

        let (tx, rx) = mpsc::channel();

        let engine = Arc::new(Self {
            tx,
            shared: shared.clone(),
            state: state.clone(),
            handle: Mutex::new(None),
            lyrics,
        });

        let worker_state = state.clone();
        let worker_shared = shared.clone();
        let worker_lyrics = engine.lyrics.clone();

        std::thread::Builder::new()
            .name("liner-audio".into())
            .spawn(move || {
                let mut worker = Worker::new(ring, worker_shared, worker_state, worker_lyrics);
                worker.run(rx);
            })
            .map_err(|e| AppError::Audio(format!("spawn audio thread: {e}")))?;

        Ok(engine)
    }

    pub fn attach_handle(&self, handle: AppHandle) {
        *self.handle.lock() = Some(handle);
    }

    pub fn handle(&self) -> Option<AppHandle> {
        self.handle.lock().clone()
    }

    pub fn send(&self, command: Command) -> AppResult<()> {
        self.tx
            .send(command)
            .map_err(|_| AppError::Audio("audio worker stopped".into()))
    }

    pub fn snapshot(&self) -> Snapshot {
        let (tx, rx) = mpsc::channel();
        if self.send(Command::Snapshot(tx)).is_ok() {
            if let Ok(snapshot) = rx.recv_timeout(Duration::from_millis(500)) {
                return snapshot;
            }
        }
        // Worker busy or gone: answer from the cached mirror.
        let transport = self.state.lock().clone();
        Snapshot {
            track_id: transport.track.as_ref().map(|t| t.id.clone()),
            title: transport.track.as_ref().map(|t| t.title.clone()),
            artist: transport.track.as_ref().map(|t| t.artist.clone()),
            album: transport.track.as_ref().and_then(|t| t.album.clone()),
            cover_path: transport.track.as_ref().and_then(|t| t.cover_path.clone()),
            position_ms: transport.position_ms,
            duration_ms: transport.duration_ms,
            playing: transport.playing,
            buffering: transport.buffering,
            volume: transport.volume,
            rate: transport.rate,
            queue_len: transport.queue_len,
            queue_index: transport.queue_index,
            underruns: self.shared.underruns.load(Ordering::Relaxed),
        }
    }

    pub fn set_volume(&self, volume: f32) -> AppResult<()> {
        self.shared.set_volume(volume);
        self.state.lock().volume = volume.clamp(0.0, 1.0);
        self.send(Command::SetVolume(volume))
    }

    pub fn is_playing(&self) -> bool {
        self.shared.playing.load(Ordering::Relaxed)
    }

    pub fn shared(&self) -> &Arc<OutputShared> {
        &self.shared
    }
}

struct Worker {
    ring: Arc<SpscRing>,
    shared: Arc<OutputShared>,
    status: Arc<Mutex<Transport>>,
    lyrics: Arc<LyricsEngine>,

    stream: Option<cpal::Stream>,
    decoder: Option<DecoderState>,
    lookahead: Option<(DecoderState, QueueItem)>,
    resampler: Resampler,
    dsp: DspChain,
    equalizer: EqualizerSettings,

    queue: Vec<QueueItem>,
    index: usize,
    current: Option<TrackMeta>,

    playing: bool,
    finished: bool,
    /// Set when a seek is pending; applied before the next decode.
    pending_seek: Option<u64>,

    last_emit: Instant,
    last_position: u64,
}

impl Worker {
    fn new(
        ring: Arc<SpscRing>,
        shared: Arc<OutputShared>,
        status: Arc<Mutex<Transport>>,
        lyrics: Arc<LyricsEngine>,
    ) -> Self {
        Self {
            ring,
            shared,
            status,
            lyrics,
            stream: None,
            decoder: None,
            lookahead: None,
            resampler: Resampler::new(),
            dsp: DspChain::new(PIPELINE_RATE as f32),
            equalizer: EqualizerSettings::default(),
            queue: Vec::new(),
            index: 0,
            current: None,
            playing: false,
            finished: false,
            pending_seek: None,
            last_emit: Instant::now(),
            last_position: 0,
        }
    }

    fn run(&mut self, rx: Receiver<Command>) {
        if let Err(err) = self.ensure_stream() {
            log::error!("audio output unavailable: {err}");
        }

        loop {
            match rx.recv_timeout(Duration::from_millis(4)) {
                Ok(Command::Shutdown) => break,
                Ok(command) => {
                    if let Err(err) = self.handle_command(command) {
                        log::error!("audio command failed: {err}");
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }

            self.pump();

            if self.last_emit.elapsed() >= Duration::from_millis(200) {
                self.publish();
            }
        }

        self.shared.playing.store(false, Ordering::Release);
        self.stream = None;
    }

    fn ensure_stream(&mut self) -> AppResult<()> {
        if self.stream.is_some() {
            return Ok(());
        }
        #[cfg(target_os = "ios")]
        crate::platform_ios::audio_session::activate()?;

        let opened = output::open(&self.shared, PIPELINE_RATE)?;
        log::info!(
            "audio output: {} Hz, {} ch, {:?}",
            opened.sample_rate,
            opened.channels,
            opened.sample_format
        );
        self.resampler
            .configure(PIPELINE_RATE, opened.sample_rate.max(1));
        self.dsp
            .configure(&self.equalizer, opened.sample_rate as f32);
        self.stream = Some(opened.stream);
        Ok(())
    }

    fn handle_command(&mut self, command: Command) -> AppResult<()> {
        match command {
            Command::PlayFile {
                path,
                meta,
                autoplay,
            } => {
                self.load_file(path, meta, autoplay)?;
            }
            Command::Enqueue(item) => {
                self.queue.push(item.into_internal());
                self.sync_queue_meta();
            }
            Command::SetQueue(items) => {
                self.queue = items.into_iter().map(|i| i.into_internal()).collect();
                self.sync_queue_meta();
            }
            Command::ClearQueue => {
                self.queue.clear();
                self.index = 0;
                self.sync_queue_meta();
            }
            Command::Play => self.play()?,
            Command::Pause => self.pause(),
            Command::Next => {
                self.ring.reset();
                self.advance_queue(false)?;
                self.play()?;
            }
            Command::Previous => {
                // Match every other player: <3 s restarts the track, otherwise
                // jump to the previous queue entry.
                if self.audible_position() > 3_000 {
                    self.pending_seek = Some(0);
                    self.ring.reset();
                } else if self.index > 0 {
                    self.index -= 1;
                    let item = self.queue[self.index].clone();
                    self.load_file(item.path, item.meta, true)?;
                } else {
                    self.pending_seek = Some(0);
                    self.ring.reset();
                }
            }
            Command::Stop => {
                self.pause();
                self.decoder = None;
                self.lookahead = None;
                self.finished = false;
                self.ring.reset();
                self.set_position(0);
            }
            Command::Seek(ms) => {
                self.pending_seek = Some(ms);
                self.ring.reset();
            }
            Command::SetVolume(volume) => {
                self.shared.set_volume(volume);
                self.status.lock().volume = volume.clamp(0.0, 1.0);
            }
            Command::SetRate(rate) => {
                let rate = rate.clamp(0.25, 4.0);
                self.status.lock().rate = rate;
                #[cfg(target_os = "ios")]
                crate::platform_ios::now_playing::set_rate(rate as f64);
            }
            Command::SetEqualizer(settings) => {
                self.equalizer = settings;
                let rate = self.stream_rate();
                self.dsp.configure(&self.equalizer, rate);
            }
            Command::Snapshot(reply) => {
                let snapshot = self.build_snapshot();
                let _ = reply.send(snapshot);
            }
            Command::Shutdown => {}
        }
        Ok(())
    }

    fn stream_rate(&self) -> f32 {
        PIPELINE_RATE as f32
    }

    fn load_file(&mut self, path: PathBuf, meta: TrackMeta, autoplay: bool) -> AppResult<()> {
        self.ensure_stream()?;

        let decoder = DecoderState::open(&path)?;
        self.ring.reset();
        self.resampler.reset();
        self.dsp.reset();
        self.finished = false;
        self.pending_seek = None;
        self.lookahead = None;
        self.decoder = Some(decoder);
        self.current = Some(meta.clone());
        self.playing = autoplay;

        if autoplay {
            self.shared.playing.store(true, Ordering::Release);
        }

        {
            let mut status = self.status.lock();
            status.track = Some(meta.clone());
            status.duration_ms = meta.duration_ms;
            status.position_ms = 0;
            status.playing = autoplay;
            status.buffering = false;
            status.rate = 1.0;
        }

        self.last_position = 0;
        self.emit_track_changed(&meta);
        self.publish();

        #[cfg(target_os = "ios")]
        crate::platform_ios::now_playing::update(
            &meta.title,
            &meta.artist,
            meta.album.as_deref(),
            meta.duration_ms,
            0.0,
            if autoplay { 1.0 } else { 0.0 },
            meta.cover_path.as_deref(),
        );

        Ok(())
    }

    fn play(&mut self) -> AppResult<()> {
        self.ensure_stream()?;
        if self.decoder.is_none() && !self.queue.is_empty() {
            self.advance_queue(false)?;
        }
        self.playing = true;
        self.shared.playing.store(true, Ordering::Release);
        self.status.lock().playing = true;
        #[cfg(target_os = "ios")]
        crate::platform_ios::now_playing::set_playing(true);
        self.publish();
        Ok(())
    }

    fn pause(&mut self) {
        self.playing = false;
        self.shared.playing.store(false, Ordering::Release);
        self.status.lock().playing = false;
        #[cfg(target_os = "ios")]
        crate::platform_ios::now_playing::set_playing(false);
        self.publish();
    }

    /// Fill the ring while there is room. Returns after one pass so the
    /// command channel stays responsive.
    fn pump(&mut self) {
        if let Some(seek) = self.pending_seek.take() {
            if let Some(decoder) = self.decoder.as_mut() {
                if let Err(err) = decoder.seek_ms(seek) {
                    log::error!("seek failed: {err}");
                } else {
                    self.resampler.reset();
                    self.dsp.reset();
                    self.set_position(seek);
                }
            }
        }

        if !self.playing {
            return;
        }
        if self.decoder.is_none() {
            return;
        }

        // Drain up to ~8 blocks per pass; the ring supplies the backpressure.
        for _ in 0..8 {
            if self.ring.free() < 4096 * 2 {
                break;
            }

            // The decoder borrow must end before `push_block` takes `&mut self`.
            let step = {
                let Some(decoder) = self.decoder.as_mut() else {
                    return;
                };
                match decoder.next_block() {
                    Ok(Some(block)) => Some((block, decoder.sample_rate)),
                    Ok(None) => None,
                    Err(err) => {
                        log::error!("decode error: {err}");
                        None
                    }
                }
            };

            match step {
                Some((block, rate)) => {
                    if let Err(err) = self.push_block(block, rate) {
                        log::error!("push failed: {err}");
                        break;
                    }
                }
                None => {
                    self.on_end_of_stream();
                    return;
                }
            }
        }

        self.maybe_preload_next();
    }

    fn push_block(&mut self, block: Vec<f32>, input_rate: u32) -> AppResult<()> {
        self.resampler.configure(input_rate.max(1), PIPELINE_RATE);

        // Worst-case output size for a downsampling ratio of 8x is generous.
        let mut out = vec![0.0f32; block.len() * 8 + 8];
        let frames = self.resampler.process(&block, &mut out);
        out.truncate(frames * 2);

        self.dsp.process(&mut out);

        let mut offset = 0usize;
        while offset < out.len() {
            let written = self.ring.push(&out[offset..]);
            if written == 0 {
                // Ring full: wait for the render callback to consume.
                std::thread::sleep(Duration::from_millis(2));
                continue;
            }
            offset += written;
        }
        Ok(())
    }

    /// Open the next queue entry a few seconds early so the transition is
    /// sample-accurate (no silence, no re-open at the boundary).
    fn maybe_preload_next(&mut self) {
        if self.lookahead.is_some() {
            return;
        }
        let Some(decoder) = self.decoder.as_ref() else {
            return;
        };
        let duration = decoder.duration_ms;
        if duration == 0 || decoder.position_ms() + 4_000 < duration {
            return;
        }
        let Some(next) = self.queue.get(self.index + 1).cloned() else {
            return;
        };
        match DecoderState::open(&next.path) {
            Ok(next_decoder) => self.lookahead = Some((next_decoder, next)),
            Err(err) => log::warn!("preload failed for {}: {err}", next.path.display()),
        }
    }

    fn on_end_of_stream(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;

        // Let the ring drain so the tail of the track is actually heard.
        let started = Instant::now();
        while self.ring.available() > 0 && started.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(10));
        }

        if let Err(err) = self.advance_queue(true) {
            log::error!("queue advance failed: {err}");
            self.playing = false;
            self.shared.playing.store(false, Ordering::Release);
            self.status.lock().playing = false;
        }
    }

    /// Move to the next queue entry. With `from_end = true` the current track
    /// already finished, so a preloaded decoder can be swapped in directly.
    fn advance_queue(&mut self, from_end: bool) -> AppResult<()> {
        let next_index = self.index + 1;
        if next_index >= self.queue.len() {
            if from_end {
                self.emit_ended();
                self.playing = false;
                self.shared.playing.store(false, Ordering::Release);
                self.status.lock().playing = false;
                #[cfg(target_os = "ios")]
                crate::platform_ios::now_playing::set_playing(false);
            }
            return Ok(());
        }

        self.index = next_index;

        match self.lookahead.take() {
            Some((decoder, item)) if from_end => {
                self.ring.reset();
                self.resampler.reset();
                self.dsp.reset();
                self.decoder = Some(decoder);
                self.current = Some(item.meta.clone());
                self.finished = false;
                self.set_position(0);
                self.status.lock().track = Some(item.meta.clone());
                self.status.lock().duration_ms = item.meta.duration_ms;
                self.emit_track_changed(&item.meta);
                #[cfg(target_os = "ios")]
                crate::platform_ios::now_playing::update(
                    &item.meta.title,
                    &item.meta.artist,
                    item.meta.album.as_deref(),
                    item.meta.duration_ms,
                    0.0,
                    1.0,
                    item.meta.cover_path.as_deref(),
                );
            }
            _ => {
                let item = self.queue[next_index].clone();
                self.load_file(item.path, item.meta, true)?;
            }
        }

        self.sync_queue_meta();
        Ok(())
    }

    fn sync_queue_meta(&mut self) {
        let mut status = self.status.lock();
        status.queue_len = self.queue.len();
        status.queue_index = self.index;
    }

    fn set_position(&mut self, ms: u64) {
        self.last_position = ms;
        self.status.lock().position_ms = ms;
    }

    /// Audible position: decoded position minus whatever is still buffered.
    fn audible_position(&self) -> u64 {
        let Some(decoder) = self.decoder.as_ref() else {
            return self.last_position;
        };
        let buffered_ms = self.ring.available() / 2 * 1000 / PIPELINE_RATE as u64;
        decoder.position_ms().saturating_sub(buffered_ms)
    }

    fn publish(&mut self) {
        let position = self.audible_position();
        self.last_position = position;

        {
            let mut status = self.status.lock();
            status.position_ms = position;
            status.playing = self.playing;
            status.buffering = self.playing && self.ring.available() < 4096;
            if let Some(decoder) = self.decoder.as_ref() {
                if status.duration_ms == 0 {
                    status.duration_ms = decoder.duration_ms;
                }
            }
        }

        let snapshot = self.build_snapshot();

        if let Some(handle) = crate::state::current_handle() {
            let _ = handle.emit("audio:progress", &snapshot);
        }

        // Drive the lyric clock from the same sample-accurate position the UI
        // shows, so a drifting wall clock can never desync the two.
        self.lyrics.on_position(position);

        if let Some(handle) = crate::state::current_handle() {
            self.lyrics.flush_events(&handle);
        }

        #[cfg(target_os = "ios")]
        crate::platform_ios::now_playing::update_elapsed(
            position as f64 / 1000.0,
            if self.playing { 1.0 } else { 0.0 },
        );

        self.last_emit = Instant::now();
    }

    fn build_snapshot(&self) -> Snapshot {
        let transport = self.status.lock().clone();
        Snapshot {
            track_id: transport.track.as_ref().map(|t| t.id.clone()),
            title: transport.track.as_ref().map(|t| t.title.clone()),
            artist: transport.track.as_ref().map(|t| t.artist.clone()),
            album: transport.track.as_ref().and_then(|t| t.album.clone()),
            cover_path: transport.track.as_ref().and_then(|t| t.cover_path.clone()),
            position_ms: transport.position_ms,
            duration_ms: transport.duration_ms,
            playing: transport.playing,
            buffering: transport.buffering,
            volume: transport.volume,
            rate: transport.rate,
            queue_len: self.queue.len(),
            queue_index: self.index,
            underruns: self.shared.underruns.load(Ordering::Relaxed),
        }
    }

    fn emit_track_changed(&self, meta: &TrackMeta) {
        if let Some(handle) = crate::state::current_handle() {
            let _ = handle.emit("audio:track-changed", meta);
        }
    }

    fn emit_ended(&self) {
        if let Some(handle) = crate::state::current_handle() {
            let _ = handle.emit("audio:ended", ());
        }
    }

    /// Drain the lock-screen / headphone-button queue.
    ///
    /// These arrive on a main-thread ObjC callback that only stores an atom; the
    /// audio thread translates them into the same `Command` the UI uses, so
    /// there is exactly one state machine.
    #[cfg(target_os = "ios")]
    fn poll_remote(&mut self) {
        while let Some(code) = crate::platform_ios::take_remote_command() {
            let command = match code {
                0 => Command::Play,
                1 => Command::Pause,
                2 => {
                    if self.playing {
                        Command::Pause
                    } else {
                        Command::Play
                    }
                }
                3 => Command::Next,
                4 => Command::Previous,
                5 => Command::Seek(self.audible_position() + 15_000),
                6 => Command::Seek(self.audible_position().saturating_sub(15_000)),
                _ => continue,
            };
            if let Err(err) = self.handle_command(command) {
                log::error!("remote command failed: {err}");
            }
        }

        if let Some(seconds) = crate::platform_ios::take_remote_seek() {
            let ms = (seconds.max(0.0) * 1000.0) as u64;
            if let Err(err) = self.handle_command(Command::Seek(ms)) {
                log::error!("remote seek failed: {err}");
            }
        }
    }
}

/// Dispatch used by the iOS remote-command bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteCommand {
    Play,
    Pause,
    Toggle,
    Next,
    Previous,
    SeekForward,
    SeekBackward,
}

pub fn dispatch_remote(engine: &AudioEngine, command: RemoteCommand) {
    let _ = match command {
        RemoteCommand::Play => engine.send(Command::Play),
        RemoteCommand::Pause => engine.send(Command::Pause),
        RemoteCommand::Toggle => {
            if engine.is_playing() {
                engine.send(Command::Pause)
            } else {
                engine.send(Command::Play)
            }
        }
        RemoteCommand::Next => engine.send(Command::Next),
        RemoteCommand::Previous => engine.send(Command::Previous),
        RemoteCommand::SeekForward => {
            let snapshot = engine.snapshot();
            engine.send(Command::Seek(snapshot.position_ms + 15_000))
        }
        RemoteCommand::SeekBackward => {
            let snapshot = engine.snapshot();
            engine.send(Command::Seek(snapshot.position_ms.saturating_sub(15_000)))
        }
    };
}
