//! Apple HIG Toolbar Component for KUNYIN Neo Native Client.
//!
//! Features:
//! - 52px optical height
//! - Frosted glass translucent header
//! - Apple search pill (border-radius 999px, subtle border highlight)
//! - Theme toggle & native window control buttons

use serde::{Deserialize, Serialize};
use crate::theme::AppleTheme;

pub struct ToolbarView {
    pub search_placeholder: String,
    pub can_go_back: bool,
    pub can_go_forward: bool,
}

impl Default for ToolbarView {
    fn default() -> Self {
        Self {
            search_placeholder: "搜索音乐、歌手、歌单...".to_string(),
            can_go_back: false,
            can_go_forward: false,
        }
    }
}

impl ToolbarView {
    pub fn search_pill_style(&self, theme: &AppleTheme, is_focused: bool) -> SearchPillStyle {
        let border_color = if is_focused {
            theme.colors.accent_primary.to_css_rgba()
        } else {
            theme.colors.border_subtle.to_css_rgba()
        };

        SearchPillStyle {
            background: theme.colors.background_tertiary.to_css_rgba(),
            text_color: theme.colors.text_primary.to_css_rgba(),
            placeholder_color: theme.colors.text_tertiary.to_css_rgba(),
            border_color,
            border_radius: theme.metrics.radius_pill,
            height: 32.0,
            width: 260.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPillStyle {
    pub background: String,
    pub text_color: String,
    pub placeholder_color: String,
    pub border_color: String,
    pub border_radius: f32,
    pub height: f32,
    pub width: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbar_search_pill_style_reflects_focus_state() {
        let toolbar = ToolbarView::default();
        let theme = AppleTheme::default();
        let normal = toolbar.search_pill_style(&theme, false);
        let focused = toolbar.search_pill_style(&theme, true);

        assert_eq!(normal.border_radius, 999.0);
        assert_eq!(focused.border_radius, 999.0);
        assert_ne!(normal.border_color, focused.border_color);
    }
}
