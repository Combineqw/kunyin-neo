//! Reactive Application State & Native Session Orchestration for KUNYIN Neo.
//!
//! Directly unifies:
//! - `aurora-player`: Deterministic state machine timeline
//! - `aurora-library`: SQLite WAL + FTS5 local music storage
//! - `aurora-audio`: WASAPI output capabilities and 10-band DSP controls

use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

use aurora_library::{Library, Playlist, SongRecord};
use aurora_player::{PlaybackSession, PlaybackSnapshot, PlaybackStatus};
use aurora_core::lyrics::{find_active_line, parse_lrc, LyricLine, LyricLineMatch};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppRoute {
    Discover,
    Library,
    Trial,
    Favorites,
    Playlist(i64),
    Settings,
    Lyrics,
}

impl Default for AppRoute {
    fn default() -> Self {
        Self::Discover
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppTrack {
    pub id: i64,
    pub source: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    pub cover_url: Option<String>,
    pub file_path: Option<String>,
}

impl AppTrack {
    pub fn from_song_record(record: &SongRecord) -> Self {
        let value: serde_json::Value = serde_json::from_str(&record.song_json).unwrap_or_default();
        let title = value.get("title").and_then(|v| v.as_str()).unwrap_or("未知曲目").to_string();
        let artist = value.get("artist").and_then(|v| v.as_str()).unwrap_or("未知歌手").to_string();
        let album = value.get("album").and_then(|v| v.as_str()).unwrap_or("未知专辑").to_string();
        let duration_ms = value.get("duration").and_then(|v| v.as_u64()).unwrap_or(0);
        let cover_url = value.get("cover").and_then(|v| v.as_str()).map(str::to_string);
        let file_path = value.get("path").and_then(|v| v.as_str()).map(str::to_string);

        Self {
            id: record.id,
            source: record.source.clone(),
            title,
            artist,
            album,
            duration_ms,
            cover_url,
            file_path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DspSettings {
    pub enabled: bool,
    pub bands: [f32; 10], // dB gains from 31Hz to 16kHz
    pub replay_gain_db: f32,
    pub exclusive_mode: bool,
    pub target_device: Option<String>,
}

impl Default for DspSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            bands: [0.0; 10],
            replay_gain_db: 0.0,
            exclusive_mode: false,
            target_device: None,
        }
    }
}

pub struct AppState {
    pub current_route: AppRoute,
    pub current_track: Option<AppTrack>,
    pub queue: Vec<AppTrack>,
    pub queue_index: usize,
    pub player_session: PlaybackSession,
    pub volume: f32,
    pub muted: bool,
    pub library: Option<Arc<Mutex<Library>>>,
    pub playlists: Vec<Playlist>,
    pub current_tracks: Vec<AppTrack>,
    pub search_query: String,
    pub search_results: Vec<AppTrack>,
    pub dsp: DspSettings,
    pub lyrics_lines: Vec<LyricLine>,
    pub active_lyric: Option<LyricLineMatch>,
    pub desktop_lyric_visible: bool,
    pub desktop_lyric_opacity: f32,
    pub desktop_lyric_locked: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_route: AppRoute::Discover,
            current_track: None,
            queue: Vec::new(),
            queue_index: 0,
            player_session: PlaybackSession::default(),
            volume: 0.85,
            muted: false,
            library: None,
            playlists: Vec::new(),
            current_tracks: Vec::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            dsp: DspSettings::default(),
            lyrics_lines: Vec::new(),
            active_lyric: None,
            desktop_lyric_visible: false,
            desktop_lyric_opacity: 0.72,
            desktop_lyric_locked: false,
        }
    }

    pub fn with_in_memory_library() -> Self {
        let mut state = Self::new();
        if let Ok(lib) = Library::open_in_memory() {
            let playlists = lib.list_playlists().unwrap_or_default();
            state.library = Some(Arc::new(Mutex::new(lib)));
            state.playlists = playlists;
        }
        state
    }

    pub fn navigate_to(&mut self, route: AppRoute) {
        self.current_route = route;
        self.refresh_route_data();
    }

    pub fn refresh_route_data(&mut self) {
        let Some(lib_arc) = &self.library else { return };
        let Ok(lib) = lib_arc.lock() else { return };

        match self.current_route {
            AppRoute::Trial => {
                if let Ok(Some(trial_id)) = lib.system_playlist_id(aurora_library::SYSTEM_KIND_TRIAL) {
                    if let Ok(records) = lib.query_playlist_songs(trial_id) {
                        self.current_tracks = records.iter().map(AppTrack::from_song_record).collect();
                    }
                }
            }
            AppRoute::Favorites => {
                if let Ok(Some(fav_id)) = lib.system_playlist_id(aurora_library::SYSTEM_KIND_FAVORITES) {
                    if let Ok(records) = lib.query_playlist_songs(fav_id) {
                        self.current_tracks = records.iter().map(AppTrack::from_song_record).collect();
                    }
                }
            }
            AppRoute::Playlist(id) => {
                if let Ok(records) = lib.query_playlist_songs(id) {
                    self.current_tracks = records.iter().map(AppTrack::from_song_record).collect();
                }
            }
            _ => {}
        }
        if let Ok(playlists) = lib.list_playlists() {
            self.playlists = playlists;
        }
    }

