//! N-API adapter for the shared aurora-core library.

#![deny(clippy::unwrap_used)]

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use aurora_audio::{probe_capabilities, NativePlaybackEngine, Qmc2Decryptor};
use aurora_core::{
    download::DownloadSession, lyrics, metadata, provider, scan, settings_io, sync, AuroraError,
};
use aurora_player::PlaybackSession;
use napi::bindgen_prelude::Buffer;
use napi_derive::napi;

static PLAYBACK_SESSION: OnceLock<Mutex<PlaybackSession>> = OnceLock::new();
static NATIVE_AUDIO: OnceLock<Mutex<Option<NativePlaybackEngine>>> = OnceLock::new();
static QMC2_DECRYPTOR: OnceLock<Mutex<Option<(String, Arc<Qmc2Decryptor>)>>> = OnceLock::new();
static QMC2_STREAMS: OnceLock<Mutex<HashMap<u64, Arc<Qmc2Decryptor>>>> = OnceLock::new();
static NEXT_QMC2_STREAM_ID: OnceLock<Mutex<u64>> = OnceLock::new();
static DOWNLOADS: OnceLock<Mutex<HashMap<u64, DownloadSession>>> = OnceLock::new();
static NEXT_DOWNLOAD_ID: OnceLock<Mutex<u64>> = OnceLock::new();
const MAX_QMC2_STREAMS: usize = 64;

fn downloads() -> &'static Mutex<HashMap<u64, DownloadSession>> {
    DOWNLOADS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_download_id() -> napi::Result<u64> {
    let mut next = NEXT_DOWNLOAD_ID
        .get_or_init(|| Mutex::new(1))
        .lock()
        .map_err(|_| napi::Error::from_reason("download id lock poisoned"))?;
    let id = *next;
    *next = next.checked_add(1).unwrap_or(1);
    Ok(id)
}

fn next_qmc2_stream_id() -> napi::Result<u64> {
    let mut next = NEXT_QMC2_STREAM_ID
        .get_or_init(|| Mutex::new(1))
        .lock()
        .map_err(|_| napi::Error::from_reason("QMC2 stream id lock poisoned"))?;
    let id = *next;
    *next = next.checked_add(1).unwrap_or(1);
    Ok(id)
}
const QMC2_FILE_CHUNK_SIZE: usize = 256 * 1024;

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

/// Create a bounded-lifetime QMC2 stream session for one HTTP Range response.
/// The ekey is parsed once; subsequent chunks only carry the session id and
/// absolute encrypted offset, avoiding repeated key marshaling and cache locks.
#[napi]
pub fn audio_stream_create(ekey: String) -> napi::Result<i64> {
    let decryptor = Arc::new(
        Qmc2Decryptor::from_ekey(&ekey)
            .ok_or_else(|| napi::Error::from_reason("invalid QMC2 ekey"))?,
    );
    let streams = QMC2_STREAMS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = streams
        .lock()
        .map_err(|_| napi::Error::from_reason("QMC2 stream lock poisoned"))?;
    if guard.len() >= MAX_QMC2_STREAMS {
        return Err(napi::Error::from_reason("too many QMC2 stream sessions"));
    }
    let id = next_qmc2_stream_id()?;
    guard.insert(id, decryptor);
    Ok(id as i64)
}

/// Decrypt one chunk through a previously created QMC2 stream session.
#[napi]
pub fn audio_stream_decrypt(id: i64, file_offset: i64, chunk: Buffer) -> napi::Result<Buffer> {
    if id <= 0 || file_offset < 0 {
        return Err(napi::Error::from_reason("invalid QMC2 stream arguments"));
    }
    let streams = QMC2_STREAMS.get_or_init(|| Mutex::new(HashMap::new()));
    let decryptor = streams
        .lock()
        .map_err(|_| napi::Error::from_reason("QMC2 stream lock poisoned"))?
        .get(&(id as u64))
        .cloned()
        .ok_or_else(|| napi::Error::from_reason("QMC2 stream session not found"))?;
    Ok(Buffer::from(decryptor.decrypt(&chunk, file_offset as u64)))
}

/// Release a QMC2 stream session after the corresponding HTTP response ends.
#[napi]
pub fn audio_stream_close(id: i64) -> bool {
    if id <= 0 {
        return false;
    }
    QMC2_STREAMS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map(|mut streams| streams.remove(&(id as u64)).is_some())
        .unwrap_or(false)
}

