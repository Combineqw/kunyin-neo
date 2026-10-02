//! Rust-owned theme motion profiles.
//!
//! The renderer still performs the final CSS compositing, but all seasonal
//! motion geometry and timing comes from this deterministic native contract.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MotionBlob {
    pub top_percent: i16,
    pub left_percent: i16,
    pub size_percent: u16,
    pub duration_ms: u32,
    pub delay_ms: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MotionProfile {
    pub theme_id: String,
    pub enabled: bool,
    pub frame_interval_ms: u16,
    pub blur_px: u16,
    pub opacity: f32,
    pub blobs: Vec<MotionBlob>,
}

const LAYOUT: [(i16, i16, u16, u32, i32); 5] = [
    (-10, -5, 70, 18_000, 0),
    (20, 55, 65, 22_000, -6_000),
    (50, 10, 75, 26_000, -12_000),
    (35, 40, 60, 20_000, -3_000),
    (5, 30, 68, 24_000, -9_000),
];

/// Return the native motion contract for one theme. Unknown themes use the
/// conservative default; reduced motion disables animation without changing
/// the theme colors.
pub fn motion_profile(theme_id: &str, reduced_motion: bool) -> MotionProfile {
    let (blur_px, opacity, scale) = match theme_id {
        "aurora_spring" => (78, 0.78, 0.92),
        "aurora_summer" => (86, 0.76, 1.04),
        "aurora_autumn" => (94, 0.80, 1.08),
        "aurora_winter" => (72, 0.70, 0.88),
        "aurora_polar_night" => (96, 0.62, 1.12),
        id if id.starts_with("aurora_") => (90, 0.74, 1.0),
        _ => (0, 0.0, 1.0),
    };
    let enabled = !reduced_motion && blur_px > 0;
    let frame_interval_ms = if enabled { 33 } else { 1_000 };
    let blobs = if enabled {
        LAYOUT
            .iter()
            .map(|(top, left, size, duration, delay)| MotionBlob {
                top_percent: *top,
                left_percent: *left,
                size_percent: ((*size as f32) * scale).round() as u16,
                duration_ms: *duration,
                delay_ms: *delay,
            })
            .collect()
    } else {
        Vec::new()
    };
    MotionProfile {
        theme_id: theme_id.to_string(),
        enabled,
        frame_interval_ms,
        blur_px,
        opacity,
        blobs,
    }
}

#[cfg(test)]
mod tests {
    use super::motion_profile;

    #[test]
    fn seasonal_profiles_are_deterministic_and_bounded() {
        let spring = motion_profile("aurora_spring", false);
        assert!(spring.enabled);
        assert_eq!(spring.frame_interval_ms, 33);
        assert_eq!(spring.blobs.len(), 5);
        assert!(spring.blur_px <= 96);
        assert_eq!(spring, motion_profile("aurora_spring", false));
    }

    #[test]
    fn reduced_motion_and_non_aurora_do_not_allocate_blobs() {
        assert!(!motion_profile("aurora_autumn", true).enabled);
        assert!(motion_profile("green", false).blobs.is_empty());
    }
}
