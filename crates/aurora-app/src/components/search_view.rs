//! Apple HIG Native Search View & Provider Multiplexing for KUNYIN Neo.
//!
//! Provides:
//! - Multi-source aggregate search state
//! - Source tabs: 全部 (All), QQ音乐 (QQ), 网易云 (WY), 酷狗 (KG), 酷我 (KW)
//! - Query result filtering & structured formatting

use serde::{Deserialize, Serialize};
use crate::state::AppTrack;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchSourceTab {
    All,
    Qq,
    Wy,
    Kg,
    Kw,
}

impl SearchSourceTab {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::All => "聚合全源",
            Self::Qq => "QQ音乐",
            Self::Wy => "网易云",
            Self::Kg => "酷狗音乐",
            Self::Kw => "酷我音乐",
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Qq => "qq",
            Self::Wy => "wy",
            Self::Kg => "kg",
            Self::Kw => "kw",
        }
    }
}

pub struct SearchViewModel {
    pub current_query: String,
    pub current_tab: SearchSourceTab,
    pub all_results: Vec<AppTrack>,
}

impl SearchViewModel {
    pub const TABS: [SearchSourceTab; 5] = [
        SearchSourceTab::All,
        SearchSourceTab::Qq,
        SearchSourceTab::Wy,
        SearchSourceTab::Kg,
        SearchSourceTab::Kw,
    ];

    pub fn new(query: impl Into<String>, tab: SearchSourceTab, results: Vec<AppTrack>) -> Self {
        Self {
            current_query: query.into(),
            current_tab: tab,
            all_results: results,
        }
    }

    pub fn filtered_results(&self) -> Vec<&AppTrack> {
        match self.current_tab {
            SearchSourceTab::All => self.all_results.iter().collect(),
            tab => {
                let code = tab.code();
                self.all_results
                    .iter()
                    .filter(|t| t.source == code)
                    .collect()
            }
        }
    }

    pub fn result_count_label(&self) -> String {
        let count = self.filtered_results().len();
        if self.current_query.trim().is_empty() {
            "请输入关键词开启多源极速搜索".to_string()
        } else {
            format!("找到 {} 首匹配曲目", count)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_view_filters_tracks_by_selected_source() {
        let tracks = vec![
            AppTrack {
                id: 1,
                source: "qq".to_string(),
                title: "晴天".to_string(),
                artist: "周杰伦".to_string(),
                album: "叶惠美".to_string(),
                duration_ms: 269000,
                cover_url: None,
                file_path: None,
            },
            AppTrack {
                id: 2,
                source: "wy".to_string(),
                title: "稻香".to_string(),
                artist: "周杰伦".to_string(),
                album: "魔杰座".to_string(),
                duration_ms: 223000,
                cover_url: None,
                file_path: None,
            },
            AppTrack {
                id: 3,
                source: "kg".to_string(),
                title: "七里香".to_string(),
                artist: "周杰伦".to_string(),
                album: "七里香".to_string(),
                duration_ms: 299000,
                cover_url: None,
                file_path: None,
            },
        ];

        let model = SearchViewModel::new("周杰伦", SearchSourceTab::All, tracks.clone());
        assert_eq!(model.filtered_results().len(), 3);
        assert_eq!(model.result_count_label(), "找到 3 首匹配曲目");

        let qq_model = SearchViewModel::new("周杰伦", SearchSourceTab::Qq, tracks);
        assert_eq!(qq_model.filtered_results().len(), 1);
        assert_eq!(qq_model.filtered_results()[0].title, "晴天");
    }
}
