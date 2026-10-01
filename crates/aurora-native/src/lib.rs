//! N-API adapter for the shared aurora-core library.

#![deny(clippy::unwrap_used)]

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use aurora_core::{lyrics, metadata, scan, settings_io, AuroraError};
use aurora_player::PlaybackSession;
use napi_derive::napi;

static PLAYBACK_SESSION: OnceLock<Mutex<PlaybackSession>> = OnceLock::new();

fn playback_session() -> &'static Mutex<PlaybackSession> {
    PLAYBACK_SESSION.get_or_init(|| Mutex::new(PlaybackSession::default()))
}

fn playback_json(update: impl FnOnce(&mut PlaybackSession)) -> napi::Result<String> {
    let mut session = playback_session()
        .lock()
        .map_err(|_| napi::Error::from_reason("playback session lock poisoned"))?;
    update(&mut session);
    serde_json::to_string(&session.snapshot())
        .map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn playback_snapshot() -> napi::Result<String> {
    playback_json(|_| {})
}

#[napi]
pub fn playback_load(track_id: String, duration_ms: i64) -> napi::Result<String> {
    playback_json(|session| {
        session.load(track_id, duration_ms.max(0) as u64);
    })
}

#[napi]
pub fn playback_play() -> napi::Result<String> {
    playback_json(|session| {
        session.play();
    })
}

#[napi]
pub fn playback_pause() -> napi::Result<String> {
    playback_json(|session| {
        session.pause();
    })
}

#[napi]
pub fn playback_seek(position_ms: i64) -> napi::Result<String> {
    playback_json(|session| {
        session.seek(position_ms.max(0) as u64);
    })
}

#[napi]
pub fn playback_tick(elapsed_ms: i64) -> napi::Result<String> {
    playback_json(|session| {
        session.tick(elapsed_ms.max(0) as u64);
    })
}

#[napi]
pub fn playback_stop() -> napi::Result<String> {
    playback_json(|session| {
        session.stop();
    })
}

fn to_napi_error(error: AuroraError) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}

/// Preserve the JSON-string API consumed by the Electron bridge and shadow tests.
#[napi]
pub fn scan_directory(dir: String) -> napi::Result<String> {
    scan::scan_dir_json(Path::new(&dir)).map_err(to_napi_error)
}

#[napi]
pub fn parse_track(path: String) -> napi::Result<String> {
    scan::parse_track_json(Path::new(&path)).map_err(to_napi_error)
}

/// Read-only Rust metadata bridge. The JSON envelope is `null` for unsupported
/// extensions and a camelCase partial object for recognized audio files.
#[napi]
pub fn read_audio_tags(path: String) -> napi::Result<String> {
    metadata::read_audio_tags_json(Path::new(&path)).map_err(to_napi_error)
}

/// Write the supplied camelCase metadata JSON through the shared Rust tag
/// writer.  The boolean result preserves the core writer's fallback signal
/// for containers whose primary tag type cannot be written by Lofty.
#[napi]
pub fn write_audio_tags(path: String, metadata_json: String) -> napi::Result<bool> {
    metadata::write_audio_tags_json(Path::new(&path), &metadata_json).map_err(to_napi_error)
}

#[napi]
pub fn scan_lyrics(dir: String) -> napi::Result<String> {
    lyrics::scan_lyrics_json(Path::new(&dir)).map_err(to_napi_error)
}

#[napi]
pub fn read_settings(path: String) -> napi::Result<String> {
    settings_io::read_settings_json(Path::new(&path)).map_err(to_napi_error)
}

/// Atomically persist a complete settings JSON document through the shared
/// Rust core.  The Electron caller keeps its existing Node fallback when the
/// optional native module is unavailable or rejects the payload.
#[napi]
pub fn write_settings(path: String, settings_json: String) -> napi::Result<bool> {
    settings_io::write_settings_json(Path::new(&path), &settings_json)
        .map(|()| true)
        .map_err(to_napi_error)
}

#[napi]
pub fn settings_roundtrip(
    source: String,
    sandbox_path: String,
    key_path: String,
    new_value: String,
) -> napi::Result<String> {
    settings_io::roundtrip(
        Path::new(&source),
        Path::new(&sandbox_path),
        &key_path,
        &new_value,
    )
    .map_err(to_napi_error)
}

#[napi]
pub fn native_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
