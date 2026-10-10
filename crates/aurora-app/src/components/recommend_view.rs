//! Apple HIG Native Recommend & Curated Discovery View for KUNYIN Neo.
//!
//! 100% Pure Rust Native Component:
//! - Daily Recommendations Hero Banner (每日推荐与心动模式)
//! - Curated Playlists Grid (推荐歌单) with squircle covers & play count badges
//! - New Releases Shelf (新歌速递)
//! - Local-first privacy-preserving Heartbeat recommendation state

use serde::{Deserialize, Serialize};
use crate::state::AppTrack;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendHeroCard {
    pub eyebrow: String,
    pub title: String,
    pub description: String,
    pub reason: String,
    pub featured_track: Option<AppTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuratedPlaylistCard {
    pub id: i64,
    pub title: String,
    pub creator: String,
    pub play_count: u64,
    pub track_count: usize,
    pub category: String,
}

impl CuratedPlaylistCard {
    pub fn formatted_play_count(&self) -> String {
        if self.play_count >= 100_000_000 {
            format!("{:.1}亿", self.play_count as f64 / 100_000_000.0)
        } else if self.play_count >= 10_000 {
            format!("{:.1}万", self.play_count as f64 / 10_000.0)
        } else {
            self.play_count.to_string()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewReleaseCard {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub source: String,
    pub badge: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendViewModel {
    pub hero: RecommendHeroCard,
    pub curated_playlists: Vec<CuratedPlaylistCard>,
    pub new_releases: Vec<NewReleaseCard>,
    pub is_heartbeat_active: bool,
}

impl RecommendViewModel {
    pub fn default_curated() -> Self {
        Self {
            hero: RecommendHeroCard {
                eyebrow: "FOR YOU · LOCAL ONLY".to_string(),
                title: "每日精选与心动模式".to_string(),
                description: "基于本地播放习惯与收藏偏好生成，100% 离线保护隐私".to_string(),
                reason: "因为你经常完整聆听 Hi-Res 流行与纯音乐".to_string(),
                featured_track: Some(AppTrack {
                    id: 101,
                    source: "local".to_string(),
                    title: "晴天 (Hi-Res Master)".to_string(),
                    artist: "周杰伦".to_string(),
                    album: "叶惠美".to_string(),
                    duration_ms: 269_000,
                    cover_url: None,
                    file_path: None,
                }),
            },
            curated_playlists: vec![
                CuratedPlaylistCard {
                    id: 201,
                    title: "华语流行黄金时代精选".to_string(),
                    creator: "昆音雷达".to_string(),
                    play_count: 1_280_000,
                    track_count: 50,
                    category: "流行".to_string(),
                },
                CuratedPlaylistCard {
                    id: 202,
                    title: "24-bit 96kHz 录音室母带试音碟".to_string(),
                    creator: "无损发烧友".to_string(),
                    play_count: 580_000,
                    track_count: 32,
                    category: "Hi-Res".to_string(),
                },
                CuratedPlaylistCard {
                    id: 203,
                    title: "深夜专注 · 纯净环境器乐与爵士".to_string(),
                    creator: "声学实验室".to_string(),
                    play_count: 340_000,
                    track_count: 45,
                    category: "轻音乐".to_string(),
                },
                CuratedPlaylistCard {
                    id: 204,
                    title: "摇滚先锋 · 经典模拟声场重温".to_string(),
                    creator: "黑胶俱乐部".to_string(),
                    play_count: 125_000,
                    track_count: 40,
                    category: "摇滚".to_string(),
                },
            ],
            new_releases: vec![
                NewReleaseCard {
                    id: 301,
                    title: "如初之光".to_string(),
                    artist: "袁娅维".to_string(),
                    album: "如初之光".to_string(),
                    source: "qq".to_string(),
                    badge: "Hi-Res".to_string(),
                    duration_ms: 235_000,
                },
                NewReleaseCard {
                    id: 302,
                    title: "向云端 (Acoustic)".to_string(),
                    artist: "小霞".to_string(),
                    album: "向云端".to_string(),
                    source: "wy".to_string(),
                    badge: "FLAC".to_string(),
                    duration_ms: 248_000,
                },
                NewReleaseCard {
                    id: 303,
                    title: "时光如愿".to_string(),
                    artist: "周深".to_string(),
                    album: "时光如愿".to_string(),
                    source: "kg".to_string(),
                    badge: "SQ".to_string(),
                    duration_ms: 215_000,
                },
            ],
            is_heartbeat_active: false,
        }
    }

    pub fn toggle_heartbeat(&mut self) -> bool {
        self.is_heartbeat_active = !self.is_heartbeat_active;
        self.is_heartbeat_active
    }

    pub fn find_playlist(&self, id: i64) -> Option<&CuratedPlaylistCard> {
        self.curated_playlists.iter().find(|p| p.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_play_count_accurately() {
        let card1 = CuratedPlaylistCard {
            id: 1,
            title: "Test".to_string(),
            creator: "Admin".to_string(),
            play_count: 1_280_000,
            track_count: 20,
            category: "Pop".to_string(),
        };
        assert_eq!(card1.formatted_play_count(), "128.0万");

        let card2 = CuratedPlaylistCard {
            id: 2,
            title: "Test 2".to_string(),
            creator: "Admin".to_string(),
            play_count: 8_500,
            track_count: 10,
            category: "Rock".to_string(),
        };
        assert_eq!(card2.formatted_play_count(), "8500");

        let card3 = CuratedPlaylistCard {
            id: 3,
            title: "Test 3".to_string(),
            creator: "Admin".to_string(),
            play_count: 250_000_000,
            track_count: 100,
            category: "Classic".to_string(),
        };
        assert_eq!(card3.formatted_play_count(), "2.5亿");
    }

    #[test]
    fn recommend_model_toggles_heartbeat_and_finds_playlist() {
        let mut model = RecommendViewModel::default_curated();
        assert!(!model.is_heartbeat_active);

        assert!(model.toggle_heartbeat());
        assert!(model.is_heartbeat_active);
        assert!(!model.toggle_heartbeat());
        assert!(!model.is_heartbeat_active);

        let pl = model.find_playlist(202).expect("playlist 202 must exist");
        assert!(pl.title.contains("24-bit 96kHz"));
        assert_eq!(pl.category, "Hi-Res");
    }
}
