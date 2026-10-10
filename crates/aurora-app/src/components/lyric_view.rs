//! Fullscreen Immersive Lyric View for KUNYIN Neo (Apple HIG Design).
//!
//! Features:
//! - Centered dynamic scrolling lyric timeline
//! - 1.08x active line focal magnification with SF Pro typography
//! - Depth-of-field opacity hierarchy (Active 100%, near 60%, far 30%)
//! - Tactile clickable line seeking & timeline badge

use serde::{Deserialize, Serialize};
use aurora_core::lyrics::{find_active_line, LyricLine};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LyricLineRenderItem {
    pub index: usize,
    pub start_ms: u64,
    pub time_str: String,
    pub text: String,
    pub is_active: bool,
    pub scale: f32,
    pub opacity: f32,
    pub color_rgb: u32,
}

#[derive(Debug, Clone)]
pub struct LyricViewModel {
    pub lines: Vec<LyricLine>,
}

impl LyricViewModel {
    pub fn new(lines: Vec<LyricLine>) -> Self {
        Self { lines }
    }

    pub fn format_time_badge(ms: u64) -> String {
        let total_sec = ms / 1000;
        let min = total_sec / 60;
        let sec = total_sec % 60;
        format!("{:02}:{:02}", min, sec)
    }

    /// Computes a visible window of lines around the active line for rendering.
    pub fn build_viewport(&self, current_time_ms: u64, window_size: usize) -> Vec<LyricLineRenderItem> {
        if self.lines.is_empty() {
            return Vec::new();
        }

        let active_match = find_active_line(&self.lines, current_time_ms);
        let active_idx = active_match.map(|m| m.index).unwrap_or(0);

        let half_window = window_size / 2;
        let start_idx = active_idx.saturating_sub(half_window);
        let end_idx = (start_idx + window_size).min(self.lines.len());

        let mut items = Vec::new();
        for idx in start_idx..end_idx {
            let line = &self.lines[idx];
            let is_active = idx == active_idx;
            let distance = (idx as isize - active_idx as isize).abs();

            let (scale, opacity, color_rgb) = if is_active {
                // Active line focal state
                (1.08, 1.0, 0x00E8C083) // Apple Gold/Champagne accent
            } else {
                let op = match distance {
                    1 => 0.65,
                    2 => 0.42,
                    _ => 0.25,
                };
                (1.0, op, 0x008E8E93) // SF Pro secondary muted gray
            };

            items.push(LyricLineRenderItem {
                index: idx,
                start_ms: line.start,
                time_str: Self::format_time_badge(line.start),
                text: line.text.clone(),
                is_active,
                scale,
                opacity,
                color_rgb,
            });
        }

        items
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn lyric_view_centers_active_line_and_scales_appropriately() {
        let lines = vec![
            LyricLine { start: 1000, end: 3000, text: "第一句歌词".to_string() },
            LyricLine { start: 3000, end: 6000, text: "第二句高潮".to_string() },
            LyricLine { start: 6000, end: 9000, text: "第三句回落".to_string() },
            LyricLine { start: 9000, end: 12000, text: "第四句尾奏".to_string() },
        ];

        let model = LyricViewModel::new(lines);
        let viewport = model.build_viewport(4500, 5); // At 4.5s, line 1 is active

        assert!(!viewport.is_empty());
        let active_item = viewport.iter().find(|i| i.is_active).expect("must find active line");
        assert_eq!(active_item.index, 1);
        assert_eq!(active_item.text, "第二句高潮");
        assert_eq!(active_item.scale, 1.08);
        assert_eq!(active_item.opacity, 1.0);

        // Near lines have reduced opacity
        let prev_item = viewport.iter().find(|i| i.index == 0).unwrap();
        assert_eq!(prev_item.scale, 1.0);
        assert!((prev_item.opacity - 0.65).abs() < 0.01);
    }
}
