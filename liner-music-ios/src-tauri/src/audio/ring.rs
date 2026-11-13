//! Single-producer / single-consumer sample ring.
//!
//! The decoder thread pushes interleaved stereo `f32`; the CoreAudio render
//! thread pops. Neither side may block, so the indices are plain atomics and
//! the payload area is treated as `MaybeUninit`-equivalent (`f32` has no
//! invalid bit patterns, and every slot is written before the producer
//! publishes it through the release-store on `write`).

use std::sync::atomic::{AtomicU64, Ordering};

pub struct SpscRing {
    buf: Box<[f32]>,
    /// Capacity in samples (frames * 2).
    cap: u64,
    write: AtomicU64,
    read: AtomicU64,
}

// SAFETY: the ring is written by exactly one thread and read by exactly one
// thread; the atomic indices establish the happens-before edges.
unsafe impl Send for SpscRing {}
unsafe impl Sync for SpscRing {}

impl SpscRing {
    /// `frames` is stereo frames; actual capacity is `frames * 2` samples.
    pub fn new(frames: usize) -> Self {
        let cap = (frames.max(1) * 2) as u64;
        Self {
            buf: vec![0.0; cap as usize].into_boxed_slice(),
            cap,
            write: AtomicU64::new(0),
            read: AtomicU64::new(0),
        }
    }

    pub fn capacity_samples(&self) -> u64 {
        self.cap
    }

    pub fn available(&self) -> u64 {
        let write = self.write.load(Ordering::Acquire);
        let read = self.read.load(Ordering::Acquire);
        write.saturating_sub(read)
    }

    pub fn free(&self) -> u64 {
        self.cap.saturating_sub(self.available())
    }

    /// Push as many samples as fit; returns how many were written.
    pub fn push(&self, samples: &[f32]) -> usize {
        let write = self.write.load(Ordering::Relaxed);
        let read = self.read.load(Ordering::Acquire);
        let free = self.cap - write.saturating_sub(read);
        let n = (samples.len() as u64).min(free) as usize;
        if n == 0 {
            return 0;
        }

        let start = (write % self.cap) as usize;
        let first = n.min((self.cap as usize) - start);
        // SAFETY: slots [start, start+first) are free (free-space accounting
        // above) and this is the only producer.
        unsafe {
            let dst = self.buf.as_ptr().add(start) as *mut f32;
            std::ptr::copy_nonoverlapping(samples.as_ptr(), dst, first);
            if n > first {
                let dst2 = self.buf.as_ptr() as *mut f32;
                std::ptr::copy_nonoverlapping(samples.as_ptr().add(first), dst2, n - first);
            }
        }

        self.write.store(write + n as u64, Ordering::Release);
        n
    }

    /// Pop up to `out.len()` samples; returns the number filled. Remaining
    /// samples are left untouched so the caller can append silence.
    pub fn pop(&self, out: &mut [f32]) -> usize {
        let read = self.read.load(Ordering::Relaxed);
        let write = self.write.load(Ordering::Acquire);
        let avail = write.saturating_sub(read);
        let n = (out.len() as u64).min(avail) as usize;
        if n == 0 {
            return 0;
        }

        let start = (read % self.cap) as usize;
        let first = n.min((self.cap as usize) - start);
        // SAFETY: slots [read, read+n) were published by the producer.
        unsafe {
            let src = self.buf.as_ptr().add(start);
            std::ptr::copy_nonoverlapping(src, out.as_mut_ptr(), first);
            if n > first {
                let src2 = self.buf.as_ptr();
                std::ptr::copy_nonoverlapping(src2, out.as_mut_ptr().add(first), n - first);
            }
        }

        self.read.store(read + n as u64, Ordering::Release);
        n
    }

    /// Drop everything currently buffered (used on seek / track change).
    pub fn reset(&self) {
        let write = self.write.load(Ordering::Acquire);
        self.read.store(write, Ordering::Release);
    }
}
