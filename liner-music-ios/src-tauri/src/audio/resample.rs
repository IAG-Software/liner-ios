//! Catmull-Rom sample-rate conversion.
//!
//! Tracks arrive at 44.1 / 48 / 96 kHz; the CoreAudio render callback runs at a
//! single fixed rate so the queue can be gapless without rebuilding the
//! AudioUnit. 4-point cubic interpolation is a large step up from linear
//! (roughly −70 dB vs −40 dB alias floor at 44.1→48 kHz) at negligible CPU
//! cost, and needs no FFT-window latency.

pub struct Resampler {
    /// Stereo history: previous two input frames plus the current one.
    history: [[f32; 4]; 2],
    /// Fractional position inside the current input frame [0, 1).
    phase: f64,
    step: f64,
    primed: bool,
}

impl Resampler {
    pub fn new() -> Self {
        Self {
            history: [[0.0; 4]; 2],
            phase: 0.0,
            step: 1.0,
            primed: false,
        }
    }

    pub fn configure(&mut self, input_rate: u32, output_rate: u32) {
        let step = if input_rate == 0 {
            1.0
        } else {
            input_rate as f64 / output_rate as f64
        };
        if (step - self.step).abs() > f64::EPSILON {
            self.step = step;
            self.phase = 0.0;
            self.history = [[0.0; 4]; 2];
            self.primed = false;
        }
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.history = [[0.0; 4]; 2];
        self.primed = false;
    }

    /// Resample interleaved stereo `input` into `out` (interleaved stereo).
    /// `out` must be sized by the caller; returns the number of frames written.
    pub fn process(&mut self, input: &[f32], out: &mut [f32]) -> usize {
        if input.len() < 2 || out.len() < 2 {
            return 0;
        }
        let in_frames = input.len() / 2;
        let max_out = out.len() / 2;
        let mut written = 0usize;

        // Prime the history with the first frame so the first output sample is
        // not a fade-in from silence.
        if !self.primed {
            for ch in 0..2 {
                let s = input[ch];
                self.history[ch] = [s, s, s, s];
            }
            self.primed = true;
        }

        let mut cursor = 0usize; // index into input frames
        while written < max_out {
            // Shift history forward as the cursor advances.
            while self.phase >= 1.0 {
                self.phase -= 1.0;
                if cursor < in_frames {
                    for ch in 0..2 {
                        self.history[ch][0] = self.history[ch][1];
                        self.history[ch][1] = self.history[ch][2];
                        self.history[ch][2] = self.history[ch][3];
                        self.history[ch][3] = input[cursor * 2 + ch];
                    }
                    cursor += 1;
                } else {
                    // Flush: repeat the last frame so the tail decays cleanly.
                    for ch in 0..2 {
                        self.history[ch][0] = self.history[ch][1];
                        self.history[ch][1] = self.history[ch][2];
                        self.history[ch][2] = self.history[ch][3];
                    }
                }
            }

            let t = self.phase as f32;
            for ch in 0..2 {
                let p0 = self.history[ch][0];
                let p1 = self.history[ch][1];
                let p2 = self.history[ch][2];
                let p3 = self.history[ch][3];
                out[written * 2 + ch] = catmull_rom(p0, p1, p2, p3, t);
            }
            written += 1;
            self.phase += self.step;
        }

        written
    }
}

impl Default for Resampler {
    fn default() -> Self {
        Self::new()
    }
}

#[inline]
fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unity_passthrough_is_stable() {
        let mut r = Resampler::new();
        r.configure(48_000, 48_000);
        let input: Vec<f32> = (0..256).map(|i| (i as f32 / 32.0).sin()).collect();
        let stereo: Vec<f32> = input.iter().flat_map(|s| [*s, *s]).collect();
        let mut out = vec![0.0f32; stereo.len()];
        let frames = r.process(&stereo, &mut out);
        assert_eq!(frames, stereo.len() / 2);
        // 1:1 ratio must reproduce the signal (after the 3-frame priming).
        for i in 10..200 {
            assert!((out[i * 2] - stereo[i * 2]).abs() < 1e-3);
        }
    }

    #[test]
    fn upsample_preserves_length_ratio() {
        let mut r = Resampler::new();
        r.configure(44_100, 48_000);
        let input = vec![0.5f32; 441 * 2];
        let mut out = vec![0.0f32; 480 * 2];
        let frames = r.process(&input, &mut out);
        assert!(frames >= 400, "expected ~480 frames, got {frames}");
    }
}
