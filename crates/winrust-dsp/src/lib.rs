//! DSP building blocks for winrust.
//!
//! Contains the 10 band equalizer (mirroring the classic Winamp band set),
//! an FFT based spectrum analyzer for the visualizer and small helpers for
//! level metering.

use std::f32::consts::PI;

use biquad::{Biquad, Coefficients, DirectForm2Transposed, ToHertz, Type};
use rustfft::num_complex::Complex32;
use rustfft::FftPlanner;

/// Winamp's classic equalizer centre frequencies in Hz.
pub const WINAMP_BANDS: [f32; 10] = [
    60.0, 170.0, 310.0, 600.0, 1_000.0, 3_000.0, 6_000.0, 12_000.0, 14_000.0, 16_000.0,
];

/// Minimum gain in dB for the equalizer bands.
pub const MIN_GAIN_DB: f32 = -12.0;
/// Maximum gain in dB for the equalizer bands.
pub const MAX_GAIN_DB: f32 = 12.0;

/// Number of bands exposed by the equalizer.
pub const BAND_COUNT: usize = WINAMP_BANDS.len();

/// A single biquad filter section.
type Section = DirectForm2Transposed<f32>;

/// A 10 band graphic equalizer with a preamp, applied in series.
///
/// The band layout matches the classic Winamp equalizer so that presets
/// authored for the original player keep working.
#[derive(Debug)]
pub struct Equalizer {
    /// One filter section per band, one entry per channel inside.
    sections: Vec<Vec<Section>>,
    gains_db: [f32; BAND_COUNT],
    preamp_db: f32,
    sample_rate: f32,
    channels: usize,
    enabled: bool,
}

impl Equalizer {
    /// Creates a stereo equalizer for the given sample rate.
    pub fn new(sample_rate: f32) -> Self {
        Self::with_channels(sample_rate, 2)
    }

    /// Creates an equalizer with an explicit channel count.
    pub fn with_channels(sample_rate: f32, channels: usize) -> Self {
        let channels = channels.max(1);
        let mut eq = Self {
            sections: Vec::with_capacity(BAND_COUNT),
            gains_db: [0.0; BAND_COUNT],
            preamp_db: 0.0,
            sample_rate,
            channels,
            enabled: true,
        };
        eq.rebuild();
        eq
    }

    /// Rebuilds the filter chain from the current gains.
    fn rebuild(&mut self) {
        self.sections.clear();
        for (band, &freq) in WINAMP_BANDS.iter().enumerate() {
            let coeffs = Coefficients::<f32>::from_params(
                Type::PeakingEQ(self.gains_db[band]),
                self.sample_rate.hz(),
                freq.hz(),
                1.0,
            );
            let coeffs = match coeffs {
                Ok(c) => c,
                // A band above Nyquist cannot be built; skip it silently.
                Err(_) => continue,
            };
            let per_channel = (0..self.channels).map(|_| Section::new(coeffs)).collect();
            self.sections.push(per_channel);
        }
    }

    /// Band gains in dB, one per band.
    pub fn band_gains_db(&self) -> &[f32; BAND_COUNT] {
        &self.gains_db
    }

    /// Sets the gain of a single band in dB.
    pub fn set_band_gain(&mut self, band: usize, gain_db: f32) {
        if band >= BAND_COUNT {
            return;
        }
        self.gains_db[band] = gain_db.clamp(MIN_GAIN_DB, MAX_GAIN_DB);
        self.rebuild();
    }

    /// Sets all band gains at once.
    pub fn set_band_gains(&mut self, gains: [f32; BAND_COUNT]) {
        for (slot, gain) in self.gains_db.iter_mut().zip(gains) {
            *slot = gain.clamp(MIN_GAIN_DB, MAX_GAIN_DB);
        }
        self.rebuild();
    }

    /// Preamp gain in dB.
    pub fn preamp_db(&self) -> f32 {
        self.preamp_db
    }

