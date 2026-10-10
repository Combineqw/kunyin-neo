//! KUNYIN Neo Pure Rust Native Desktop Application Core.
//!
//! 100% Pure Rust Architecture:
//! - Zero WebView, Zero Electron, Zero Node.js
//! - Direct binding to `aurora-audio` (Symphonia, WASAPI, DSP)
//! - Direct binding to `aurora-player` (State machine)
//! - Direct binding to `aurora-library` (SQLite WAL + FTS5)
//! - Apple Human Interface Guidelines (HIG) visual system

pub mod theme;
pub mod state;
pub mod components;
pub mod window;
pub mod desktop_lyric_window;

pub use theme::{AppleTheme, ThemeMode, AppleColors, AppleMetrics};
pub use state::{AppState, AppRoute, AppTrack, DspSettings};
pub use components::*;
pub use desktop_lyric_window::{DesktopLyricWindowConfig, DesktopLyricWindowState};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutFrame {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopWindowLayout {
    pub window_width: f32,
    pub window_height: f32,
    pub sidebar_frame: LayoutFrame,
    pub toolbar_frame: LayoutFrame,
    pub main_view_frame: LayoutFrame,
    pub player_bar_frame: LayoutFrame,
}

pub struct KunyinApp {
    pub state: AppState,
    pub theme: AppleTheme,
    pub sidebar: SidebarView,
    pub toolbar: ToolbarView,
    pub song_table: SongTableView,
}

impl Default for KunyinApp {
    fn default() -> Self {
        Self::new()
    }
}

impl KunyinApp {
    pub fn new() -> Self {
        Self {
            state: AppState::with_in_memory_library(),
            theme: AppleTheme::default(),
            sidebar: SidebarView::new(),
            toolbar: ToolbarView::default(),
            song_table: SongTableView::default(),
        }
    }

    pub fn compute_layout(&self, window_width: f32, window_height: f32) -> DesktopWindowLayout {
        let sidebar_width = self.theme.metrics.sidebar_width;
        let toolbar_height = self.theme.metrics.toolbar_height;
        let player_bar_height = self.theme.metrics.player_bar_height;

        let content_width = (window_width - sidebar_width).max(0.0);
        let content_height = (window_height - toolbar_height - player_bar_height).max(0.0);

        DesktopWindowLayout {
            window_width,
            window_height,
            sidebar_frame: LayoutFrame {
                x: 0.0,
                y: 0.0,
                width: sidebar_width,
                height: (window_height - player_bar_height).max(0.0),
            },
            toolbar_frame: LayoutFrame {
                x: sidebar_width,
                y: 0.0,
                width: content_width,
                height: toolbar_height,
            },
            main_view_frame: LayoutFrame {
                x: sidebar_width,
                y: toolbar_height,
                width: content_width,
                height: content_height,
            },
            player_bar_frame: LayoutFrame {
                x: 0.0,
                y: window_height - player_bar_height,
                width: window_width,
                height: player_bar_height,
            },
        }
    }

    pub fn run_self_check(&mut self) -> Result<AppDiagnostics, String> {
        let layout = self.compute_layout(1080.0, 720.0);
        if layout.sidebar_frame.width != self.theme.metrics.sidebar_width {
            return Err("Sidebar width does not match metrics".to_string());
        }

        // Test navigation
        self.state.navigate_to(AppRoute::Favorites);
        if self.state.current_route != AppRoute::Favorites {
            return Err("Navigation failed".to_string());
        }

        // Test track load & playback session
        let test_song = AppTrack {
            id: 1,
            source: "local".to_string(),
            title: "Aurora Native Symphony".to_string(),
            artist: "Rust Audio Ensemble".to_string(),
            album: "Pure Native Hi-Res".to_string(),
            duration_ms: 240_000,
            cover_url: None,
            file_path: Some("D:/music/sample.flac".to_string()),
        };
        self.state.play_track(test_song);
        let snapshot = self.state.snapshot();
        if snapshot.status != aurora_player::PlaybackStatus::Playing {
            return Err("Playback engine failed to start".to_string());
        }

        // Test timeline advancement
        self.state.tick(5_000);
        if self.state.snapshot().position_ms != 5_000 {
            return Err("Playback timeline failed to advance".to_string());
        }

        // Test theme toggle
        let initial_mode = self.theme.mode;
        self.theme.toggle_mode();
        if self.theme.mode == initial_mode {
            return Err("Theme toggle failed".to_string());
        }

        Ok(AppDiagnostics {
            app_name: "KUNYIN Neo (Pure Rust)".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            audio_backend: "Symphonia + WASAPI Exclusive / Shared".to_string(),
            database_engine: "SQLite WAL + FTS5 (aurora-library)".to_string(),
            gui_architecture: "100% Pure Rust Native (Zero WebView)".to_string(),
            design_system: "Apple Human Interface Guidelines (SF Pro / 4pt Grid / Squircles)".to_string(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppDiagnostics {
    pub app_name: String,
    pub version: String,
    pub audio_backend: String,
    pub database_engine: String,
    pub gui_architecture: String,
    pub design_system: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kunyin_app_self_check_succeeds() {
        let mut app = KunyinApp::new();
        let diag = app.run_self_check().expect("self check should pass");
        assert_eq!(diag.app_name, "KUNYIN Neo (Pure Rust)");
        assert_eq!(diag.version, "1.1.0");
        assert!(diag.audio_backend.contains("WASAPI"));
    }

    #[test]
    fn layout_partitions_four_zones_accurately() {
        let app = KunyinApp::new();
        let layout = app.compute_layout(1200.0, 800.0);
        assert_eq!(layout.sidebar_frame.width, 220.0);
        assert_eq!(layout.toolbar_frame.height, 52.0);
        assert_eq!(layout.player_bar_frame.height, 80.0);
        assert_eq!(layout.main_view_frame.width, 1200.0 - 220.0);
        assert_eq!(layout.main_view_frame.height, 800.0 - 52.0 - 80.0);
        assert_eq!(layout.player_bar_frame.y, 800.0 - 80.0);
    }
}
