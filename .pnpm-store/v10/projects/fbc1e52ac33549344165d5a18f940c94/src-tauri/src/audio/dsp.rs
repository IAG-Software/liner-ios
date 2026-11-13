//! DSP chain: pre-amp, 10-band peaking equaliser, soft limiter.
//!
//! Runs on the decoder thread (never on the render thread) so the CoreAudio
//! callback stays a memcpy. Bands use RBJ peaking biquads in transposed direct
//! form II, which is the numerically stable variant for float audio.

use serde::Deserialize;

/// Frequencies mirror the desktop equaliser presets (Hz).
pub const DEFAULT_BANDS: [f32; 10] = [
    32.0, 64.0, 125.0, 250.0, 500.0, 1_000.0, 2_000.0, 4_000.0, 8_000.0, 16_000.0,
];

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EqualizerSettings {
    #[serde(default)]
    pub enabled: bool,
    /// Per-band gain in dB; shorter arrays are padded with 0 dB.
    #[serde(default)]
    pub gains: Vec<f32>,
    #[serde(default)]
    pub preamp_db: f32,
}

impl Default for EqualizerSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            gains: vec![0.0; DEFAULT_BANDS.len()],
            preamp_db: 0.0,
        }
    }
}

#[derive(Clone, Copy)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn peaking(freq: f32, sample_rate: f32, gain_db: f32, q: f32) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * (freq / sample_rate).clamp(1e-6, 0.49);
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    #[inline]
    fn tick(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }

    fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

pub struct DspChain {
    bands: [Biquad; 10],
    left: [Biquad; 10],
    right: [Biquad; 10],
    preamp: f32,
    sample_rate: f32,
    enabled: bool,
}

impl DspChain {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            bands: [Biquad::identity(); 10],
            left: [Biquad::identity(); 10],
            right: [Biquad::identity(); 10],
            preamp: 1.0,
            sample_rate,
            enabled: false,
        }
    }

    pub fn configure(&mut self, settings: &EqualizerSettings, sample_rate: f32) {
        self.sample_rate = sample_rate;
        self.enabled = settings.enabled;
        self.preamp = 10f32.powf(settings.preamp_db / 20.0);

        for (index, freq) in DEFAULT_BANDS.iter().enumerate() {
            let gain = settings.gains.get(index).copied().unwrap_or(0.0);
            // Q ≈ 1.41 gives roughly one-octave bands at these centres.
            let band = Biquad::peaking(*freq, sample_rate, gain, 1.41);
            self.bands[index] = band;
            self.left[index] = band;
            self.right[index] = band;
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        if (self.sample_rate - sample_rate).abs() > f32::EPSILON {
            let settings = EqualizerSettings {
                enabled: self.enabled,
                gains: self
                    .bands
                    .iter()
                    .map(|_| 0.0)
                    .collect::<Vec<_>>(),
                preamp_db: 20.0 * self.preamp.log10(),
            };
            self.configure(&settings, sample_rate);
        }
    }

    pub fn reset(&mut self) {
        for band in &mut self.left {
            band.reset();
        }
        for band in &mut self.right {
            band.reset();
        }
    }

    /// Process interleaved stereo in place.
    pub fn process(&mut self, buffer: &mut [f32]) {
        if !self.enabled && (self.preamp - 1.0).abs() < f32::EPSILON {
            return;
        }

        for frame in buffer.chunks_exact_mut(2) {
            let mut l = frame[0] * self.preamp;
            let mut r = frame[1] * self.preamp;

            if self.enabled {
                for band in &mut self.left {
                    l = band.tick(l);
                }
                for band in &mut self.right {
                    r = band.tick(r);
                }
            }

            // Soft-knee limiter: keeps boosted bands from clipping the
            // AudioUnit without the pumping of a hard limiter.
            frame[0] = soft_limit(l);
            frame[1] = soft_limit(r);
        }
    }
}

#[inline]
fn soft_limit(x: f32) -> f32 {
    const THRESHOLD: f32 = 0.94;
    if x.abs() <= THRESHOLD {
        x
    } else {
        let sign = x.signum();
        let over = x.abs() - THRESHOLD;
        sign * (THRESHOLD + (1.0 - THRESHOLD) * (over / (over + (1.0 - THRESHOLD))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_chain_is_transparent() {
        let mut chain = DspChain::new(48_000.0);
        chain.configure(&EqualizerSettings::default(), 48_000.0);
        let mut buffer = vec![0.25f32; 64];
        let before = buffer.clone();
        chain.process(&mut buffer);
        assert_eq!(buffer, before);
    }

    #[test]
    fn boost_raises_energy() {
        let mut chain = DspChain::new(48_000.0);
        chain.configure(
            &EqualizerSettings {
                enabled: true,
                gains: vec![6.0; 10],
                preamp_db: 0.0,
            },
            48_000.0,
        );
        let mut buffer = vec![0.1f32; 4096];
        chain.process(&mut buffer);
        let last = buffer[buffer.len() - 2].abs();
        assert!(last > 0.1, "expected boost, got {last}");
    }

    #[test]
    fn limiter_bounds_output() {
        for x in [-4.0f32, -1.0, 0.0, 1.0, 4.0] {
            assert!(soft_limit(x).abs() <= 1.0);
        }
    }
}
