//! Native audio subsystem.

pub mod decode;
pub mod dsp;
pub mod output;
pub mod resample;
pub mod ring;

/// Fixed pipeline rate.
///
/// One stream for the whole app lifetime keeps queue transitions gapless and
/// avoids tearing down the `AudioUnit` (which would deactivate the audio
/// session and, on iOS, drop lock-screen state). 48 kHz is what every iPhone
/// output route runs at, so the resampler is a no-op on native-48k content.
pub const PIPELINE_RATE: u32 = 48_000;

/// Ring size: ~1.5 s of stereo audio at 48 kHz. Deep enough to absorb a
/// jetsam stall or a slow decode packet, shallow enough that a seek feels
/// immediate.
pub const RING_FRAMES: usize = 72_000;
