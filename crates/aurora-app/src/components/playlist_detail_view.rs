//! Apple HIG Native Playlist Detail View Component for KUNYIN Neo.
//!
//! Provides:
//! - Playlist Header with squircle cover, title, author, description, tags
//! - Action bar ("▶ 全部播放", "♡ 收藏歌单", "⬇ 下载全部")
//! - Track list metadata aggregation and in-playlist filtering

use serde::{Deserialize, Serialize};
use crate::state::AppTrack;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistDetailViewModel {
    pub id: i64,
    pub title: String,
    pub creator: String,
    pub description: String,
    pub tags: Vec<String>,
    pub tracks: Vec<AppTrack>,
    pub is_favorite: bool,
}

impl PlaylistDetailViewModel {
    pub fn new(
        id: i64,
        title: impl Into<String>,
        creator: impl Into<String>,
        description: impl Into<String>,
        tags: Vec<String>,
        tracks: Vec<AppTrack>,
    ) -> Self {
        Self {
            id,
            title: title.into(),
            creator: creator.into(),
            description: description.into(),
            tags,
            tracks,
            is_favorite: false,
        }
    }

    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    pub fn total_duration_ms(&self) -> u64 {
        self.tracks.iter().map(|t| t.duration_ms).sum()
    }

    pub fn formatted_total_duration(&self) -> String {
        let total_sec = self.total_duration_ms() / 1000;
        let hours = total_sec / 3600;
        let mins = (total_sec % 3600) / 60;
        if hours > 0 {
            format!("{} 小时 {} 分钟", hours, mins)
        } else {
            format!("{} 分钟", mins)
        }
    }

    pub fn toggle_favorite(&mut self) -> bool {
        self.is_favorite = !self.is_favorite;
        self.is_favorite
    }

    pub fn filter_tracks(&self, keyword: &str) -> Vec<&AppTrack> {
        let kw = keyword.trim().to_lowercase();
        if kw.is_empty() {
            return self.tracks.iter().collect();
        }
        self.tracks
            .iter()
            .filter(|t| {
                t.title.to_lowercase().contains(&kw)
                    || t.artist.to_lowercase().contains(&kw)
                    || t.album.to_lowercase().contains(&kw)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playlist_detail_aggregates_duration_and_filters_tracks() {
        let tracks = vec![
            AppTrack {
                id: 1,
                source: "qq".to_string(),
                title: "晴天".to_string(),
                artist: "周杰伦".to_string(),
                album: "叶惠美".to_string(),
                duration_ms: 269_000,
                cover_url: None,
                file_path: None,
            },
            AppTrack {
                id: 2,
                source: "qq".to_string(),
                title: "一路向北".to_string(),
                artist: "周杰伦".to_string(),
                album: "十一月的萧邦".to_string(),
                duration_ms: 300_000,
                cover_url: None,
                file_path: None,
            },
            AppTrack {
                id: 3,
                source: "wy".to_string(),
                title: "江南".to_string(),
                artist: "林俊杰".to_string(),
                album: "第二天堂".to_string(),
                duration_ms: 250_000,
                cover_url: None,
                file_path: None,
            },
        ];

        let mut vm = PlaylistDetailViewModel::new(
            100,
            "经典流行精选",
            "昆音官方",
            "跨时代华语流行金曲大盘点",
            vec!["华语".to_string(), "流行".to_string()],
            tracks,
        );

        assert_eq!(vm.track_count(), 3);
        assert_eq!(vm.total_duration_ms(), 819_000);
        assert_eq!(vm.formatted_total_duration(), "13 分钟");

        // Filter
        let jj_tracks = vm.filter_tracks("林俊杰");
        assert_eq!(jj_tracks.len(), 1);
        assert_eq!(jj_tracks[0].title, "江南");

        // Toggle favorite
        assert!(!vm.is_favorite);
        assert!(vm.toggle_favorite());
        assert!(vm.is_favorite);
    }
}
