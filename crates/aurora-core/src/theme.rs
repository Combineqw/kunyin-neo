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
    /// User requested target in frames per second. `None` means automatic
    /// display-synchronised pacing; `Some(0.0)` explicitly requests a static
    /// frame.  The value is kept in the profile so the renderer can reflect
    /// the user's continuous (non-preset) setting.
    pub requested_fps: Option<f32>,
    /// Refresh rate used by the native scheduler.  It is optional because
    /// older callers do not know the monitor they will render on.
    pub display_refresh_hz: Option<f32>,
    /// Effective target after clamping a request to the display. `None` means
    /// static/no redraw.  Keeping this separate from `requested_fps` avoids
    /// making the renderer repeat the clamping policy.
    pub target_fps: Option<f32>,
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
    motion_profile_with_fps(theme_id, reduced_motion, None, None)
}

/// Resolve the motion profile with an optional continuous FPS request and
/// monitor refresh rate. A request of `0` is the explicit static mode;
/// `None` follows the display refresh rate (or the conservative 60 Hz default
/// when the caller has no display information). Positive requests are
/// clamped to the display rate so a 60 Hz panel never burns CPU producing
/// invisible 144 FPS frames.
pub fn motion_profile_with_fps(
    theme_id: &str,
    reduced_motion: bool,
    requested_fps: Option<f32>,
    display_refresh_hz: Option<f32>,
) -> MotionProfile {
    let (blur_px, opacity, scale) = match theme_id {
        "aurora_spring" => (78, 0.78, 0.92),
        "aurora_summer" => (86, 0.76, 1.04),
        "aurora_autumn" => (94, 0.80, 1.08),
        "aurora_winter" => (72, 0.70, 0.88),
        "aurora_polar_night" => (96, 0.62, 1.12),
        id if id.starts_with("aurora_") => (90, 0.74, 1.0),
        _ => (0, 0.0, 1.0),
    };
    let requested_fps = requested_fps.map(|value| {
        if value.is_finite() {
            value.max(0.0)
        } else {
            0.0
        }
    });
    let display_refresh_hz = display_refresh_hz.map(|value| {
        if value.is_finite() && value > 0.0 {
            value.clamp(1.0, 1_000.0)
        } else {
            60.0
        }
    });
    let refresh_hz = display_refresh_hz.unwrap_or(60.0);
    let explicit_static = requested_fps == Some(0.0);
    let enabled = !reduced_motion && blur_px > 0 && !explicit_static;
    let target_fps = if enabled {
        Some(requested_fps.unwrap_or(refresh_hz).clamp(1.0, refresh_hz))
    } else {
        None
    };
    let frame_interval_ms = target_fps
        .map(|fps| (1_000.0 / fps).round().clamp(1.0, u16::MAX as f32) as u16)
        .unwrap_or(1_000);
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
        requested_fps,
        display_refresh_hz,
        target_fps,
        frame_interval_ms,
        blur_px,
        opacity,
        blobs,
    }
}

#[cfg(test)]
mod tests {
    use super::{motion_profile, motion_profile_with_fps};

    #[test]
    fn seasonal_profiles_are_deterministic_and_bounded() {
        let spring = motion_profile("aurora_spring", false);
        assert!(spring.enabled);
        assert_eq!(spring.target_fps, Some(60.0));
        assert_eq!(spring.frame_interval_ms, 17);
        assert_eq!(spring.blobs.len(), 5);
        assert!(spring.blur_px <= 96);
        assert_eq!(spring, motion_profile("aurora_spring", false));
    }

    #[test]
    fn reduced_motion_and_non_aurora_do_not_allocate_blobs() {
        assert!(!motion_profile("aurora_autumn", true).enabled);
        assert!(motion_profile("green", false).blobs.is_empty());
    }

    #[test]
    fn continuous_fps_is_clamped_to_display_and_zero_is_static() {
        let profile = motion_profile_with_fps("aurora_spring", false, Some(73.25), Some(120.0));
        assert_eq!(profile.requested_fps, Some(73.25));
        assert_eq!(profile.display_refresh_hz, Some(120.0));
        assert_eq!(profile.target_fps, Some(73.25));
        assert_eq!(profile.frame_interval_ms, 14);

        let clamped = motion_profile_with_fps("aurora_spring", false, Some(240.0), Some(60.0));
        assert_eq!(clamped.target_fps, Some(60.0));
        assert_eq!(clamped.frame_interval_ms, 17);

        let invalid_display = motion_profile_with_fps("aurora_spring", false, None, Some(0.0));
        assert_eq!(invalid_display.display_refresh_hz, Some(60.0));
        assert_eq!(invalid_display.target_fps, Some(60.0));

        let static_profile =
            motion_profile_with_fps("aurora_spring", false, Some(0.0), Some(144.0));
        assert!(!static_profile.enabled);
        assert_eq!(static_profile.target_fps, None);
        assert!(static_profile.blobs.is_empty());
    }
}
