//! Apple HIG Native Mini Player Component for KUNYIN Neo.
//!
//! Provides:
//! - Compact Floating Desktop Pill layout (340pt x 110pt)
//! - Translucent Frosted Glass Visual Tokens
//! - Vinyl / Album Art Rotation Animation state
//! - Two-line Mini Lyric Ticker
//! - Tactile playback controls with 44pt touch targets

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiniPlayerViewModel {
    pub width: f32,
    pub height: f32,
    pub opacity: f32,
    pub vinyl_rotation_deg: f32,
    pub current_lyric: String,
    pub next_lyric: String,
    pub is_playing: bool,
    pub is_always_on_top: bool,
}

impl Default for MiniPlayerViewModel {
    fn default() -> Self {
        Self {
            width: 340.0,
            height: 110.0,
            opacity: 0.85,
            vinyl_rotation_deg: 0.0,
            current_lyric: "昆音 · 原生音频引擎就绪".to_string(),
            next_lyric: "点击播放探索无损音乐".to_string(),
            is_playing: false,
            is_always_on_top: true,
        }
    }
}

impl MiniPlayerViewModel {
    pub fn new(opacity: f32) -> Self {
        let mut model = Self::default();
        model.set_opacity(opacity);
        model
    }

    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.20, 1.0);
    }

    pub fn tick_rotation(&mut self, elapsed_ms: u64) {
        if self.is_playing {
            // One full rotation (360 degrees) every 10 seconds (10,000ms) = 36 deg/sec
            let delta_deg = (elapsed_ms as f32 / 10000.0) * 360.0;
            self.vinyl_rotation_deg = (self.vinyl_rotation_deg + delta_deg) % 360.0;
        }
    }

    pub fn update_lyrics(&mut self, current: impl Into<String>, next: impl Into<String>) {
        self.current_lyric = current.into();
        self.next_lyric = next.into();
    }

    pub fn toggle_always_on_top(&mut self) -> bool {
        self.is_always_on_top = !self.is_always_on_top;
        self.is_always_on_top
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mini_player_clamps_opacity_and_ticks_rotation() {
        let mut mp = MiniPlayerViewModel::new(0.05);
        assert_eq!(mp.opacity, 0.20); // clamped to min

        mp.set_opacity(1.5);
        assert_eq!(mp.opacity, 1.0); // clamped to max

        // Rotation doesn't advance when paused
        mp.is_playing = false;
        mp.tick_rotation(5000);
        assert_eq!(mp.vinyl_rotation_deg, 0.0);

        // Rotation advances when playing
        mp.is_playing = true;
        mp.tick_rotation(2500); // quarter of 10s = 90 degrees
        assert!((mp.vinyl_rotation_deg - 90.0).abs() < 0.1);

        mp.update_lyrics("主歌第一句", "主歌第二句");
        assert_eq!(mp.current_lyric, "主歌第一句");
        assert_eq!(mp.next_lyric, "主歌第二句");

        assert!(mp.is_always_on_top);
        assert!(!mp.toggle_always_on_top());
        assert!(!mp.is_always_on_top);
    }
}
