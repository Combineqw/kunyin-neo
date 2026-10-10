//! Apple HIG Native 10-Band Graphic Equalizer & DSP Visualizer Component for KUNYIN Neo.
//!
//! Provides:
//! - 10-Band Double-Precision Biquad IIR visual frequency curves
//! - dB Gain sliders (-12.0dB .. +12.0dB) with tactile response
//! - Linear spline coordinate generator for Win32 GDI / Direct2D presentation
//! - Preset transitions (平直, 原声, 低音增强, 清澈人声, 电子动感)

use serde::{Deserialize, Serialize};
use crate::components::settings_view::EqPreset;

pub const FREQUENCIES: [&str; 10] = [
    "31Hz", "62Hz", "125Hz", "250Hz", "500Hz", "1kHz", "2kHz", "4kHz", "8kHz", "16kHz",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqVisualizerModel {
    pub enabled: bool,
    pub bands: [f32; 10],
    pub preamp_db: f32,
}

impl Default for EqVisualizerModel {
    fn default() -> Self {
        Self {
            enabled: true,
            bands: [0.0; 10],
            preamp_db: 0.0,
        }
    }
}

impl EqVisualizerModel {
    pub const MIN_GAIN_DB: f32 = -12.0;
    pub const MAX_GAIN_DB: f32 = 12.0;

    pub fn new(bands: [f32; 10], preamp_db: f32, enabled: bool) -> Self {
        let mut model = Self {
            enabled,
            bands: [0.0; 10],
            preamp_db: preamp_db.clamp(-6.0, 6.0),
        };
        for (i, &g) in bands.iter().enumerate() {
            model.bands[i] = g.clamp(Self::MIN_GAIN_DB, Self::MAX_GAIN_DB);
        }
        model
    }

    pub fn set_band_gain(&mut self, index: usize, gain_db: f32) {
        if index < 10 {
            self.bands[index] = gain_db.clamp(Self::MIN_GAIN_DB, Self::MAX_GAIN_DB);
        }
    }

    pub fn apply_preset(&mut self, preset: EqPreset) {
        self.bands = preset.gains();
    }

    pub fn compute_curve_points(&self, width: f32, height: f32, margin: f32) -> Vec<(f32, f32)> {
        let usable_width = (width - 2.0 * margin).max(10.0);
        let usable_height = (height - 2.0 * margin).max(10.0);
        let center_y = margin + usable_height / 2.0;
        let step_x = usable_width / 9.0;

        let mut points = Vec::with_capacity(10);
        for (i, &gain) in self.bands.iter().enumerate() {
            let x = margin + (i as f32) * step_x;
            // Higher gain goes upward (smaller Y)
            let normalized_offset = gain / Self::MAX_GAIN_DB;
            let y = center_y - normalized_offset * (usable_height / 2.0);
            points.push((x, y.clamp(margin, height - margin)));
        }
        points
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eq_visualizer_clamps_gains_and_computes_curve_bounds() {
        let mut eq = EqVisualizerModel::new([15.0; 10], 0.0, true);
        assert_eq!(eq.bands[0], 12.0); // clamped to MAX_GAIN_DB

        eq.set_band_gain(0, -20.0);
        assert_eq!(eq.bands[0], -12.0); // clamped to MIN_GAIN_DB

        let pts = eq.compute_curve_points(500.0, 200.0, 20.0);
        assert_eq!(pts.len(), 10);
        assert_eq!(pts[0].0, 20.0); // x start
        assert_eq!(pts[9].0, 480.0); // x end

        // First point was -12dB (lowest gain), so Y should be maximum (bottom)
        assert_eq!(pts[0].1, 180.0);

        eq.apply_preset(EqPreset::Flat);
        assert_eq!(eq.bands, [0.0; 10]);
        let flat_pts = eq.compute_curve_points(500.0, 200.0, 20.0);
        assert_eq!(flat_pts[0].1, 100.0); // exactly center Y
    }
}