    /// Sets the preamp gain in dB.
    pub fn set_preamp_db(&mut self, gain_db: f32) {
        self.preamp_db = gain_db.clamp(MIN_GAIN_DB, MAX_GAIN_DB);
    }

    /// Enables or bypasses the whole equalizer.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Whether the equalizer is active.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Resets every band and the preamp to zero.
    pub fn reset(&mut self) {
        self.gains_db = [0.0; BAND_COUNT];
        self.preamp_db = 0.0;
        self.rebuild();
    }

    /// Applies the equalizer to a block of interleaved samples in place.
    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        if !self.enabled || samples.is_empty() {
            return;
        }

        let preamp = 10f32.powf(self.preamp_db / 20.0);
        if (preamp - 1.0).abs() > f32::EPSILON {
            for sample in samples.iter_mut() {
                *sample *= preamp;
            }
        }

        let channels = self.channels;
        if channels <= 1 {
            for band in self.sections.iter_mut() {
                if let Some(section) = band.first_mut() {
                    for sample in samples.iter_mut() {
                        *sample = section.run(*sample);
                    }
                }
            }
            return;
        }

        // Each channel keeps its own filter state, so the samples are
        // processed frame by frame.
        for band in self.sections.iter_mut() {
            for frame in samples.chunks_mut(channels) {
                for (channel, sample) in frame.iter_mut().enumerate() {
                    if let Some(section) = band.get_mut(channel) {
                        *sample = section.run(*sample);
                    }
                }
            }
        }
    }
}

/// An FFT based spectrum analyzer producing logarithmic bands.
///
/// The analyzer mirrors the rough behaviour of the classic Winamp
/// visualizer: a handful of bars that react to the incoming signal.
pub struct Visualizer {
    fft_size: usize,
    planner: FftPlanner<f32>,
    window: Vec<f32>,
    scratch: Vec<Complex32>,
    magnitudes: Vec<f32>,
    /// Normalised band values in the `0.0..=1.0` range.
    pub bars: Vec<f32>,
    /// Number of bars exposed to the UI.
    pub bar_count: usize,
    sample_rate: f32,
}

impl Visualizer {
    /// Creates a visualizer with the given bar count and sample rate.
    pub fn new(bar_count: usize, sample_rate: f32) -> Self {
        let fft_size = 1024;
        let planner = FftPlanner::<f32>::new();
        let window = hann_window(fft_size);
        Self {
            fft_size,
            planner,
            window,
            scratch: vec![Complex32::new(0.0, 0.0); fft_size],
            magnitudes: vec![0.0; fft_size / 2],
            bars: vec![0.0; bar_count],
            bar_count,
            sample_rate,
        }
    }

    /// Feeds interleaved samples and updates the bar values.
    pub fn push_interleaved(&mut self, samples: &[f32]) {
        let mono = to_mono(samples, 2);
        self.push_mono(&mono);
    }

    /// Feeds mono samples and updates the bar values.
    pub fn push_mono(&mut self, samples: &[f32]) {
        if samples.is_empty() || self.bar_count == 0 {
            return;
        }

        let len = samples.len().min(self.fft_size);
        let offset = samples.len() - len;

        for i in 0..self.fft_size {
            let sample = if i < len { samples[offset + i] } else { 0.0 };
            self.scratch[i].re = sample * self.window[i];
            self.scratch[i].im = 0.0;
        }

        let fft = self.planner.plan_fft_forward(self.fft_size);
        fft.process(&mut self.scratch);

        let half = self.fft_size / 2;
        for i in 0..half {
            self.magnitudes[i] = self.scratch[i].norm();
        }

        // Map the linear spectrum onto logarithmic bands, roughly matching
        // how the original player grouped frequencies.
        let nyquist = (self.sample_rate / 2.0).max(1.0);
        let min_freq = 40.0f32;
        let max_freq = nyquist.min(16_000.0).max(min_freq * 2.0);

        for bar in 0..self.bar_count {
            let lo_ratio = bar as f32 / self.bar_count as f32;
            let hi_ratio = (bar + 1) as f32 / self.bar_count as f32;
            let lo_freq = min_freq * (max_freq / min_freq).powf(lo_ratio);
            let hi_freq = min_freq * (max_freq / min_freq).powf(hi_ratio);

            let lo_bin = (((lo_freq / nyquist) * half as f32) as usize).min(half - 1);
            let hi_bin = (((hi_freq / nyquist) * half as f32) as usize)
                .max(lo_bin + 1)
                .min(half);

            let mut peak = 0.0f32;
            for bin in lo_bin..hi_bin {
                peak = peak.max(self.magnitudes[bin]);
            }

            // dB scaling with a -60 dB floor, then map into 0..1.
            let db = 20.0 * peak.max(1e-9).log10();
            let normalized = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
            let normalized = normalized.powf(1.6);

            // Smooth the bar movement so the display does not flicker.
            let previous = self.bars[bar];
            self.bars[bar] = previous * 0.6 + normalized * 0.4;
        }
    }

