//! Apple HIG Song Table Component for KUNYIN Neo Native Client.
//!
//! Features:
//! - 48px row height
//! - Virtualized viewport calculation
//! - Subtle hover & selected states
//! - Optical alignment with 4pt grid

use serde::{Deserialize, Serialize};
use crate::theme::AppleTheme;

pub struct SongTableView {
    pub row_height: f32,
}

impl Default for SongTableView {
    fn default() -> Self {
        Self { row_height: 48.0 }
    }
}

impl SongTableView {
    pub fn compute_visible_range(&self, scroll_y: f32, viewport_height: f32, total_items: usize) -> (usize, usize) {
        if total_items == 0 || viewport_height <= 0.0 {
            return (0, 0);
        }
        let start = (scroll_y / self.row_height).floor() as usize;
        let visible_count = (viewport_height / self.row_height).ceil() as usize + 2; // +2 for buffer
        let end = (start + visible_count).min(total_items);
        (start.min(total_items), end)
    }

    pub fn row_style(&self, theme: &AppleTheme, is_playing: bool, is_hovered: bool) -> SongRowStyle {
        let background = if is_playing {
            theme.colors.surface_pill_active.to_css_rgba()
        } else if is_hovered {
            theme.colors.background_secondary.to_css_rgba()
        } else {
            "transparent".to_string()
        };

        let title_color = if is_playing {
            theme.colors.accent_primary.to_css_rgba()
        } else {
            theme.colors.text_primary.to_css_rgba()
        };

        SongRowStyle {
            background,
            title_color,
            artist_color: theme.colors.text_secondary.to_css_rgba(),
            duration_color: theme.colors.text_tertiary.to_css_rgba(),
            height: self.row_height,
            border_radius: theme.metrics.radius_sm,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongRowStyle {
    pub background: String,
    pub title_color: String,
    pub artist_color: String,
    pub duration_color: String,
    pub height: f32,
    pub border_radius: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtualized_visible_range_clamps_correctly() {
        let view = SongTableView::default();
        let (start, end) = view.compute_visible_range(100.0, 480.0, 100);
        assert_eq!(start, 2); // 100 / 48 = 2.08 -> floor 2
        assert_eq!(end, 14); // 2 + 10 + 2 buffer = 14

        let (empty_start, empty_end) = view.compute_visible_range(0.0, 400.0, 0);
        assert_eq!((empty_start, empty_end), (0, 0));
    }
}
