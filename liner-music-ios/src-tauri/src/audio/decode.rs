//! Symphonia-backed file decoder.
//!
//! Decoding is intentionally file-based rather than network-based: the cache
//! manager materialises the media first, which gives us free seeking, exact
//! duration, and no rebuffering stalls while the app is backgrounded (iOS can
//! suspend a socket at any moment but never a local `read`).

use std::fs::File;
use std::path::{Path, PathBuf};

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

use crate::error::{AppError, AppResult};

pub struct DecoderState {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    pub sample_rate: u32,
    pub channels: usize,
    pub duration_ms: u64,
    /// Frames decoded since `origin_frame`, used for the transport clock.
    frames_decoded: u64,
    origin_frame: u64,
    sample_buf: Option<SampleBuffer<f32>>,
    finished: bool,
    path: PathBuf,
}

impl DecoderState {
    pub fn open(path: &Path) -> AppResult<Self> {
        let file = File::open(path)?;
        let stream = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                stream,
                &FormatOptions {
                    enable_gapless: true,
                    ..Default::default()
                },
                &MetadataOptions::default(),
            )
            .map_err(|e| AppError::Decode(format!("probe failed: {e}")))?;

        let reader = probed.format;
        let track = reader
            .default_track()
            .ok_or_else(|| AppError::Decode("no default audio track".into()))?;

        if track.codec_params.codec == CODEC_TYPE_NULL {
            return Err(AppError::Decode("unsupported codec".into()));
        }

        let track_id = track.id;
        let sample_rate = track.codec_params.sample_rate.unwrap_or(44_100);
        let channels = track
            .codec_params
            .channels
            .map(|c| c.count())
            .unwrap_or(2)
            .clamp(1, 2);

        let duration_ms = track
            .codec_params
            .n_frames
            .map(|frames| {
                if sample_rate == 0 {
                    0
                } else {
                    frames.saturating_mul(1000) / sample_rate as u64
                }
            })
            .unwrap_or(0);

        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| AppError::Decode(format!("decoder init failed: {e}")))?;

        Ok(Self {
            reader,
            decoder,
            track_id,
            sample_rate,
            channels,
            duration_ms,
            frames_decoded: 0,
            origin_frame: 0,
            sample_buf: None,
            finished: false,
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn position_ms(&self) -> u64 {
        if self.sample_rate == 0 {
            return 0;
        }
        (self.origin_frame + self.frames_decoded) * 1000 / self.sample_rate as u64
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Decode the next packet into interleaved stereo `f32`.
    ///
    /// Returns `Ok(None)` at end-of-stream. Mono sources are duplicated to
    /// stereo so downstream code only ever handles one layout.
    pub fn next_block(&mut self) -> AppResult<Option<Vec<f32>>> {
        if self.finished {
            return Ok(None);
        }

        loop {
            let packet = match self.reader.next_packet() {
                Ok(packet) => packet,
                Err(SymphoniaError::IoError(err))
                    if err.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    self.finished = true;
                    return Ok(None);
                }
                Err(SymphoniaError::ResetRequired) => {
                    self.finished = true;
                    return Ok(None);
                }
                Err(err) => return Err(AppError::Decode(err.to_string())),
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            let decoded = match self.decoder.decode(&packet) {
                Ok(decoded) => decoded,
                // A corrupt packet mid-file should not kill playback.
                Err(SymphoniaError::DecodeError(err)) => {
                    log::warn!("skipping undecodable packet: {err}");
                    continue;
                }
                Err(err) => return Err(AppError::Decode(err.to_string())),
            };

            let spec = *decoded.spec();
            self.sample_rate = spec.rate;
            self.channels = spec.channels.count();

            let capacity = decoded.capacity() as u64;
            let buffer = self.sample_buf.get_or_insert_with(|| {
                SampleBuffer::<f32>::new(capacity, spec)
            });

            // The capacity can grow between packets on some containers.
            if buffer.capacity() < capacity as usize {
                *buffer = SampleBuffer::<f32>::new(capacity, spec);
            }

            buffer.copy_interleaved_ref(decoded);
            let samples = buffer.samples();
            let frames = samples.len() / spec.channels.count().max(1);
            self.frames_decoded += frames as u64;

            let interleaved = to_stereo(samples, spec.channels.count());
            return Ok(Some(interleaved));
        }
    }

    pub fn seek_ms(&mut self, position_ms: u64) -> AppResult<()> {
        let time = Time::from(position_ms as f64 / 1000.0);
        self.reader
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time,
                    track_id: Some(self.track_id),
                },
            )
            .map_err(|e| AppError::Decode(format!("seek failed: {e}")))?;

        self.origin_frame = position_ms.saturating_mul(self.sample_rate as u64) / 1000;
        self.frames_decoded = 0;
        self.finished = false;
        // The codec keeps inter-frame state; a flush prevents pre-roll artefacts
        // after a seek.
        let _ = self.decoder.finalize();
        Ok(())
    }
}

fn to_stereo(samples: &[f32], channels: usize) -> Vec<f32> {
    match channels {
        1 => samples.iter().flat_map(|s| [*s, *s]).collect(),
        2 => samples.to_vec(),
        n => {
            // Down-mix surround to stereo by taking L/R and folding the rest in.
            let frames = samples.len() / n;
            let mut out = Vec::with_capacity(frames * 2);
            for frame in 0..frames {
                let base = frame * n;
                let mut left = samples[base];
                let mut right = samples[base + 1];
                for ch in 2..n {
                    let extra = samples[base + ch] * 0.5;
                    left += extra;
                    right += extra;
                }
                out.push(left);
                out.push(right);
            }
            out
        }
    }
}
