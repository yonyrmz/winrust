//! Sample rate conversion helpers built on top of `rubato`.

use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Async, FixedAsync, Indexing, PolynomialDegree, Resampler as RubatoResampler};

/// A chunk based sample rate converter.
///
/// The decoder produces packets of arbitrary size, while the resampler works
/// on fixed size chunks, so decoded samples are buffered until a full chunk
/// is available.
pub struct SampleRateConverter {
    inner: Option<Async<f32>>,
    channels: usize,
    chunk_size: usize,
    pending: Vec<f32>,
    input_buffer: Vec<f32>,
    output_buffer: Vec<f32>,
    input_frames: u64,
    output_frames: u64,
    /// True when the converter is a pass-through.
    passthrough: bool,
}

impl SampleRateConverter {
    /// Creates a converter from `input_rate` to `output_rate`.
    ///
    /// When both rates match, a pass-through converter is returned.
    pub fn new(input_rate: u32, output_rate: u32, channels: usize) -> Result<Self, String> {
        let channels = channels.max(1);
        let chunk_size = 1024;

        if input_rate == output_rate || input_rate == 0 || output_rate == 0 {
            return Ok(Self {
                inner: None,
                channels,
                chunk_size,
                pending: Vec::new(),
                input_buffer: Vec::new(),
                output_buffer: Vec::new(),
                input_frames: 0,
                output_frames: 0,
                passthrough: true,
            });
        }

        let ratio = output_rate as f64 / input_rate as f64;
        let inner = Async::<f32>::new_poly(
            ratio,
            1.1,
            PolynomialDegree::Cubic,
            chunk_size,
            channels,
            FixedAsync::Input,
        )
        .map_err(|err| format!("failed to create resampler: {err}"))?;

        let output_capacity = inner.output_frames_max();

        Ok(Self {
            inner: Some(inner),
            channels,
            chunk_size,
            pending: Vec::new(),
            input_buffer: vec![0.0; chunk_size * channels],
            output_buffer: vec![0.0; output_capacity * channels],
            input_frames: 0,
            output_frames: 0,
            passthrough: false,
        })
    }

    /// Whether the converter simply forwards samples untouched.
    pub fn is_passthrough(&self) -> bool {
        self.passthrough
    }

    /// Feeds interleaved samples and appends the converted result to `out`.
    pub fn process(&mut self, samples: &[f32], out: &mut Vec<f32>) -> Result<(), String> {
        if samples.is_empty() {
            return Ok(());
        }

        if self.passthrough {
            out.extend_from_slice(samples);
            self.input_frames += (samples.len() / self.channels) as u64;
            self.output_frames = self.input_frames;
            return Ok(());
        }

        self.pending.extend_from_slice(samples);
        let chunk_samples = self.chunk_size * self.channels;

        while self.pending.len() >= chunk_samples {
            let (head, tail) = self.pending.split_at(chunk_samples);
            self.input_buffer.copy_from_slice(head);
            self.pending = tail.to_vec();
            self.convert_chunk(out, self.chunk_size)?;
        }

        Ok(())
    }

    /// Flushes the remaining samples, padding the tail with silence.
    pub fn flush(&mut self, out: &mut Vec<f32>) -> Result<(), String> {
        if self.passthrough || self.pending.is_empty() {
            return Ok(());
        }

        let chunk_samples = self.chunk_size * self.channels;
        self.input_buffer.fill(0.0);
        let count = self.pending.len().min(chunk_samples);
        self.input_buffer[..count].copy_from_slice(&self.pending[..count]);
        self.pending.clear();
        self.convert_chunk(out, count / self.channels)?;
        Ok(())
    }

    /// Runs one conversion chunk.
    fn convert_chunk(&mut self, out: &mut Vec<f32>, input_frames: usize) -> Result<(), String> {
        let inner = self
            .inner
            .as_mut()
            .expect("converter is not a pass-through");

        let indexing = Indexing::new().partial_len(input_frames);
        let input_adapter = InterleavedSlice::new(&self.input_buffer, self.channels, input_frames)
            .map_err(|err| format!("invalid input adapter: {err}"))?;

        let out_capacity = self.output_buffer.len() / self.channels;
        let mut output_adapter =
            InterleavedSlice::new_mut(&mut self.output_buffer, self.channels, out_capacity)
                .map_err(|err| format!("invalid output adapter: {err}"))?;

        let (_read, written) = inner
            .process_into_buffer(&input_adapter, &mut output_adapter, Some(&indexing))
            .map_err(|err| format!("resampling failed: {err}"))?;

        out.extend_from_slice(&self.output_buffer[..written * self.channels]);
        self.input_frames += input_frames as u64;
        self.output_frames += written as u64;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_when_rates_match() {
        let mut converter = SampleRateConverter::new(44_100, 44_100, 2).unwrap();
        assert!(converter.is_passthrough());
        let mut out = Vec::new();
        converter.process(&[1.0, 2.0, 3.0, 4.0], &mut out).unwrap();
        assert_eq!(out, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn resamples_to_new_rate() {
        let mut converter = SampleRateConverter::new(44_100, 48_000, 2).unwrap();
        assert!(!converter.is_passthrough());
        let mut out = Vec::new();
        let input: Vec<f32> = (0..4410)
            .map(|i| {
                let t = i as f32 / 44_100.0;
                (t * 440.0 * std::f32::consts::TAU).sin()
            })
            .flat_map(|s| [s, s])
            .collect();
        converter.process(&input, &mut out).unwrap();
        converter.flush(&mut out).unwrap();
        assert!(!out.is_empty());
        assert!(out.iter().all(|s| s.is_finite()));
    }
}
