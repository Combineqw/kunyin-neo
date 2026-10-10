//! Native Audio DSP Engine: 10-Band Biquad IIR EQ, ReplayGain, Gain Ramp & Soft Limiter.
//!
//! Provides an allocation-free, sample-accurate audio processing pipeline for
//! interleaved f32 PCM buffers. Directly mirrors Web Audio API transfer functions
//! while executing completely natively in the audio pipeline.

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

use crate::{PcmFormat, ReplayGainDsp};

pub const EQ_BAND_COUNT: usize = 10;

/// Standard ISO 10-band center frequencies (in Hz).
pub const EQ_FREQUENCIES: [f64; EQ_BAND_COUNT] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

/// Filter type for biquad audio filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FilterType {
    LowShelf,
    Peaking,
    HighShelf,
}

/// Normalized biquad filter coefficients (where a0 = 1.0).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BiquadCoeffs {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
}

impl Default for BiquadCoeffs {
    fn default() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }
}

impl BiquadCoeffs {
    /// Calculate normalized biquad coefficients according to Audio EQ Cookbook.
    pub fn calculate(filter_type: FilterType, freq: f64, sample_rate: u32, q: f64, gain_db: f64) -> Self {
        let sr = (sample_rate as f64).max(8000.0);
        // Clamp frequency below Nyquist frequency
        let f0 = freq.clamp(10.0, sr * 0.499);
        let q = q.max(0.01);
        let w0 = 2.0 * PI * f0 / sr;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();
        let a = 10.0_f64.powf(gain_db / 40.0);

        let (b0, b1, b2, a0, a1, a2) = match filter_type {
            FilterType::Peaking => {
                let b0 = 1.0 + alpha * a;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0 - alpha * a;
                let a0 = 1.0 + alpha / a;
                let a1 = -2.0 * cos_w0;
                let a2 = 1.0 - alpha / a;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::LowShelf => {
                let sqrt_a = a.sqrt();
                let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha);
                let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha);
                let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
                let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighShelf => {
                let sqrt_a = a.sqrt();
                let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha);
                let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha);
                let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
                let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        if a0.abs() < 1e-12 {
            return Self::default();
        }

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }
}

/// A biquad filter section maintaining per-channel Direct Form II Transposed state.
#[derive(Debug, Clone, PartialEq)]
pub struct BiquadFilter {
    pub filter_type: FilterType,
    pub freq: f64,
    pub q: f64,
    pub gain_db: f64,
    pub coeffs: BiquadCoeffs,
    // (s1, s2) per channel
    state: Vec<[f64; 2]>,
    is_active: bool,
}

impl BiquadFilter {
    pub fn new(filter_type: FilterType, freq: f64, sample_rate: u32, q: f64, gain_db: f64, channels: u16) -> Self {
        let is_active = gain_db.abs() > 0.01;
        let coeffs = if is_active {
            BiquadCoeffs::calculate(filter_type, freq, sample_rate, q, gain_db)
        } else {
            BiquadCoeffs::default()
        };
        let ch = usize::from(channels).max(1);
        Self {
            filter_type,
            freq,
            q,
            gain_db,
            coeffs,
            state: vec![[0.0, 0.0]; ch],
            is_active,
        }
    }

    pub fn set_gain(&mut self, gain_db: f64, sample_rate: u32) {
        self.gain_db = gain_db.clamp(-12.0, 12.0);
        self.is_active = self.gain_db.abs() > 0.01;
        if self.is_active {
            self.coeffs = BiquadCoeffs::calculate(
                self.filter_type,
                self.freq,
                sample_rate,
                self.q,
                self.gain_db,
            );
        } else {
            self.coeffs = BiquadCoeffs::default();
        }
    }

    pub fn reset_state(&mut self) {
        for s in &mut self.state {
            *s = [0.0, 0.0];
        }
    }

    pub fn ensure_channels(&mut self, channels: u16) {
        let ch = usize::from(channels).max(1);
        if self.state.len() != ch {
            self.state = vec![[0.0, 0.0]; ch];
        }
    }

