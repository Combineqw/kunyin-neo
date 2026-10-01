//! N-API adapter for the shared aurora-core library.

#![deny(clippy::unwrap_used)]

use std::path::Path;

use aurora_core::{lyrics, metadata, scan, settings_io, AuroraError};
use napi_derive::napi;

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
    metadata::write_audio_tags_json(Path::new(&path), &metadata_json)
        .map_err(to_napi_error)
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
