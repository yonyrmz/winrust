//! A lock free-ish single producer / single consumer ring buffer.
//!
//! The decoder thread pushes PCM frames into the buffer while the `cpal`
//! callback pops them. The implementation uses a `parking_lot` mutex around a
//! `VecDeque`, which is more than fast enough for the buffer sizes used here
//! and keeps the code simple and correct.

use std::collections::VecDeque;

use parking_lot::Mutex;

/// A bounded FIFO queue of `f32` samples.
#[derive(Debug)]
pub struct RingBuffer {
    inner: Mutex<Inner>,
    capacity: usize,
}

#[derive(Debug)]
struct Inner {
    data: VecDeque<f32>,
}

impl RingBuffer {
    /// Creates a ring buffer able to hold `capacity` samples.
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(Inner {
                data: VecDeque::with_capacity(capacity.min(1 << 20)),
            }),
            capacity: capacity.max(1),
        }
    }

    /// Maximum number of samples the buffer can hold.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Pushes samples, dropping the oldest ones when the buffer is full.
    pub fn push(&self, samples: &[f32]) {
        if samples.is_empty() {
            return;
        }
        let mut guard = self.inner.lock();
        let inner = &mut *guard;

        let total = inner.data.len() + samples.len();
        if total <= self.capacity {
            inner.data.extend(samples.iter().copied());
            return;
        }

        // The buffer is full: drop the oldest samples, first the ones that
        // are already buffered and then the oldest of the new batch.
        let excess = total - self.capacity;
        let drop_buffered = excess.min(inner.data.len());
        inner.data.drain(0..drop_buffered);

        let skip = (excess - drop_buffered).min(samples.len());
        inner.data.extend(samples[skip..].iter().copied());
    }

    /// Pops up to `out.len()` samples into `out`, returning how many were read.
    pub fn pop(&self, out: &mut [f32]) -> usize {
        let mut guard = self.inner.lock();
        let inner = &mut *guard;
        let count = out.len().min(inner.data.len());
        for (index, slot) in out.iter_mut().take(count).enumerate() {
            *slot = inner.data[index];
        }
        inner.data.drain(0..count);
        count
    }

    /// Number of samples currently buffered.
    pub fn len(&self) -> usize {
        self.inner.lock().data.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Drops every buffered sample.
    pub fn clear(&self) {
        self.inner.lock().data.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pushes_and_pops_in_order() {
        let buffer = RingBuffer::new(16);
        buffer.push(&[1.0, 2.0, 3.0]);
        let mut out = [0.0; 4];
        assert_eq!(buffer.pop(&mut out), 3);
        assert_eq!(&out[..3], &[1.0, 2.0, 3.0]);
        assert_eq!(out[3], 0.0);
    }

    #[test]
    fn drops_oldest_when_full() {
        let buffer = RingBuffer::new(4);
        buffer.push(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let mut out = [0.0; 4];
        assert_eq!(buffer.pop(&mut out), 4);
        assert_eq!(out, [2.0, 3.0, 4.0, 5.0]);
    }
}