    /// Process a single sample through Direct Form II Transposed filter.
    #[inline(always)]
    pub fn process_sample(&mut self, channel: usize, input: f64) -> f64 {
        if !self.is_active || channel >= self.state.len() {
            return input;
        }
        let s = &mut self.state[channel];
        let y = self.coeffs.b0 * input + s[0];
        s[0] = self.coeffs.b1 * input - self.coeffs.a1 * y + s[1];
        s[1] = self.coeffs.b2 * input - self.coeffs.a2 * y;
        y
    }
}

/// 10-Band Biquad IIR Equalizer DSP matching the Web Audio API filter cascade.
#[derive(Debug, Clone, PartialEq)]
pub struct EqualizerDsp {
    sample_rate: u32,
    channels: u16,
    bands: Vec<BiquadFilter>,
    enabled: bool,
}

impl EqualizerDsp {
    /// Create a standard 10-band equalizer.
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        let mut bands = Vec::with_capacity(EQ_BAND_COUNT);
        for (i, &freq) in EQ_FREQUENCIES.iter().enumerate() {
            let filter_type = if i == 0 {
                FilterType::LowShelf
            } else if i == EQ_BAND_COUNT - 1 {
                FilterType::HighShelf
            } else {
                FilterType::Peaking
            };
            // Q = 1.414 for 1-octave bandwidth
            let q = 1.41421356;
            bands.push(BiquadFilter::new(filter_type, freq, sample_rate, q, 0.0, channels));
        }
        Self {
            sample_rate,
            channels,
            bands,
            enabled: true,
        }
    }

    /// Set gains for all 10 bands.
    pub fn set_gains(&mut self, gains_db: &[f64; EQ_BAND_COUNT]) {
        for (i, &gain) in gains_db.iter().enumerate() {
            if let Some(band) = self.bands.get_mut(i) {
                band.set_gain(gain, self.sample_rate);
            }
        }
    }

    /// Set gain for a single band (0..9).
    pub fn set_band_gain(&mut self, band_index: usize, gain_db: f64) {
        if let Some(band) = self.bands.get_mut(band_index) {
            band.set_gain(gain_db, self.sample_rate);
        }
    }

    /// Enable or disable equalizer processing.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Check if any band is actively filtering.
    pub fn is_active(&self) -> bool {
        self.enabled && self.bands.iter().any(|b| b.is_active)
    }

    /// Update sample rate and recalculate all active filter coefficients.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        if self.sample_rate != sample_rate {
            self.sample_rate = sample_rate;
            for band in &mut self.bands {
                band.set_gain(band.gain_db, sample_rate);
            }
        }
    }

    /// Ensure internal state tracks the channel count.
    pub fn ensure_channels(&mut self, channels: u16) {
        if self.channels != channels {
            self.channels = channels;
            for band in &mut self.bands {
                band.ensure_channels(channels);
            }
        }
    }

    /// Reset internal filter state (prevents clicks between songs or seeks).
    pub fn reset_state(&mut self) {
        for band in &mut self.bands {
            band.reset_state();
        }
    }

    /// In-place process interleaved f32 PCM buffer.
    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        if !self.is_active() || samples.is_empty() {
            return;
        }

        let channels = usize::from(self.channels).max(1);
        let frame_count = samples.len() / channels;

        for frame in 0..frame_count {
            let offset = frame * channels;
            for ch in 0..channels {
                let mut val = samples[offset + ch] as f64;
                for band in &mut self.bands {
                    if band.is_active {
                        val = band.process_sample(ch, val);
                    }
                }
                samples[offset + ch] = val as f32;
            }
        }
    }
}

/// Configuration payload for the DSP pipeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DspPipelineConfig {
    pub eq_enabled: bool,
    pub eq_gains_db: [f64; EQ_BAND_COUNT],
    pub replay_gain_db: Option<f64>,
    pub volume: f32,
    pub muted: bool,
    pub limiter_enabled: bool,
}

impl Default for DspPipelineConfig {
    fn default() -> Self {
        Self {
            eq_enabled: false,
            eq_gains_db: [0.0; EQ_BAND_COUNT],
            replay_gain_db: None,
            volume: 1.0,
            muted: false,
            limiter_enabled: true,
        }
    }
}