    /// Decays all bars, used when playback stops.
    pub fn decay(&mut self) {
        for bar in self.bars.iter_mut() {
            *bar *= 0.85;
        }
    }
}

impl std::fmt::Debug for Visualizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Visualizer")
            .field("bar_count", &self.bar_count)
            .field("sample_rate", &self.sample_rate)
            .field("bars", &self.bars)
            .finish()
    }
}

/// Generates a Hann window of the given length.
fn hann_window(len: usize) -> Vec<f32> {
    (0..len)
        .map(|i| {
            let t = i as f32 / (len as f32 - 1.0).max(1.0);
            0.5 * (1.0 - (2.0 * PI * t).cos())
        })
        .collect()
}

/// Downmixes interleaved samples to mono.
pub fn to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return samples.to_vec();
    }
    samples
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Converts a linear amplitude to decibels.
pub fn amplitude_to_db(amplitude: f32) -> f32 {
    20.0 * amplitude.max(1e-9).log10()
}

/// A first order low pass filter, useful for smoothing VU meter values.
#[derive(Debug, Clone, Copy)]
pub struct OnePole {
    alpha: f32,
    state: f32,
}

impl OnePole {
    /// Creates a filter with the given smoothing factor in the `0.0..1.0` range.
    pub fn new(alpha: f32) -> Self {
        Self {
            alpha: alpha.clamp(0.0, 1.0),
            state: 0.0,
        }
    }

    /// Filters a single sample.
    pub fn process(&mut self, sample: f32) -> f32 {
        self.state += self.alpha * (sample - self.state);
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equalizer_clamps_gains() {
        let mut eq = Equalizer::new(44_100.0);
        eq.set_band_gain(0, 100.0);
        assert_eq!(eq.band_gains_db()[0], MAX_GAIN_DB);
        eq.set_band_gain(0, -100.0);
        assert_eq!(eq.band_gains_db()[0], MIN_GAIN_DB);
    }

    #[test]
    fn equalizer_processes_stereo_without_distortion() {
        let mut eq = Equalizer::new(44_100.0);
        let samples: Vec<f32> = (0..1024)
            .map(|i| {
                let t = i as f32 / 44_100.0;
                (t * 440.0 * 2.0 * PI).sin()
            })
            .collect();
        let interleaved: Vec<f32> = samples.iter().flat_map(|s| [*s, *s]).collect();
        let mut processed = interleaved.clone();
        eq.process_interleaved(&mut processed);
        assert_eq!(processed.len(), interleaved.len());
        assert!(processed.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn visualizer_produces_bars() {
        let mut viz = Visualizer::new(20, 44_100.0);
        let samples: Vec<f32> = (0..2048).map(|i| (i as f32 * 0.05).sin()).collect();
        viz.push_mono(&samples);
        assert_eq!(viz.bars.len(), 20);
        assert!(viz.bars.iter().all(|b| (0.0..=1.0).contains(b)));
    }
}
