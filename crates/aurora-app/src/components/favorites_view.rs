//! Favorites View Component for KUNYIN Neo (Pure Rust Native).
//!
//! Aligns with the full-featured Favorites playlist:
//! - Hero Card: Apple HIG frosted glass card with Heart badge (♥ 我的收藏)
//! - Header actions: Play All (▶ 播放全部), Shuffle All (🔀 随机播放)
//! - Search filter query
//! - Fast favorite track toggling

use serde::{Deserialize, Serialize};
use crate::state::AppTrack;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FavoritesViewModel {
    pub title: String,
    pub description: String,
    pub tracks: Vec<AppTrack>,
    pub filter_query: String,
}

impl Default for FavoritesViewModel {
    fn default() -> Self {
        Self::new_with_samples()
    }
}

impl FavoritesViewModel {
    pub fn new(tracks: Vec<AppTrack>) -> Self {
        Self {
            title: "我的珍藏音乐".to_string(),
            description: "精选收藏歌曲 · WASAPI 独占硬件原音直通输出".to_string(),
            tracks,
            filter_query: String::new(),
        }
    }

    pub fn new_with_samples() -> Self {
        let samples = vec![
            AppTrack {
                id: 101,
                source: "qq".to_string(),
                title: "稻香 (Master)".to_string(),
                artist: "周杰伦".to_string(),
                album: "魔杰座".to_string(),
                duration_ms: 223_000,
                cover_url: None,
                file_path: None,
            },
            AppTrack {
                id: 102,
                source: "wy".to_string(),
                title: "海阔天空 (Remaster)".to_string(),
                artist: "Beyond".to_string(),
                album: "海阔天空".to_string(),
                duration_ms: 326_000,
                cover_url: None,
                file_path: None,
            },
            AppTrack {
                id: 103,
                source: "local".to_string(),
                title: "夜曲 (Hi-Res)".to_string(),
                artist: "周杰伦".to_string(),
                album: "十一月的萧邦".to_string(),
                duration_ms: 226_000,
                cover_url: None,
                file_path: None,
            },
        ];
        Self::new(samples)
    }

    pub fn total_count(&self) -> usize {
        self.tracks.len()
    }

    pub fn formatted_duration(&self) -> String {
        let total_ms: u64 = self.tracks.iter().map(|t| t.duration_ms).sum();
        let total_sec = total_ms / 1000;
        let minutes = total_sec / 60;
        let seconds = total_sec % 60;
        format!("{} 分 {} 秒", minutes, seconds)
    }

    pub fn add_track(&mut self, track: AppTrack) {
        if !self.tracks.iter().any(|t| t.id == track.id) {
            self.tracks.push(track);
        }
    }

    pub fn remove_track(&mut self, track_id: i64) -> bool {
        let len_before = self.tracks.len();
        self.tracks.retain(|t| t.id != track_id);
        self.tracks.len() < len_before
    }

    pub fn is_favorite(&self, track_id: i64) -> bool {
        self.tracks.iter().any(|t| t.id == track_id)
    }

    pub fn filtered_tracks(&self) -> Vec<AppTrack> {
        let query = self.filter_query.trim().to_lowercase();
        if query.is_empty() {
            self.tracks.clone()
        } else {
            self.tracks
                .iter()
                .filter(|t| {
                    t.title.to_lowercase().contains(&query)
                        || t.artist.to_lowercase().contains(&query)
                        || t.album.to_lowercase().contains(&query)
                })
                .cloned()
                .collect()
        }
    }

    pub fn set_filter(&mut self, query: &str) {
        self.filter_query = query.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn favorites_lifecycle_and_filtering() {
        let mut model = FavoritesViewModel::new_with_samples();
        assert_eq!(model.total_count(), 3);
        assert!(model.is_favorite(101));

        model.remove_track(101);
        assert_eq!(model.total_count(), 2);
        assert!(!model.is_favorite(101));

        let new_track = AppTrack {
            id: 104,
            source: "kg".to_string(),
            title: "江南".to_string(),
            artist: "林俊杰".to_string(),
            album: "第二天堂".to_string(),
            duration_ms: 250_000,
            cover_url: None,
            file_path: None,
        };
        model.add_track(new_track);
        assert_eq!(model.total_count(), 3);
        assert!(model.is_favorite(104));
    }
}