/// Snapshot of the DSP pipeline metrics and states.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DspPipelineSnapshot {
    pub format: PcmFormat,
    pub config: DspPipelineConfig,
    pub eq_active: bool,
    pub current_gain_linear: f32,
    pub replay_gain_linear: f32,
    pub peak_level: f32,
}

/// Comprehensive native DSP pipeline: Equalizer -> ReplayGain -> Smooth Gain Ramp -> Soft Limiter.
pub struct NativeDspPipeline {
    format: PcmFormat,
    eq: EqualizerDsp,
    config: DspPipelineConfig,
    current_volume: f32,
    target_volume: f32,
    volume_step: f32,
    replay_gain_linear: f32,
    last_peak_level: f32,
}

impl NativeDspPipeline {
    pub fn new(format: PcmFormat, config: DspPipelineConfig) -> Self {
        let mut eq = EqualizerDsp::new(format.sample_rate, format.channels);
        eq.set_enabled(config.eq_enabled);
        eq.set_gains(&config.eq_gains_db);

        let target_vol = if config.muted { 0.0 } else { config.volume.clamp(0.0, 1.0) };
        let rg_linear = config
            .replay_gain_db
            .map(|db| ReplayGainDsp::db_to_linear(db) as f32)
            .unwrap_or(1.0);

        Self {
            format,
            eq,
            config,
            current_volume: target_vol,
            target_volume: target_vol,
            // 20ms ramp duration
            volume_step: Self::calc_ramp_step(format.sample_rate, 0.020),
            replay_gain_linear: rg_linear,
            last_peak_level: 0.0,
        }
    }

    fn calc_ramp_step(sample_rate: u32, duration_secs: f32) -> f32 {
        let total_frames = ((sample_rate as f32) * duration_secs).max(1.0);
        1.0 / total_frames
    }

    /// Configure the pipeline settings.
    pub fn configure(&mut self, config: DspPipelineConfig) {
        self.eq.set_enabled(config.eq_enabled);
        self.eq.set_gains(&config.eq_gains_db);

        let new_target = if config.muted { 0.0 } else { config.volume.clamp(0.0, 1.0) };
        self.target_volume = new_target;

        self.replay_gain_linear = config
            .replay_gain_db
            .map(|db| ReplayGainDsp::db_to_linear(db) as f32)
            .unwrap_or(1.0);

        self.config = config;
    }

    /// Update format and reconfigure sample rate & channels.
    pub fn set_format(&mut self, format: PcmFormat) {
        if self.format != format {
            self.format = format;
            self.eq.set_sample_rate(format.sample_rate);
            self.eq.ensure_channels(format.channels);
            self.volume_step = Self::calc_ramp_step(format.sample_rate, 0.020);
        }
    }

    /// Reset internal filter & ramp state.
    pub fn reset(&mut self) {
        self.eq.reset_state();
        self.current_volume = self.target_volume;
        self.last_peak_level = 0.0;
    }

    /// Soft limiter: smooth saturation curve when |s| approaches or exceeds 0.95.
    #[inline(always)]
    fn soft_limit(s: f32) -> f32 {
        let threshold = 0.95f32;
        let abs_s = s.abs();
        if abs_s <= threshold {
            s
        } else {
            // Cubic soft saturation curve mapping smoothly towards 1.0
            let sign = if s >= 0.0 { 1.0 } else { -1.0 };
            let diff = abs_s - threshold;
            let compressed = threshold + (1.0 - threshold) * (diff / (1.0 + diff));
            sign * compressed.min(0.9999)
        }
    }

