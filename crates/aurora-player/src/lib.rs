//! Host-neutral playback session state.
//!
//! This crate deliberately does not decode audio or talk to an output device.
//! It owns the deterministic load/play/pause/seek/tick contract so each host
//! can drive the same timeline while the audio backend migrates separately.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackStatus {
    Idle,
    Paused,
    Playing,
    Ended,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSnapshot {
    pub track_id: Option<String>,
    pub status: PlaybackStatus,
    pub position_ms: u64,
    pub duration_ms: u64,
}

impl Default for PlaybackSnapshot {
    fn default() -> Self {
        Self {
            track_id: None,
            status: PlaybackStatus::Idle,
            position_ms: 0,
            duration_ms: 0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PlaybackSession {
    snapshot: PlaybackSnapshot,
}

impl PlaybackSession {
    pub fn snapshot(&self) -> PlaybackSnapshot {
        self.snapshot.clone()
    }

    pub fn load(&mut self, track_id: impl Into<String>, duration_ms: u64) -> PlaybackSnapshot {
        self.snapshot = PlaybackSnapshot {
            track_id: Some(track_id.into()),
            status: PlaybackStatus::Paused,
            position_ms: 0,
            duration_ms,
        };
        self.snapshot()
    }

    pub fn play(&mut self) -> PlaybackSnapshot {
        if self.snapshot.track_id.is_some() {
            if self.snapshot.position_ms >= self.snapshot.duration_ms {
                self.snapshot.position_ms = 0;
            }
            self.snapshot.status = PlaybackStatus::Playing;
        }
        self.snapshot()
    }

    pub fn pause(&mut self) -> PlaybackSnapshot {
        if self.snapshot.status == PlaybackStatus::Playing {
            self.snapshot.status = PlaybackStatus::Paused;
        }
        self.snapshot()
    }

    pub fn seek(&mut self, position_ms: u64) -> PlaybackSnapshot {
        self.snapshot.position_ms = position_ms.min(self.snapshot.duration_ms);
        self.snapshot.status = if self.snapshot.track_id.is_none() {
            PlaybackStatus::Idle
        } else if self.snapshot.position_ms >= self.snapshot.duration_ms {
            PlaybackStatus::Ended
        } else if self.snapshot.status == PlaybackStatus::Ended {
            PlaybackStatus::Paused
        } else {
            self.snapshot.status
        };
        self.snapshot()
    }

    pub fn tick(&mut self, elapsed_ms: u64) -> PlaybackSnapshot {
        if self.snapshot.status != PlaybackStatus::Playing {
            return self.snapshot();
        }
        self.snapshot.position_ms = self
            .snapshot
            .position_ms
            .saturating_add(elapsed_ms)
            .min(self.snapshot.duration_ms);
        if self.snapshot.position_ms >= self.snapshot.duration_ms {
            self.snapshot.status = PlaybackStatus::Ended;
        }
        self.snapshot()
    }

    pub fn stop(&mut self) -> PlaybackSnapshot {
        self.snapshot = PlaybackSnapshot::default();
        self.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::{PlaybackSession, PlaybackStatus};

    #[test]
    fn timeline_only_advances_while_playing_and_ends_at_duration() {
        let mut session = PlaybackSession::default();
        assert_eq!(session.tick(100), session.snapshot());
        assert_eq!(session.load("song-1", 1_000).status, PlaybackStatus::Paused);
        assert_eq!(session.tick(100).position_ms, 0);
        assert_eq!(session.play().status, PlaybackStatus::Playing);
        assert_eq!(session.tick(400).position_ms, 400);
        assert_eq!(session.tick(700).position_ms, 1_000);
        assert_eq!(session.snapshot().status, PlaybackStatus::Ended);
    }

    #[test]
    fn seek_clamps_and_resume_from_end_restarts_track() {
        let mut session = PlaybackSession::default();
        session.load("song-1", 1_000);
        assert_eq!(session.seek(2_000).position_ms, 1_000);
        assert_eq!(session.snapshot().status, PlaybackStatus::Ended);
        assert_eq!(session.play().position_ms, 0);
        assert_eq!(session.snapshot().status, PlaybackStatus::Playing);
    }

    #[test]
    fn pause_freezes_position_and_stop_clears_track() {
        let mut session = PlaybackSession::default();
        session.load("song-1", 1_000);
        session.play();
        session.tick(300);
        assert_eq!(session.pause().status, PlaybackStatus::Paused);
        assert_eq!(session.tick(300).position_ms, 300);
        assert_eq!(session.stop().status, PlaybackStatus::Idle);
        assert_eq!(session.snapshot().track_id, None);
    }
}
