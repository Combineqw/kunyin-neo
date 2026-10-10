//! Apple HIG Design System Tokens for KUNYIN Neo Native Client.
//!
//! Fully aligned with Apple Human Interface Guidelines:
//! - SF Pro Optical Tracking and Typography
//! - 4pt Optical Grid Spacing
//! - Continuous Squircles and Curvature
//! - Dynamic Contrast-Safe Dual Modes (Light / Dark)
//! - Tactile 44pt Touch Targets & Frosted Glass Highlights

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Light,
    Dark,
}

impl Default for ThemeMode {
    fn default() -> Self {
        Self::Dark
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorRgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ColorRgba {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn hex(hex_str: &str, alpha: f32) -> Self {
        let clean = hex_str.trim_start_matches('#');
        if clean.len() == 6 {
            let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(0) as f32 / 255.0;
            let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(0) as f32 / 255.0;
            let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(0) as f32 / 255.0;
            Self::new(r, g, b, alpha)
        } else {
            Self::new(0.0, 0.0, 0.0, alpha)
        }
    }

    pub fn to_hex(&self) -> String {
        format!(
            "#{:02X}{:02X}{:02X}",
            (self.r * 255.0).round() as u8,
            (self.g * 255.0).round() as u8,
            (self.b * 255.0).round() as u8
        )
    }

    pub fn to_css_rgba(&self) -> String {
        format!(
            "rgba({}, {}, {}, {:.2})",
            (self.r * 255.0).round() as u8,
            (self.g * 255.0).round() as u8,
            (self.b * 255.0).round() as u8,
            self.a
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppleColors {
    pub background_primary: ColorRgba,
    pub background_secondary: ColorRgba,
    pub background_tertiary: ColorRgba,
    pub background_glass: ColorRgba,
    pub text_primary: ColorRgba,
    pub text_secondary: ColorRgba,
    pub text_tertiary: ColorRgba,
    pub accent_primary: ColorRgba,
    pub accent_hover: ColorRgba,
    pub border_subtle: ColorRgba,
    pub border_highlight: ColorRgba,
    pub surface_pill_active: ColorRgba,
}

impl AppleColors {
    pub fn light() -> Self {
        Self {
            background_primary: ColorRgba::hex("#FFFFFF", 1.0),
            background_secondary: ColorRgba::hex("#F5F5F7", 1.0),
            background_tertiary: ColorRgba::hex("#EBEBED", 1.0),
            background_glass: ColorRgba::new(1.0, 1.0, 1.0, 0.78),
            text_primary: ColorRgba::hex("#1C1C1E", 1.0),
            text_secondary: ColorRgba::hex("#8E8E93", 1.0),
            text_tertiary: ColorRgba::hex("#AEAEB2", 1.0),
            accent_primary: ColorRgba::hex("#007AFF", 1.0),
            accent_hover: ColorRgba::hex("#0062CC", 1.0),
            border_subtle: ColorRgba::new(0.0, 0.0, 0.0, 0.08),
            border_highlight: ColorRgba::new(1.0, 1.0, 1.0, 0.8),
            surface_pill_active: ColorRgba::new(0.0, 0.48, 1.0, 0.12),
        }
    }

    pub fn dark() -> Self {
        Self {
            background_primary: ColorRgba::hex("#0F0F12", 1.0),
            background_secondary: ColorRgba::hex("#1C1C1E", 1.0),
            background_tertiary: ColorRgba::hex("#2C2C2E", 1.0),
            background_glass: ColorRgba::new(0.06, 0.06, 0.08, 0.78),
            text_primary: ColorRgba::hex("#F5F5F7", 1.0),
            text_secondary: ColorRgba::hex("#98989D", 1.0),
            text_tertiary: ColorRgba::hex("#636366", 1.0),
            accent_primary: ColorRgba::hex("#0A84FF", 1.0),
            accent_hover: ColorRgba::hex("#409CFF", 1.0),
            border_subtle: ColorRgba::new(1.0, 1.0, 1.0, 0.08),
            border_highlight: ColorRgba::new(1.0, 1.0, 1.0, 0.14),
            surface_pill_active: ColorRgba::new(0.04, 0.52, 1.0, 0.18),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppleMetrics {
    // 4pt optical grid
    pub space_xs: f32, // 4.0
    pub space_sm: f32, // 8.0
    pub space_md: f32, // 12.0
    pub space_lg: f32, // 16.0
    pub space_xl: f32, // 24.0
    pub space_2xl: f32, // 32.0

    // Component Dimensions
    pub sidebar_width: f32,    // 220.0
    pub toolbar_height: f32,   // 52.0
    pub player_bar_height: f32,// 80.0
    pub play_button_size: f32, // 44.0 (HIG minimum tactile target)
    pub cover_size: f32,       // 56.0
    pub nav_capsule_height: f32, // 36.0

    // Corner Radii (Squircle)
    pub radius_xs: f32,   // 6.0
    pub radius_sm: f32,   // 8.0
    pub radius_cover: f32,// 10.0
    pub radius_card: f32, // 16.0
    pub radius_pill: f32, // 999.0

    // Typography
    pub font_size_large_title: f32, // 28.0
    pub font_size_title: f32,       // 20.0
    pub font_size_headline: f32,    // 16.0
    pub font_size_body: f32,        // 14.0
    pub font_size_caption: f32,     // 12.0
    pub font_size_micro: f32,       // 10.0
    pub letter_spacing: f32,        // -0.015em
}

impl Default for AppleMetrics {
    fn default() -> Self {
        Self {
            space_xs: 4.0,
            space_sm: 8.0,
            space_md: 12.0,
            space_lg: 16.0,
            space_xl: 24.0,
            space_2xl: 32.0,
            sidebar_width: 220.0,
            toolbar_height: 52.0,
            player_bar_height: 80.0,
            play_button_size: 44.0,
            cover_size: 56.0,
            nav_capsule_height: 36.0,
            radius_xs: 6.0,
            radius_sm: 8.0,
            radius_cover: 10.0,
            radius_card: 16.0,
            radius_pill: 999.0,
            font_size_large_title: 28.0,
            font_size_title: 20.0,
            font_size_headline: 16.0,
            font_size_body: 14.0,
            font_size_caption: 12.0,
            font_size_micro: 10.0,
            letter_spacing: -0.015,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppleTheme {
    pub mode: ThemeMode,
    pub colors: AppleColors,
    pub metrics: AppleMetrics,
}

impl Default for AppleTheme {
    fn default() -> Self {
        Self::new(ThemeMode::Dark)
    }
}

impl AppleTheme {
    pub fn new(mode: ThemeMode) -> Self {
        let colors = match mode {
            ThemeMode::Light => AppleColors::light(),
            ThemeMode::Dark => AppleColors::dark(),
        };
        Self {
            mode,
            colors,
            metrics: AppleMetrics::default(),
        }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        };
        self.colors = match self.mode {
            ThemeMode::Light => AppleColors::light(),
            ThemeMode::Dark => AppleColors::dark(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_mode_toggles_and_updates_colors() {
        let mut theme = AppleTheme::new(ThemeMode::Dark);
        assert_eq!(theme.mode, ThemeMode::Dark);
        assert_eq!(theme.colors.background_primary.to_hex(), "#0F0F12");

        theme.toggle_mode();
        assert_eq!(theme.mode, ThemeMode::Light);
        assert_eq!(theme.colors.background_primary.to_hex(), "#FFFFFF");
        assert_eq!(theme.colors.accent_primary.to_hex(), "#007AFF");
    }

    #[test]
    fn metrics_adhere_to_apple_hig_spec() {
        let metrics = AppleMetrics::default();
        assert_eq!(metrics.play_button_size, 44.0); // 44pt touch target rule
        assert_eq!(metrics.radius_pill, 999.0);
        assert_eq!(metrics.radius_card, 16.0);
        assert_eq!(metrics.space_lg, 16.0);
        assert_eq!(metrics.sidebar_width, 220.0);
    }
}
