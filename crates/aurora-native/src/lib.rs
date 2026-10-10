//! N-API adapter for the shared aurora-core library.

#![deny(clippy::unwrap_used)]

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use aurora_audio::{
    negotiate_remote_format, probe_capabilities, resolve_replay_gain_db, NativePlaybackEngine,
    Qmc2Decryptor, RemotePlaybackShadowSession, ReplayGainDsp, StreamCache,
};
use aurora_core::{
    download::DownloadSession, lyrics, metadata, provider, scan, settings_io, sync, theme,
    AuroraError,
};
use aurora_player::PlaybackSession;
use napi::bindgen_prelude::Buffer;
use napi_derive::napi;

static PLAYBACK_SESSION: OnceLock<Mutex<PlaybackSession>> = OnceLock::new();
static NATIVE_AUDIO: OnceLock<Mutex<Option<NativePlaybackEngine>>> = OnceLock::new();
static QMC2_DECRYPTOR: OnceLock<Mutex<Option<(String, Arc<Qmc2Decryptor>)>>> = OnceLock::new();
static QMC2_STREAMS: OnceLock<Mutex<HashMap<u64, Arc<Qmc2Decryptor>>>> = OnceLock::new();
static NEXT_QMC2_STREAM_ID: OnceLock<Mutex<u64>> = OnceLock::new();
static REMOTE_SHADOW_SESSIONS: OnceLock<Mutex<HashMap<String, RemotePlaybackShadowSession>>> =
    OnceLock::new();
static STREAM_CACHES: OnceLock<Mutex<HashMap<String, StreamCache>>> = OnceLock::new();
static DOWNLOADS: OnceLock<Mutex<HashMap<u64, DownloadSession>>> = OnceLock::new();
static NEXT_DOWNLOAD_ID: OnceLock<Mutex<u64>> = OnceLock::new();
const MAX_QMC2_STREAMS: usize = 64;
const MAX_REMOTE_SHADOW_SESSIONS: usize = 64;
const MAX_STREAM_CACHES: usize = 32;

fn stream_caches() -> &'static Mutex<HashMap<String, StreamCache>> {
    STREAM_CACHES.get_or_init(|| Mutex::new(HashMap::new()))
}

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

fn remote_shadow_sessions() -> &'static Mutex<HashMap<String, RemotePlaybackShadowSession>> {
    REMOTE_SHADOW_SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
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

/// Convert a ReplayGain dB value through the Rust DSP boundary.
///
/// This is intentionally a parameter-level call rather than a per-buffer IPC
/// path. The renderer keeps its existing GainNode until native mixing is
/// attached, while native output adapters can share the exact clamp rules.
#[napi]
pub fn audio_replay_gain_linear(db: f64) -> f64 {
    ReplayGainDsp::db_to_linear(db)
}

/// Resolve one track/album ReplayGain value with peak protection in Rust.
/// The renderer can compare this result with its existing pure TypeScript
/// resolver before a future native mixer consumes the value directly.
#[napi]
pub fn audio_replay_gain_db(
    track_db: Option<f64>,
    album_db: Option<f64>,
    track_peak: Option<f64>,
    album_peak: Option<f64>,
    album_mode: bool,
    preamp_db: f64,
    max_db: f64,
) -> f64 {
    resolve_replay_gain_db(
        track_db, album_db, track_peak, album_peak, album_mode, preamp_db, max_db,
    )
}