    /// In-place process interleaved PCM buffer through all DSP stages.
    pub fn process_interleaved(&mut self, samples: &mut [f32]) {
        if samples.is_empty() {
            return;
        }

        // 1. Equalizer Stage
        self.eq.process_interleaved(samples);

        // 2. ReplayGain & Smooth Gain Ramp & Soft Limiter Stage
        let channels = usize::from(self.format.channels).max(1);
        let frame_count = samples.len() / channels;
        let mut max_peak = 0.0f32;

        for frame in 0..frame_count {
            // Advance volume ramp per frame
            if (self.current_volume - self.target_volume).abs() > 1e-5 {
                if self.current_volume < self.target_volume {
                    self.current_volume = (self.current_volume + self.volume_step).min(self.target_volume);
                } else {
                    self.current_volume = (self.current_volume - self.volume_step).max(self.target_volume);
                }
            } else {
                self.current_volume = self.target_volume;
            }

            let effective_gain = self.current_volume * self.replay_gain_linear;
            let offset = frame * channels;

            for ch in 0..channels {
                let mut s = samples[offset + ch] * effective_gain;
                if self.config.limiter_enabled {
                    s = Self::soft_limit(s);
                }
                samples[offset + ch] = s;
                let abs_s = s.abs();
                if abs_s > max_peak {
                    max_peak = abs_s;
                }
            }
        }

        self.last_peak_level = max_peak;
    }

    /// Obtain snapshot of the DSP pipeline.
    pub fn snapshot(&self) -> DspPipelineSnapshot {
        DspPipelineSnapshot {
            format: self.format,
            config: self.config.clone(),
            eq_active: self.eq.is_active(),
            current_gain_linear: self.current_volume,
            replay_gain_linear: self.replay_gain_linear,
            peak_level: self.last_peak_level,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn biquad_coeffs_peaking_at_unity_gain_is_neutral() {
        let coeffs = BiquadCoeffs::calculate(FilterType::Peaking, 1000.0, 44100, 1.414, 0.0);
        assert!((coeffs.b0 - 1.0).abs() < 1e-4);
        assert!(coeffs.b1.abs() > 0.0);
        assert!((coeffs.a1 - coeffs.b1).abs() < 1e-4);
    }

    #[test]
    fn equalizer_bypasses_when_all_gains_zero() {
        let mut eq = EqualizerDsp::new(44100, 2);
        assert!(!eq.is_active());

        let mut samples = vec![0.5f32, -0.5f32, 0.25f32, -0.25f32];
        let original = samples.clone();
        eq.process_interleaved(&mut samples);

        assert_eq!(samples, original);
    }

    #[test]
    fn equalizer_boosts_target_frequency_band() {
        let mut eq = EqualizerDsp::new(44100, 1);
        eq.set_band_gain(5, 6.0); // +6 dB on 1000 Hz (index 5)
        assert!(eq.is_active());

        // Generate 1 kHz sine wave
        let sample_rate = 44100.0;
        let freq = 1000.0;
        let mut samples: Vec<f32> = (0..441)
            .map(|i| (2.0 * PI * freq * (i as f64) / sample_rate).sin() as f32 * 0.1)
            .collect();

        let orig_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
        eq.process_interleaved(&mut samples);
        let boosted_energy: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();

        // +6 dB means roughly 4x power boost
        assert!(boosted_energy > orig_energy * 2.5);
    }

    #[test]
    fn gain_ramp_smooths_volume_transition() {
        let format = PcmFormat { sample_rate: 44100, channels: 1 };
        let config = DspPipelineConfig {
            volume: 1.0,
            ..Default::default()
        };
        let mut pipeline = NativeDspPipeline::new(format, config);

        // Instant volume cut to 0.0
        pipeline.configure(DspPipelineConfig {
            volume: 0.0,
            ..Default::default()
        });

        let mut samples = vec![1.0f32; 100];
        pipeline.process_interleaved(&mut samples);

        // First sample should be close to 1.0, and steadily decreasing
        assert!(samples[0] > 0.95);
        assert!(samples[99] < samples[0]);
    }

    #[test]
    fn soft_limiter_clamps_extreme_boosts() {
        let format = PcmFormat { sample_rate: 44100, channels: 1 };
        let config = DspPipelineConfig {
            replay_gain_db: Some(12.0), // +12 dB boost = ~4x
            limiter_enabled: true,
            ..Default::default()
        };
        let mut pipeline = NativeDspPipeline::new(format, config);

        let mut samples = vec![0.8f32; 100]; // 0.8 * 4 = 3.2 > 1.0
        pipeline.process_interleaved(&mut samples);

        for &s in &samples {
            assert!(s < 1.0);
            assert!(s > 0.9);
        }
    }
}
