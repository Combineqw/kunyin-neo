//! Library View Component for KUNYIN Neo (Pure Rust Native).
//!
//! Aligns 1:1 with the full-featured LocalPlaylistView:
//! - 4 Category tabs: Tracks (曲目), Albums (专辑), Artists (歌手), Folders (文件夹)
//! - Library stats: Total tracks, folders count, lossless ratio, formatted duration
//! - Sorting: DateAdded, Title, Artist, PlayCount
//! - In-library search query filtering
//! - Quick action buttons: Scan Folder, Play All, Shuffle All

use serde::{Deserialize, Serialize};
use crate::state::AppTrack;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LibraryCategoryTab {
    Tracks,
    Albums,
    Artists,
    Folders,
}

impl Default for LibraryCategoryTab {
    fn default() -> Self {
        Self::Tracks
    }
}

impl LibraryCategoryTab {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Tracks => "全部曲目",
            Self::Albums => "专辑分类",
            Self::Artists => "歌手分类",
            Self::Folders => "本地文件夹",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LibrarySortOrder {
    DateAdded,
    Title,
    Artist,
    PlayCount,
}

impl Default for LibrarySortOrder {
    fn default() -> Self {
        Self::DateAdded
    }
}

impl LibrarySortOrder {
    pub fn label(&self) -> &'static str {
        match self {
            Self::DateAdded => "添加时间",
            Self::Title => "歌曲名称",
            Self::Artist => "歌手名字",
            Self::PlayCount => "播放频次",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryStats {
    pub total_tracks: usize,
    pub total_folders: usize,
    pub lossless_percent: f32,
    pub total_duration_ms: u64,
}

impl LibraryStats {
    pub fn formatted_duration(&self) -> String {
        let total_sec = self.total_duration_ms / 1000;
        let hours = total_sec / 3600;
        let minutes = (total_sec % 3600) / 60;
        if hours > 0 {
            format!("{} 小时 {} 分钟", hours, minutes)
        } else {
            format!("{} 分钟", minutes)
        }
    }

    pub fn formatted_lossless(&self) -> String {
        format!("{:.1}% 无损率", self.lossless_percent)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryAlbumSummary {
    pub name: String,
    pub artist: String,
    pub track_count: usize,
    pub year: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryArtistSummary {
    pub name: String,
    pub track_count: usize,
    pub album_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryFolderSummary {
    pub path: String,
    pub track_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryViewModel {
    pub current_tab: LibraryCategoryTab,
    pub sort_order: LibrarySortOrder,
    pub search_query: String,
    pub tracks: Vec<AppTrack>,
    pub albums: Vec<LibraryAlbumSummary>,
    pub artists: Vec<LibraryArtistSummary>,
    pub folders: Vec<LibraryFolderSummary>,
}

impl Default for LibraryViewModel {
    fn default() -> Self {
        Self::new_with_samples()
    }
}

impl LibraryViewModel {
    pub fn new(tracks: Vec<AppTrack>) -> Self {
        let mut model = Self {
            current_tab: LibraryCategoryTab::Tracks,
            sort_order: LibrarySortOrder::DateAdded,
            search_query: String::new(),
            tracks,
            albums: Vec::new(),
            artists: Vec::new(),
            folders: Vec::new(),
        };
        model.recompute_summaries();
        model
    }

    pub fn new_with_samples() -> Self {
        let samples = vec![
            AppTrack {
                id: 1,
                source: "local".to_string(),
                title: "晴天 (Lossless)".to_string(),
                artist: "周杰伦".to_string(),
                album: "叶惠美".to_string(),
                duration_ms: 269_000,
                cover_url: None,
                file_path: Some("D:/Music/Jay/晴天.flac".to_string()),
            },
            AppTrack {
                id: 2,
                source: "local".to_string(),
                title: "七里香 (Hi-Res)".to_string(),
                artist: "周杰伦".to_string(),
                album: "七里香".to_string(),
                duration_ms: 299_000,
                cover_url: None,
                file_path: Some("D:/Music/Jay/七里香.flac".to_string()),
            },
            AppTrack {
                id: 3,
                source: "local".to_string(),
                title: "起风了 (Master)".to_string(),
                artist: "买辣椒也用券".to_string(),
                album: "起风了".to_string(),
                duration_ms: 325_000,
                cover_url: None,
                file_path: Some("D:/Music/Pop/起风了.wav".to_string()),
            },
            AppTrack {
                id: 4,
                source: "local".to_string(),
                title: "光辉岁月 (Remaster)".to_string(),
                artist: "Beyond".to_string(),
                album: "命运派对".to_string(),
                duration_ms: 302_000,
                cover_url: None,
                file_path: Some("D:/Music/Rock/光辉岁月.flac".to_string()),
            },
        ];
        Self::new(samples)
    }

    pub fn recompute_summaries(&mut self) {
        use std::collections::{BTreeMap, BTreeSet};

        let mut album_map: BTreeMap<String, (String, usize, Option<String>)> = BTreeMap::new();
        let mut artist_map: BTreeMap<String, (usize, BTreeSet<String>)> = BTreeMap::new();
        let mut folder_map: BTreeMap<String, usize> = BTreeMap::new();

        for track in &self.tracks {
            // Album
            let album_entry = album_map.entry(track.album.clone()).or_insert((track.artist.clone(), 0, None));
            album_entry.1 += 1;

            // Artist
            let artist_entry = artist_map.entry(track.artist.clone()).or_insert((0, BTreeSet::new()));
            artist_entry.0 += 1;
            artist_entry.1.insert(track.album.clone());

            // Folder
            if let Some(path) = &track.file_path {
                let parent = if let Some(idx) = path.rfind(['/', '\\']) {
                    &path[..idx]
                } else {
                    path.as_str()
                };
                *folder_map.entry(parent.to_string()).or_insert(0) += 1;
            }
        }

        self.albums = album_map
            .into_iter()
            .map(|(name, (artist, track_count, year))| LibraryAlbumSummary {
                name,
                artist,
                track_count,
                year,
            })
            .collect();

        self.artists = artist_map
            .into_iter()
            .map(|(name, (track_count, albums))| LibraryArtistSummary {
                name,
                track_count,
                album_count: albums.len(),
            })
            .collect();

        self.folders = folder_map
            .into_iter()
            .map(|(path, track_count)| LibraryFolderSummary { path, track_count })
            .collect();
    }

    pub fn compute_stats(&self) -> LibraryStats {
        let total_tracks = self.tracks.len();
        let total_folders = self.folders.len();
        let total_duration_ms: u64 = self.tracks.iter().map(|t| t.duration_ms).sum();
        let lossless_count = self
            .tracks
            .iter()
            .filter(|t| {
                t.title.contains("Lossless")
                    || t.title.contains("Hi-Res")
                    || t.title.contains("Master")
                    || t.file_path.as_ref().map(|p| p.ends_with(".flac") || p.ends_with(".wav")).unwrap_or(false)
            })
            .count();

        let lossless_percent = if total_tracks > 0 {
            (lossless_count as f32 / total_tracks as f32) * 100.0
        } else {
            0.0
        };

        LibraryStats {
            total_tracks,
            total_folders,
            lossless_percent,
            total_duration_ms,
        }
    }

    pub fn filtered_tracks(&self) -> Vec<AppTrack> {
        let query = self.search_query.trim().to_lowercase();
        let mut list: Vec<AppTrack> = if query.is_empty() {
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
        };

        match self.sort_order {
            LibrarySortOrder::DateAdded => {
                // Keep insertion order or id
                list.sort_by_key(|t| t.id);
            }
            LibrarySortOrder::Title => {
                list.sort_by(|a, b| a.title.cmp(&b.title));
            }
            LibrarySortOrder::Artist => {
                list.sort_by(|a, b| a.artist.cmp(&b.artist));
            }
            LibrarySortOrder::PlayCount => {
                // Longer songs as simulated proxy or reverse id
                list.sort_by(|a, b| b.duration_ms.cmp(&a.duration_ms));
            }
        }

        list
    }

    pub fn set_tab(&mut self, tab: LibraryCategoryTab) {
        self.current_tab = tab;
    }

    pub fn set_sort(&mut self, sort: LibrarySortOrder) {
        self.sort_order = sort;
    }

    pub fn set_search_query(&mut self, query: &str) {
        self.search_query = query.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_stats_and_summaries_calculation() {
        let model = LibraryViewModel::new_with_samples();
        let stats = model.compute_stats();
        assert_eq!(stats.total_tracks, 4);
        assert!(stats.lossless_percent >= 75.0);
        assert_eq!(model.albums.len(), 4);
        assert_eq!(model.artists.len(), 3);
        assert_eq!(model.folders.len(), 3);
    }

    #[test]
    fn library_filtering_and_sorting() {
        let mut model = LibraryViewModel::new_with_samples();
        model.set_search_query("周杰伦");
        let filtered = model.filtered_tracks();
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|t| t.artist == "周杰伦"));

        model.set_search_query("");
        model.set_sort(LibrarySortOrder::Title);
        let sorted = model.filtered_tracks();
        assert_eq!(sorted[0].title, "七里香 (Hi-Res)");
    }
}