/// Return the deterministic Rust-owned theme motion profile. The renderer
/// consumes the JSON only to apply CSS variables and final compositing.
///
/// `requested_fps` and `display_refresh_hz` are optional to preserve the
/// two-argument ABI used by older Electron builds. A request of zero selects
/// static mode; omitted values use native automatic pacing.
#[napi]
pub fn theme_motion_profile(
    theme_id: String,
    reduced_motion: bool,
    requested_fps: Option<f64>,
    display_refresh_hz: Option<f64>,
) -> napi::Result<String> {
    let requested_fps = requested_fps.map(|value| value as f32);
    let display_refresh_hz = display_refresh_hz.map(|value| value as f32);
    serde_json::to_string(&theme::motion_profile_with_fps(
        &theme_id,
        reduced_motion,
        requested_fps,
        display_refresh_hz,
    ))
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

/// Negotiate remote stream format (MIME, container, encryption, and normalized URL).
///
/// This pure, host-neutral calculation provides the exact same contract as the
/// Electron protocol handler while avoiding repeated JS regex/string operations.
#[napi]
pub fn remote_stream_negotiate_format(
    url: String,
    content_type_hint: Option<String>,
    upstream_content_type: Option<String>,
    ekey: Option<String>,
) -> napi::Result<String> {
    let result = negotiate_remote_format(
        &url,
        content_type_hint.as_deref(),
        upstream_content_type.as_deref(),
        ekey.as_deref(),
    );
    serde_json::to_string(&result).map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Create or reset a testable shadow playback session for one remote stream.
///
/// The shadow session tracks format negotiation, buffer watermarks, and seek
/// state without taking over Chromium HTMLAudio decoding or Web Audio rendering.
#[napi]
pub fn remote_playback_shadow_create(
    session_id: String,
    url: String,
    content_type_hint: Option<String>,
    ekey: Option<String>,
    duration_ms: i64,
) -> napi::Result<String> {
    let mut sessions = remote_shadow_sessions()
        .lock()
        .map_err(|_| napi::Error::from_reason("remote shadow session lock poisoned"))?;
    if sessions.len() >= MAX_REMOTE_SHADOW_SESSIONS && !sessions.contains_key(&session_id) {
        if let Some(oldest) = sessions.keys().next().cloned() {
            sessions.remove(&oldest);
        }
    }
    let session = RemotePlaybackShadowSession::new(
        &session_id,
        &url,
        content_type_hint.as_deref(),
        ekey.as_deref(),
        duration_ms.max(0) as u64,
    );
    let snapshot = session.snapshot();
    sessions.insert(session_id, session);
    serde_json::to_string(&snapshot).map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Update shadow session with upstream HTTP response metadata.
#[napi]
pub fn remote_playback_shadow_update_response(
    session_id: String,
    status_code: u32,
    content_length: Option<i64>,
    content_range: Option<String>,
    upstream_content_type: Option<String>,
) -> napi::Result<Option<String>> {
    let mut sessions = remote_shadow_sessions()
        .lock()
        .map_err(|_| napi::Error::from_reason("remote shadow session lock poisoned"))?;
    let Some(session) = sessions.get_mut(&session_id) else {
        return Ok(None);
    };
    let snapshot = session.update_response(
        status_code as u16,
        content_length.filter(|cl| *cl > 0).map(|cl| cl as u64),
        content_range.as_deref(),
        upstream_content_type.as_deref(),
    );
    serde_json::to_string(&snapshot)
        .map(Some)
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Update shadow session with received chunk byte offset and length.
#[napi]
pub fn remote_playback_shadow_receive_chunk(
    session_id: String,
    offset: i64,
    length: i64,
) -> napi::Result<Option<String>> {
    if offset < 0 || length <= 0 {
        return Ok(None);
    }
    let mut sessions = remote_shadow_sessions()
        .lock()
        .map_err(|_| napi::Error::from_reason("remote shadow session lock poisoned"))?;
    let Some(session) = sessions.get_mut(&session_id) else {
        return Ok(None);
    };
    let snapshot = session.receive_chunk(offset as u64, length as u64);
    serde_json::to_string(&snapshot)
        .map(Some)
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Record seek intent in the shadow session and update watermark/seek state.
#[napi]
pub fn remote_playback_shadow_seek(
    session_id: String,
    target_ms: i64,
    duration_ms: i64,
) -> napi::Result<Option<String>> {
    let mut sessions = remote_shadow_sessions()
        .lock()
        .map_err(|_| napi::Error::from_reason("remote shadow session lock poisoned"))?;
    let Some(session) = sessions.get_mut(&session_id) else {
        return Ok(None);
    };
    let snapshot = session.seek(target_ms.max(0) as u64, duration_ms.max(0) as u64);
    serde_json::to_string(&snapshot)
        .map(Some)
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Record play/pause intent in the shadow session.
#[napi]
pub fn remote_playback_shadow_set_play_state(
    session_id: String,
    playing: bool,
) -> napi::Result<Option<String>> {
    let mut sessions = remote_shadow_sessions()
        .lock()
        .map_err(|_| napi::Error::from_reason("remote shadow session lock poisoned"))?;
    let Some(session) = sessions.get_mut(&session_id) else {
        return Ok(None);
    };
    let snapshot = session.set_play_state(playing);
    serde_json::to_string(&snapshot)
        .map(Some)
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Query current snapshot of a remote shadow playback session.
#[napi]
pub fn remote_playback_shadow_snapshot(session_id: String) -> napi::Result<Option<String>> {
    let sessions = remote_shadow_sessions()
        .lock()
        .map_err(|_| napi::Error::from_reason("remote shadow session lock poisoned"))?;
    let Some(session) = sessions.get(&session_id) else {
        return Ok(None);
    };
    serde_json::to_string(&session.snapshot())
        .map(Some)
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Close and remove a remote shadow playback session.
#[napi]
pub fn remote_playback_shadow_close(session_id: String) -> bool {
    remote_shadow_sessions()
        .lock()
        .map(|mut sessions| sessions.remove(&session_id).is_some())
        .unwrap_or(false)
}

/// Plan next chunk HTTP Range request for a remote shadow playback session.
#[napi]
pub fn remote_stream_plan_range(
    session_id: String,
    current_offset: Option<i64>,
    chunk_size: Option<i64>,
) -> napi::Result<Option<String>> {
    let mut sessions = remote_shadow_sessions()
        .lock()
        .map_err(|_| napi::Error::from_reason("remote shadow session lock poisoned"))?;
    let Some(session) = sessions.get_mut(&session_id) else {
        return Ok(None);
    };
    if let Some(offset) = current_offset.filter(|o| *o >= 0) {
        session.buffer.current_offset = offset as u64;
    }
    let plan = session.plan_next_chunk(chunk_size.filter(|s| *s > 0).map(|s| s as u64));
    match plan {
        Some(p) => serde_json::to_string(&p)
            .map(Some)
            .map_err(|err| napi::Error::from_reason(err.to_string())),
        None => Ok(None),
    }
}

/// Query list of currently buffered byte ranges for a remote shadow session.
#[napi]
pub fn remote_stream_get_buffered_ranges(session_id: String) -> napi::Result<Option<String>> {
    let sessions = remote_shadow_sessions()
        .lock()
        .map_err(|_| napi::Error::from_reason("remote shadow session lock poisoned"))?;
    let Some(session) = sessions.get(&session_id) else {
        return Ok(None);
    };
    let ranges = session.get_buffered_ranges();
    serde_json::to_string(&ranges)
        .map(Some)
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Fast check whether a specific byte offset is already buffered.
#[napi]
pub fn remote_stream_is_offset_buffered(session_id: String, offset: i64) -> napi::Result<bool> {
    if offset < 0 {
        return Ok(false);
    }
    let sessions = remote_shadow_sessions()
        .lock()
        .map_err(|_| napi::Error::from_reason("remote shadow session lock poisoned"))?;
    let Some(session) = sessions.get(&session_id) else {
        return Ok(false);
    };
    Ok(session.is_offset_cached(offset as u64))
}

/// Calculate block-aligned byte range for a target seek timestamp.
#[napi]
pub fn remote_stream_calculate_seek_range(
    session_id: String,
    target_ms: i64,
    duration_ms: i64,
    align_bytes: Option<i64>,
) -> napi::Result<Option<String>> {
    let mut sessions = remote_shadow_sessions()
        .lock()
        .map_err(|_| napi::Error::from_reason("remote shadow session lock poisoned"))?;
    let Some(session) = sessions.get_mut(&session_id) else {
        return Ok(None);
    };
    if duration_ms > 0 {
        session.duration_ms = duration_ms as u64;
    }
    let calc = session.calculate_seek_range(
        target_ms.max(0) as u64,
        align_bytes.filter(|a| *a > 0).map(|a| a as u64),
    );
    serde_json::to_string(&calc)
        .map(Some)
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Create or initialize a bounded two-tier stream cache.
#[napi]
pub fn stream_cache_create(
    cache_id: String,
    max_memory_bytes: Option<i64>,
    temp_dir: Option<String>,
) -> napi::Result<String> {
    let mut caches = stream_caches()
        .lock()
        .map_err(|_| napi::Error::from_reason("stream cache lock poisoned"))?;

    if caches.len() >= MAX_STREAM_CACHES && !caches.contains_key(&cache_id) {
        if let Some(oldest) = caches.keys().next().cloned() {
            caches.remove(&oldest);
        }
    }

    let max_mem = max_memory_bytes
        .filter(|m| *m > 0)
        .map(|m| m as usize)
        .unwrap_or(8 * 1024 * 1024); // default 8 MiB
    let dir = temp_dir.map(std::path::PathBuf::from);

    let cache = StreamCache::new(&cache_id, max_mem, dir);
    let stats = cache.stats();
    caches.insert(cache_id, cache);

    serde_json::to_string(&stats).map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Write a stream chunk at offset into the stream cache.
#[napi]
pub fn stream_cache_write_chunk(cache_id: String, offset: i64, chunk: Buffer) -> napi::Result<i64> {
    if offset < 0 {
        return Err(napi::Error::from_reason("offset cannot be negative"));
    }
    let mut caches = stream_caches()
        .lock()
        .map_err(|_| napi::Error::from_reason("stream cache lock poisoned"))?;
    let cache = caches
        .get_mut(&cache_id)
        .ok_or_else(|| napi::Error::from_reason("stream cache not found"))?;

    let bytes_written = cache
        .write_chunk(offset as u64, &chunk)
        .map_err(|err| napi::Error::from_reason(err.to_string()))?;
    Ok(bytes_written as i64)
}

/// Read cached stream bytes at specific offset.
#[napi]
pub fn stream_cache_read(
    cache_id: String,
    offset: i64,
    max_len: i64,
) -> napi::Result<Option<Buffer>> {
    if offset < 0 || max_len <= 0 {
        return Ok(None);
    }
    let mut caches = stream_caches()
        .lock()
        .map_err(|_| napi::Error::from_reason("stream cache lock poisoned"))?;
    let cache = caches
        .get_mut(&cache_id)
        .ok_or_else(|| napi::Error::from_reason("stream cache not found"))?;

    let maybe_data = cache
        .read_at(offset as u64, max_len as usize)
        .map_err(|err| napi::Error::from_reason(err.to_string()))?;

    Ok(maybe_data.map(Buffer::from))
}

/// Query stats snapshot of a stream cache.
#[napi]
pub fn stream_cache_stats(cache_id: String) -> napi::Result<Option<String>> {
    let caches = stream_caches()
        .lock()
        .map_err(|_| napi::Error::from_reason("stream cache lock poisoned"))?;
    let Some(cache) = caches.get(&cache_id) else {
        return Ok(None);
    };
    serde_json::to_string(&cache.stats())
        .map(Some)
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Close and destroy a stream cache, freeing RAM and cleaning temp files.
#[napi]
pub fn stream_cache_close(cache_id: String) -> bool {
    stream_caches()
        .lock()
        .map(|mut caches| caches.remove(&cache_id).is_some())
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