    pub fn search(&mut self, query: &str) {
        self.search_query = query.to_string();
        if query.trim().is_empty() {
            self.search_results.clear();
            return;
        }
        let Some(lib_arc) = &self.library else { return };
        let Ok(lib) = lib_arc.lock() else { return };
        if let Ok(records) = lib.search_songs(query, 50, 0) {
            self.search_results = records.iter().map(AppTrack::from_song_record).collect();
        }
    }

    pub fn play_track(&mut self, track: AppTrack) {
        let duration = track.duration_ms;
        let track_id = format!("{}:{}", track.source, track.id);
        self.current_track = Some(track);
        self.player_session.load(track_id, duration);
        self.player_session.play();
    }

    pub fn toggle_play_pause(&mut self) -> PlaybackStatus {
        let status = self.player_session.snapshot().status;
        match status {
            PlaybackStatus::Playing => {
                self.player_session.pause();
                PlaybackStatus::Paused
            }
            PlaybackStatus::Paused | PlaybackStatus::Idle | PlaybackStatus::Ended => {
                if self.current_track.is_some() {
                    self.player_session.play();
                    PlaybackStatus::Playing
                } else {
                    PlaybackStatus::Idle
                }
            }
        }
    }

    pub fn seek_to(&mut self, position_ms: u64) -> PlaybackSnapshot {
        let snap = self.player_session.seek(position_ms);
        if !self.lyrics_lines.is_empty() {
            self.active_lyric = find_active_line(&self.lyrics_lines, snap.position_ms);
        }
        snap
    }

    pub fn tick(&mut self, elapsed_ms: u64) -> PlaybackSnapshot {
        let snap = self.player_session.tick(elapsed_ms);
        if !self.lyrics_lines.is_empty() {
            self.active_lyric = find_active_line(&self.lyrics_lines, snap.position_ms);
        }
        snap
    }

    pub fn load_lyrics(&mut self, lrc_text: &str) {
        let parsed = parse_lrc(lrc_text);
        self.lyrics_lines = parsed;
        if !self.lyrics_lines.is_empty() {
            let pos_ms = self.player_session.snapshot().position_ms;
            self.active_lyric = find_active_line(&self.lyrics_lines, pos_ms);
        } else {
            self.active_lyric = None;
        }
    }

    pub fn toggle_desktop_lyric(&mut self) -> bool {
        self.desktop_lyric_visible = !self.desktop_lyric_visible;
        self.desktop_lyric_visible
    }

    pub fn set_desktop_lyric_opacity(&mut self, opacity: f32) {
        self.desktop_lyric_opacity = opacity.clamp(0.0, 1.0);
    }

    pub fn set_desktop_lyric_locked(&mut self, locked: bool) {
        self.desktop_lyric_locked = locked;
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    pub fn toggle_mute(&mut self) -> bool {
        self.muted = !self.muted;
        self.muted
    }

    pub fn snapshot(&self) -> PlaybackSnapshot {
        self.player_session.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_state_lyrics_and_desktop_lyric_toggling() {
        let mut state = AppState::new();
        assert!(!state.desktop_lyric_visible);

        state.toggle_desktop_lyric();
        assert!(state.desktop_lyric_visible);

        state.set_desktop_lyric_opacity(0.85);
        assert_eq!(state.desktop_lyric_opacity, 0.85);

        let sample_lrc = "[00:00.00]前奏\n[00:05.00]第一句主歌\n[00:10.00]第二句副歌";
        state.load_lyrics(sample_lrc);
        assert_eq!(state.lyrics_lines.len(), 3);

        // Advance playback
        state.player_session.load("test:1".to_string(), 30000);
        state.player_session.play();
        state.tick(6000); // at 6s

        let active = state.active_lyric.as_ref().expect("active lyric must exist at 6s");
        assert_eq!(active.text, "第一句主歌");
        assert_eq!(active.index, 1);
    }

    #[test]
    fn app_state_navigation_and_playback_session() {
        let mut state = AppState::with_in_memory_library();
        assert_eq!(state.current_route, AppRoute::Discover);

        state.navigate_to(AppRoute::Favorites);
        assert_eq!(state.current_route, AppRoute::Favorites);

        let test_track = AppTrack {
            id: 1001,
            source: "local".to_string(),
            title: "Apple HIG Melody".to_string(),
            artist: "Aurora Sound".to_string(),
            album: "Native Soundscapes".to_string(),
            duration_ms: 180_000,
            cover_url: None,
            file_path: Some("C:/music/track.flac".to_string()),
        };

        state.play_track(test_track.clone());
        assert_eq!(state.snapshot().status, PlaybackStatus::Playing);
        assert_eq!(state.snapshot().duration_ms, 180_000);

        state.tick(10_000);
        assert_eq!(state.snapshot().position_ms, 10_000);

        let status = state.toggle_play_pause();
        assert_eq!(status, PlaybackStatus::Paused);
    }
}
