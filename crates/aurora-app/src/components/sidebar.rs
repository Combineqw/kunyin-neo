//! Apple HIG Navigation Sidebar Component for KUNYIN Neo Native Client.
//!
//! Features:
//! - 220px fixed optical width
//! - Frosted glass background layer
//! - 36px capsule selection pill with Apple spring curve
//! - System playlists (Trial, Favorites) and user playlists hierarchy

use serde::{Deserialize, Serialize};
use crate::state::AppRoute;
use crate::theme::AppleTheme;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavItem {
    pub route: AppRoute,
    pub title: String,
    pub icon: String,
    pub badge: Option<usize>,
}

pub struct SidebarView {
    pub main_items: Vec<NavItem>,
    pub bottom_items: Vec<NavItem>,
}

impl Default for SidebarView {
    fn default() -> Self {
        Self::new()
    }
}

impl SidebarView {
    pub fn new() -> Self {
        Self {
            main_items: vec![
                NavItem {
                    route: AppRoute::Discover,
                    title: "发现音乐".to_string(),
                    icon: "ri-compass-3-line".to_string(),
                    badge: None,
                },
                NavItem {
                    route: AppRoute::Library,
                    title: "本地曲库".to_string(),
                    icon: "ri-folder-music-line".to_string(),
                    badge: None,
                },
                NavItem {
                    route: AppRoute::Trial,
                    title: "试听列表".to_string(),
                    icon: "ri-disc-line".to_string(),
                    badge: None,
                },
                NavItem {
                    route: AppRoute::Favorites,
                    title: "我的收藏".to_string(),
                    icon: "ri-heart-line".to_string(),
                    badge: None,
                },
            ],
            bottom_items: vec![
                NavItem {
                    route: AppRoute::Settings,
                    title: "音频设置".to_string(),
                    icon: "ri-settings-3-line".to_string(),
                    badge: None,
                },
            ],
        }
    }

    pub fn item_style(&self, item: &NavItem, current_route: &AppRoute, theme: &AppleTheme) -> NavItemStyle {
        let is_active = &item.route == current_route;
        if is_active {
            NavItemStyle {
                background: theme.colors.surface_pill_active.to_css_rgba(),
                text_color: theme.colors.accent_primary.to_css_rgba(),
                font_weight: 600,
                border_radius: theme.metrics.radius_sm,
                height: theme.metrics.nav_capsule_height,
            }
        } else {
            NavItemStyle {
                background: "transparent".to_string(),
                text_color: theme.colors.text_secondary.to_css_rgba(),
                font_weight: 400,
                border_radius: theme.metrics.radius_sm,
                height: theme.metrics.nav_capsule_height,
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavItemStyle {
    pub background: String,
    pub text_color: String,
    pub font_weight: u16,
    pub border_radius: f32,
    pub height: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidebar_generates_hig_compliant_styles() {
        let sidebar = SidebarView::new();
        let theme = AppleTheme::default();
        let active_style = sidebar.item_style(&sidebar.main_items[0], &AppRoute::Discover, &theme);
        assert_eq!(active_style.font_weight, 600);
        assert_eq!(active_style.height, 36.0);

        let inactive_style = sidebar.item_style(&sidebar.main_items[1], &AppRoute::Discover, &theme);
        assert_eq!(inactive_style.font_weight, 400);
        assert_eq!(inactive_style.background, "transparent");
    }
}