/// Start a bounded temporary-file download transaction.
#[napi]
pub fn download_create(temp_path: String, final_path: String, max_bytes: i64) -> napi::Result<i64> {
    let id = next_download_id()?;
    let session = DownloadSession::create(
        Path::new(&temp_path),
        Path::new(&final_path),
        max_bytes.max(0) as u64,
    )
    .map_err(to_napi_error)?;
    downloads()
        .lock()
        .map_err(|_| napi::Error::from_reason("download lock poisoned"))?
        .insert(id, session);
    Ok(id as i64)
}

#[napi]
pub fn download_write(id: i64, chunk: Buffer) -> napi::Result<String> {
    if id <= 0 {
        return Err(napi::Error::from_reason("invalid download id"));
    }
    let mut sessions = downloads()
        .lock()
        .map_err(|_| napi::Error::from_reason("download lock poisoned"))?;
    let session = sessions
        .get_mut(&(id as u64))
        .ok_or_else(|| napi::Error::from_reason("download session not found"))?;
    let progress = session.write_chunk(&chunk).map_err(to_napi_error)?;
    serde_json::to_string(&progress).map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn download_commit(id: i64) -> napi::Result<String> {
    if id <= 0 {
        return Err(napi::Error::from_reason("invalid download id"));
    }
    let session = downloads()
        .lock()
        .map_err(|_| napi::Error::from_reason("download lock poisoned"))?
        .remove(&(id as u64))
        .ok_or_else(|| napi::Error::from_reason("download session not found"))?;
    let progress = session.commit().map_err(to_napi_error)?;
    serde_json::to_string(&progress).map_err(|error| napi::Error::from_reason(error.to_string()))
}

#[napi]
pub fn download_abort(id: i64) -> napi::Result<bool> {
    if id <= 0 {
        return Ok(false);
    }
    let session = downloads()
        .lock()
        .map_err(|_| napi::Error::from_reason("download lock poisoned"))?
        .remove(&(id as u64));
    match session {
        Some(session) => session.abort().map(|()| true).map_err(to_napi_error),
        None => Ok(false),
    }
}

/// Decrypt a downloaded QMC2 file in place with a bounded native buffer.
/// The file length is unchanged; each chunk is transformed using its absolute
/// encrypted offset so map and RC4 streams match the HTTP protocol path.
#[napi]
pub fn audio_decrypt_qmc2_file(path: String, ekey: String) -> napi::Result<bool> {
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

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|error| napi::Error::from_reason(format!("open QMC2 file: {error}")))?;
    let file_len = file
        .metadata()
        .map_err(|error| napi::Error::from_reason(format!("stat QMC2 file: {error}")))?
        .len();
    let mut buffer = vec![0_u8; QMC2_FILE_CHUNK_SIZE];
    let mut offset = 0_u64;
    while offset < file_len {
        let count = (file_len - offset).min(buffer.len() as u64) as usize;
        file.seek(SeekFrom::Start(offset))
            .map_err(|error| napi::Error::from_reason(format!("seek QMC2 file: {error}")))?;
        file.read_exact(&mut buffer[..count])
            .map_err(|error| napi::Error::from_reason(format!("read QMC2 file: {error}")))?;
        let plain = decryptor.decrypt(&buffer[..count], offset);
        file.seek(SeekFrom::Start(offset))
            .map_err(|error| napi::Error::from_reason(format!("seek QMC2 output: {error}")))?;
        file.write_all(&plain)
            .map_err(|error| napi::Error::from_reason(format!("write QMC2 file: {error}")))?;
        offset += count as u64;
    }
    file.flush()
        .map_err(|error| napi::Error::from_reason(format!("flush QMC2 file: {error}")))?;
    Ok(true)
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

/// Normalize an LX sync server URL while preserving the TypeScript fallback
/// when the optional native module is unavailable.
#[napi]
pub fn sync_normalize_base_url(url: String) -> napi::Result<String> {
    sync::normalize_base_url(&url).map_err(to_napi_error)
}

/// Validate and canonicalize a persisted LX sync session JSON document.
/// `null` means that the session is absent or malformed.
#[napi]
pub fn sync_validate_session(session_json: String) -> napi::Result<String> {
    let value = sync::session_json(&session_json).map_err(to_napi_error)?;
    Ok(value.unwrap_or_else(|| "null".to_string()))
}

/// Parse one QQ Music provider song through the shared Rust mapping.
/// The Electron caller keeps its TypeScript parser when this optional binding
/// is unavailable or rejects malformed input.
#[napi]
pub fn provider_parse_qq_track(item_json: String) -> napi::Result<String> {
    provider::parse_qq_track_json(&item_json).map_err(to_napi_error)
}

/// Parse one Netease Cloud Music provider item through the shared Rust mapping.
#[napi]
pub fn provider_parse_wy_track(item_json: String) -> napi::Result<String> {
    provider::parse_wy_track_json(&item_json).map_err(to_napi_error)
}
