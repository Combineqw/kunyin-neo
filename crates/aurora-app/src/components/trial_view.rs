//! Trial Queue & Playback History View Component for KUNYIN Neo (Pure Rust Native).
//!
//! Aligns with the full-featured Trial/Queue view:
//! - Queue Banner: "试听历史与播放队列"
//! - Action pills: Clear Queue (🗑 清空列表), Save as Playlist (📥 转存为本地歌单), Play Sequence (▶ 顺序播放)
//! - Queue items manipulation: remove, prepend next, reorder

use serde::{Deserialize, Serialize};
use crate::state::AppTrack;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrialQueueViewModel {
    pub title: String,
    pub queue: Vec<AppTrack>,
    pub max_history: usize,
}

impl Default for TrialQueueViewModel {
    fn default() -> Self {
        Self::new_with_samples()
    }
}

impl TrialQueueViewModel {
    pub fn new(queue: Vec<AppTrack>) -> Self {
        Self {
            title: "试听列表与临时队列".to_string(),
            queue,
            max_history: 100,
        }
    }

    pub fn new_with_samples() -> Self {
        let samples = vec![
            AppTrack {
                id: 201,
                source: "qq".to_string(),
                title: "反方向的钟 (Trial)".to_string(),
                artist: "周杰伦".to_string(),
                album: "Jay".to_string(),
                duration_ms: 228_000,
                cover_url: None,
                file_path: None,
            },
            AppTrack {
                id: 202,
                source: "wy".to_string(),
                title: "枫 (Hi-Res)".to_string(),
                artist: "周杰伦".to_string(),
                album: "十一月的萧邦".to_string(),
                duration_ms: 275_000,
                cover_url: None,
                file_path: None,
            },
        ];
        Self::new(samples)
    }

    pub fn count(&self) -> usize {
        self.queue.len()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }

    pub fn push_track(&mut self, track: AppTrack) {
        // Avoid duplicate immediate enqueue
        self.queue.retain(|t| t.id != track.id);
        self.queue.push(track);
        if self.queue.len() > self.max_history {
            self.queue.remove(0);
        }
    }

    pub fn play_next(&mut self, track: AppTrack) {
        self.queue.retain(|t| t.id != track.id);
        self.queue.insert(0, track);
    }

    pub fn remove_at(&mut self, index: usize) -> Option<AppTrack> {
        if index < self.queue.len() {
            Some(self.queue.remove(index))
        } else {
            None
        }
    }

    pub fn formatted_duration(&self) -> String {
        let total_ms: u64 = self.queue.iter().map(|t| t.duration_ms).sum();
        let total_sec = total_ms / 1000;
        let minutes = total_sec / 60;
        let seconds = total_sec % 60;
        format!("{} 分 {} 秒", minutes, seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trial_queue_operations() {
        let mut model = TrialQueueViewModel::new_with_samples();
        assert_eq!(model.count(), 2);

        let next_track = AppTrack {
            id: 203,
            source: "kw".to_string(),
            title: "安静".to_string(),
            artist: "周杰伦".to_string(),
            album: "范特西".to_string(),
            duration_ms: 334_000,
            cover_url: None,
            file_path: None,
        };

        model.play_next(next_track.clone());
        assert_eq!(model.count(), 3);
        assert_eq!(model.queue[0].id, 203);

        model.remove_at(0);
        assert_eq!(model.count(), 2);

        model.clear();
        assert_eq!(model.count(), 0);
    }
}
