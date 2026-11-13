//! CoreAudio output via `cpal`.
//!
//! On iOS `cpal` talks to an `AudioUnit` (`kAudioUnitSubType_RemoteIO`), which
//! is the only output that keeps rendering while the app is backgrounded — and
//! only if `AVAudioSession` is active with the `playback` category (see
//! `platform_ios::audio_session`).
//!
//! The stream is created on the audio worker thread and never moved off it:
//! `cpal::Stream` is `!Send` on Apple platforms because the render callback
//! captures a non-thread-safe `AudioUnit` reference.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate, StreamConfig};

use super::ring::SpscRing;
use crate::error::{AppError, AppResult};

/// Everything the render callback needs. Kept small: the callback must never
/// allocate or lock.
pub struct OutputShared {
    pub ring: Arc<SpscRing>,
    /// Transport state; `false` means output silence (paused/stopped).
    pub playing: Arc<AtomicBool>,
    /// Linear volume, stored as `f32` bits.
    pub volume: Arc<AtomicU32>,
    /// Set by the callback when the ring runs dry while playing.
    pub underruns: Arc<AtomicU32>,
}

impl OutputShared {
    pub fn new(ring: Arc<SpscRing>) -> Self {
        Self {
            ring,
            playing: Arc::new(AtomicBool::new(false)),
            volume: Arc::new(AtomicU32::new(1.0f32.to_bits())),
            underruns: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn set_volume(&self, volume: f32) {
        self.volume
            .store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }
}

pub struct OpenedOutput {
    pub stream: cpal::Stream,
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_format: SampleFormat,
}

/// Open the default output device at `preferred_rate` when supported, else at
/// the device default. Returns the real rate so the decoder can resample to it.
pub fn open(shared: &OutputShared, preferred_rate: u32) -> AppResult<OpenedOutput> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| AppError::Audio("no default output device".into()))?;

    let supported: Vec<_> = device
        .supported_output_configs()
        .map_err(|e| AppError::Audio(format!("enumerate output configs: {e}")))?
        .collect();

    // Prefer f32 stereo at the preferred rate, then f32 stereo at anything,
    // then whatever the device defaults to.
    let pick = supported
        .iter()
        .find(|c| {
            c.sample_format() == SampleFormat::F32
                && c.channels() == 2
                && c.min_sample_rate().0 <= preferred_rate
                && c.max_sample_rate().0 >= preferred_rate
        })
        .or_else(|| {
            supported
                .iter()
                .find(|c| c.sample_format() == SampleFormat::F32 && c.channels() == 2)
        })
        .or_else(|| supported.iter().find(|c| c.channels() == 2))
        .or_else(|| supported.first());

    let (config, sample_format) = match pick {
        Some(range) => {
            let rate = SampleRate(preferred_rate.clamp(
                range.min_sample_rate().0,
                range.max_sample_rate().0,
            ));
            (
                StreamConfig {
                    channels: range.channels(),
                    sample_rate: rate,
                    buffer_size: cpal::BufferSize::Default,
                },
                range.sample_format(),
            )
        }
        None => {
            let default = device
                .default_output_config()
                .map_err(|e| AppError::Audio(format!("default output config: {e}")))?;
            (
                StreamConfig {
                    channels: default.channels(),
                    sample_rate: default.sample_rate(),
                    buffer_size: cpal::BufferSize::Default,
                },
                default.sample_format(),
            )
        }
    };

    let err_fn = |err| log::error!("audio output error: {err}");

    let stream = match sample_format {
        SampleFormat::F32 => {
            let shared = clone_shared(shared);
            device
                .build_output_stream(
                    &config,
                    move |data: &mut [f32], _| render_f32(data, &shared),
                    err_fn,
                    None,
                )
                .map_err(|e| AppError::Audio(format!("build f32 stream: {e}")))?
        }
        SampleFormat::I16 => {
            let shared = clone_shared(shared);
            device
                .build_output_stream(
                    &config,
                    move |data: &mut [i16], _| render_i16(data, &shared),
                    err_fn,
                    None,
                )
                .map_err(|e| AppError::Audio(format!("build i16 stream: {e}")))?
        }
        other => {
            return Err(AppError::Audio(format!(
                "unsupported output sample format {other:?}"
            )))
        }
    };

    stream
        .play()
        .map_err(|e| AppError::Audio(format!("start stream: {e}")))?;

    Ok(OpenedOutput {
        stream,
        sample_rate: config.sample_rate.0,
        channels: config.channels,
        sample_format,
    })
}

fn clone_shared(shared: &OutputShared) -> OutputShared {
    OutputShared {
        ring: shared.ring.clone(),
        playing: shared.playing.clone(),
        volume: shared.volume.clone(),
        underruns: shared.underruns.clone(),
    }
}

/// Per-callback volume smoothing state, kept as thread-local because the
/// callback is called on a single render thread.
fn render_f32(data: &mut [f32], shared: &OutputShared) {
    let playing = shared.playing.load(Ordering::Acquire);

    if !playing {
        data.fill(0.0);
        return;
    }

    let filled = shared.ring.pop(data);
    if filled < data.len() {
        data[filled..].fill(0.0);
        if filled < data.len() / 2 {
            shared.underruns.fetch_add(1, Ordering::Relaxed);
        }
    }

    apply_volume(data, shared);
}

fn render_i16(data: &mut [i16], shared: &OutputShared) {
    let playing = shared.playing.load(Ordering::Acquire);
    if !playing {
        data.fill(0);
        return;
    }

    // Convert through a small scratch buffer allocated once per callback size.
    let mut scratch: Vec<f32> = vec![0.0; data.len()];
    let filled = shared.ring.pop(&mut scratch);
    if filled < scratch.len() {
        scratch[filled..].fill(0.0);
        if filled < scratch.len() / 2 {
            shared.underruns.fetch_add(1, Ordering::Relaxed);
        }
    }

    apply_volume(&mut scratch, shared);
    for (dst, src) in data.iter_mut().zip(scratch.iter()) {
        *dst = (src.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
    }
}

/// Ramp toward the target gain over one buffer to avoid zipper noise.
fn apply_volume(data: &mut [f32], shared: &OutputShared) {
    let target = f32::from_bits(shared.volume.load(Ordering::Relaxed));
    if (target - 1.0).abs() < f32::EPSILON {
        return;
    }
    // A linear ramp across the buffer is inaudible at 5–10 ms buffer sizes.
    let step = target / data.len().max(1) as f32;
    let mut gain = target - step * data.len() as f32;
    for sample in data.iter_mut() {
        gain += step;
        *sample *= gain;
    }
}
