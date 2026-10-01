//! N-API adapter for the shared aurora-core library.

#![deny(clippy::unwrap_used)]

use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use aurora_audio::{probe_capabilities, NativePlaybackEngine, Qmc2Decryptor};
use aurora_core::{lyrics, metadata, scan, settings_io, AuroraError};
use aurora_player::PlaybackSession;
use napi::bindgen_prelude::Buffer;
use napi_derive::napi;

static PLAYBACK_SESSION: OnceLock<Mutex<PlaybackSession>> = OnceLock::new();
static NATIVE_AUDIO: OnceLock<Mutex<Option<NativePlaybackEngine>>> = OnceLock::new();
static QMC2_DECRYPTOR: OnceLock<Mutex<Option<(String, Arc<Qmc2Decryptor>)>>> = OnceLock::new();

fn playback_session() -> &'static Mutex<PlaybackSession> {
    PLAYBACK_SESSION.get_or_init(|| Mutex::new(PlaybackSession::default()))
}

fn native_audio_session() -> &'static Mutex<Option<NativePlaybackEngine>> {
    NATIVE_AUDIO.get_or_init(|| Mutex::new(None))
}

fn native_audio_json(
    update: impl FnOnce(&mut Option<NativePlaybackEngine>) -> Result<(), String>,
) -> napi::Result<String> {
    let mut session = native_audio_session()
        .lock()
        .map_err(|_| napi::Error::from_reason("native audio session lock poisoned"))?;
    update(&mut session).map_err(napi::Error::from_reason)?;
    serde_json::to_string(&session.as_ref().map(NativePlaybackEngine::snapshot))
        .map_err(|error| napi::Error::from_reason(error.to_string()))
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

/// Return the implemented native-audio boundary and its explicit fallback.
/// The Electron player remains on HTMLAudio until a decoder/output backend is
/// attached to this contract.
#[napi]
pub fn audio_backend_capabilities() -> napi::Result<String> {
    serde_json::to_string(&probe_capabilities())
        .map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn native_audio_start_file(path: String) -> napi::Result<String> {
    let mut session = native_audio_session()
        .lock()
        .map_err(|_| napi::Error::from_reason("native audio session lock poisoned"))?;
    if let Some(previous) = session.as_mut() {
        previous
            .stop()
            .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    }
    let engine = NativePlaybackEngine::open(path)
        .map_err(|error| napi::Error::from_reason(error.to_string()))?;
    *session = Some(engine);
    serde_json::to_string(&session.as_ref().map(NativePlaybackEngine::snapshot))
        .map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn native_audio_play() -> napi::Result<String> {
    native_audio_json(|session| {
        session
            .as_mut()
            .ok_or_else(|| "native audio file is not loaded".to_string())?
            .play()
            .map_err(|error| error.to_string())
    })
}

#[napi]
pub fn native_audio_pause() -> napi::Result<String> {
    native_audio_json(|session| {
        session
            .as_mut()
            .ok_or_else(|| "native audio file is not loaded".to_string())?
            .pause()
            .map_err(|error| error.to_string())
    })
}

#[napi]
pub fn native_audio_set_volume(volume: f64, muted: bool) -> napi::Result<String> {
    native_audio_json(|session| {
        if let Some(engine) = session.as_ref() {
            engine.set_volume(volume as f32, muted);
        }
        Ok(())
    })
}

#[napi]
pub fn native_audio_seek(position_ms: i64) -> napi::Result<String> {
    native_audio_json(|session| {
        session
            .as_mut()
            .ok_or_else(|| "native audio file is not loaded".to_string())?
            .seek(position_ms.max(0) as u64)
            .map_err(|error| error.to_string())
    })
}

#[napi]
pub fn native_audio_stop() -> napi::Result<String> {
    native_audio_json(|session| {
        if let Some(engine) = session.as_mut() {
            engine.stop().map_err(|error| error.to_string())?;
        }
        *session = None;
        Ok(())
    })
}

#[napi]
pub fn native_audio_snapshot() -> napi::Result<String> {
    let session = native_audio_session()
        .lock()
        .map_err(|_| napi::Error::from_reason("native audio session lock poisoned"))?;
    serde_json::to_string(&session.as_ref().map(NativePlaybackEngine::snapshot))
        .map_err(|error| napi::Error::from_reason(error.to_string()))
}

/// Best-effort native QMC2 chunk decryption for the Electron stream protocol.
/// The caller supplies the encrypted file offset so HTTP Range responses can
/// be decrypted independently.  Invalid keys return an error and the host
/// keeps its TypeScript decryptor fallback.
#[napi]
pub fn audio_decrypt_qmc2_chunk(
    ekey: String,
    file_offset: i64,
    chunk: Buffer,
) -> napi::Result<Buffer> {
    if file_offset < 0 {
        return Err(napi::Error::from_reason("negative encrypted stream offset"));
    }
    let cache = QMC2_DECRYPTOR.get_or_init(|| Mutex::new(None));
    let decryptor = {
        let mut guard = cache
            .lock()
            .map_err(|_| napi::Error::from_reason("QMC2 decryptor lock poisoned"))?;
        if let Some((cached_ekey, decryptor)) = guard.as_ref() {
            if cached_ekey == &ekey {
                Arc::clone(decryptor)
            } else {
                let decryptor = Arc::new(
                    Qmc2Decryptor::from_ekey(&ekey)
                        .ok_or_else(|| napi::Error::from_reason("invalid QMC2 ekey"))?,
                );
                *guard = Some((ekey, Arc::clone(&decryptor)));
                decryptor
            }
        } else {
            let decryptor = Arc::new(
                Qmc2Decryptor::from_ekey(&ekey)
                    .ok_or_else(|| napi::Error::from_reason("invalid QMC2 ekey"))?,
            );
            *guard = Some((ekey, Arc::clone(&decryptor)));
            decryptor
        }
    };
    Ok(Buffer::from(decryptor.decrypt(&chunk, file_offset as u64)))
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
