//! N-API adapter for the shared aurora-core library.

#![deny(clippy::unwrap_used)]

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use aurora_audio::{
    enumerate_audio_devices, negotiate_remote_format, probe_capabilities, resolve_replay_gain_db,
    DspPipelineConfig, NativeAudioSink, NativeDspPipeline, NativePlaybackEngine, PcmFormat,
    Qmc2Decryptor, RemotePlaybackShadowSession, ReplayGainDsp, StreamCache, StreamDecodeOutput,
    StreamDecoder,
};
use aurora_core::{
    download::DownloadSession, lyrics, metadata, network, provider, scan, settings_io, sync,
    theme, AuroraError,
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
static STREAM_CACHES: OnceLock<Mutex<HashMap<String, Arc<Mutex<StreamCache>>>>> = OnceLock::new();
static STREAM_DECODERS: OnceLock<Mutex<HashMap<String, StreamDecoder>>> = OnceLock::new();
static DSP_PIPELINES: OnceLock<Mutex<HashMap<String, NativeDspPipeline>>> = OnceLock::new();
static AUDIO_SINKS: OnceLock<Mutex<HashMap<String, Arc<NativeAudioSink>>>> = OnceLock::new();
static DOWNLOADS: OnceLock<Mutex<HashMap<u64, DownloadSession>>> = OnceLock::new();
static NEXT_DOWNLOAD_ID: OnceLock<Mutex<u64>> = OnceLock::new();
const MAX_QMC2_STREAMS: usize = 64;
const MAX_REMOTE_SHADOW_SESSIONS: usize = 64;
const MAX_STREAM_CACHES: usize = 32;
const MAX_STREAM_DECODERS: usize = 16;
const MAX_DSP_PIPELINES: usize = 16;
const MAX_AUDIO_SINKS: usize = 16;

fn dsp_pipelines() -> &'static Mutex<HashMap<String, NativeDspPipeline>> {
    DSP_PIPELINES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn audio_sinks() -> &'static Mutex<HashMap<String, Arc<NativeAudioSink>>> {
    AUDIO_SINKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn stream_caches() -> &'static Mutex<HashMap<String, Arc<Mutex<StreamCache>>>> {
    STREAM_CACHES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn stream_decoders() -> &'static Mutex<HashMap<String, StreamDecoder>> {
    STREAM_DECODERS.get_or_init(|| Mutex::new(HashMap::new()))
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

    let cache = Arc::new(Mutex::new(StreamCache::new(&cache_id, max_mem, dir)));
    let stats = cache
        .lock()
        .map_err(|_| napi::Error::from_reason("stream cache lock poisoned"))?
        .stats();
    caches.insert(cache_id, cache);

    serde_json::to_string(&stats).map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Write a stream chunk at offset into the stream cache.
#[napi]
pub fn stream_cache_write_chunk(cache_id: String, offset: i64, chunk: Buffer) -> napi::Result<i64> {
    if offset < 0 {
        return Err(napi::Error::from_reason("offset cannot be negative"));
    }
    let caches = stream_caches()
        .lock()
        .map_err(|_| napi::Error::from_reason("stream cache lock poisoned"))?;
    let cache = caches
        .get(&cache_id)
        .ok_or_else(|| napi::Error::from_reason("stream cache not found"))?;

    let mut guard = cache
        .lock()
        .map_err(|_| napi::Error::from_reason("stream cache lock poisoned"))?;
    let bytes_written = guard
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
    let caches = stream_caches()
        .lock()
        .map_err(|_| napi::Error::from_reason("stream cache lock poisoned"))?;
    let cache = caches
        .get(&cache_id)
        .ok_or_else(|| napi::Error::from_reason("stream cache not found"))?;

    let mut guard = cache
        .lock()
        .map_err(|_| napi::Error::from_reason("stream cache lock poisoned"))?;
    let maybe_data = guard
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
    let guard = cache
        .lock()
        .map_err(|_| napi::Error::from_reason("stream cache lock poisoned"))?;
    serde_json::to_string(&guard.stats())
        .map(Some)
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Set known total stream length (from Content-Length or on stream end).
#[napi]
pub fn stream_cache_set_total_length(cache_id: String, total_length: i64) -> napi::Result<bool> {
    if total_length < 0 {
        return Err(napi::Error::from_reason("total length cannot be negative"));
    }
    let caches = stream_caches()
        .lock()
        .map_err(|_| napi::Error::from_reason("stream cache lock poisoned"))?;
    let cache = caches
        .get(&cache_id)
        .ok_or_else(|| napi::Error::from_reason("stream cache not found"))?;

    let mut guard = cache
        .lock()
        .map_err(|_| napi::Error::from_reason("stream cache lock poisoned"))?;
    guard.set_total_length(total_length as u64);
    Ok(true)
}

/// Close and destroy a stream cache, freeing RAM and cleaning temp files.
#[napi]
pub fn stream_cache_close(cache_id: String) -> bool {
    stream_caches()
        .lock()
        .map(|mut caches| caches.remove(&cache_id).is_some())
        .unwrap_or(false)
}

/// Create a stream decoder associated with an existing stream cache.
#[napi]
pub fn stream_decoder_create(
    decoder_id: String,
    cache_id: String,
    container_hint: Option<String>,
    mime_hint: Option<String>,
) -> napi::Result<String> {
    let cache = {
        let caches = stream_caches()
            .lock()
            .map_err(|_| napi::Error::from_reason("stream cache lock poisoned"))?;
        caches
            .get(&cache_id)
            .cloned()
            .ok_or_else(|| napi::Error::from_reason("associated stream cache not found"))?
    };

    let mut decoders = stream_decoders()
        .lock()
        .map_err(|_| napi::Error::from_reason("stream decoder lock poisoned"))?;

    if decoders.len() >= MAX_STREAM_DECODERS && !decoders.contains_key(&decoder_id) {
        if let Some(oldest) = decoders.keys().next().cloned() {
            decoders.remove(&oldest);
        }
    }

    let decoder = StreamDecoder::new(&decoder_id, cache, container_hint, mime_hint);
    let snapshot = decoder.snapshot();
    decoders.insert(decoder_id, decoder);

    serde_json::to_string(&snapshot).map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Attempt to probe metadata of the audio stream.
#[napi]
pub fn stream_decoder_probe(decoder_id: String) -> napi::Result<Option<String>> {
    let mut decoders = stream_decoders()
        .lock()
        .map_err(|_| napi::Error::from_reason("stream decoder lock poisoned"))?;
    let decoder = decoders
        .get_mut(&decoder_id)
        .ok_or_else(|| napi::Error::from_reason("stream decoder not found"))?;

    let info = decoder
        .try_probe()
        .map_err(|err| napi::Error::from_reason(err.to_string()))?;

    match info {
        Some(info) => serde_json::to_string(&info)
            .map(Some)
            .map_err(|err| napi::Error::from_reason(err.to_string())),
        None => Ok(None),
    }
}

/// Step decoding by at most one packet or chunk.
#[napi]
pub fn stream_decoder_step(decoder_id: String) -> napi::Result<String> {
    let mut decoders = stream_decoders()
        .lock()
        .map_err(|_| napi::Error::from_reason("stream decoder lock poisoned"))?;
    let decoder = decoders
        .get_mut(&decoder_id)
        .ok_or_else(|| napi::Error::from_reason("stream decoder not found"))?;

    let output = decoder
        .decode_next_chunk()
        .map_err(|err| napi::Error::from_reason(err.to_string()))?;

    let snap = decoder.snapshot();
    let res = match output {
        StreamDecodeOutput::Chunk(chunk) => serde_json::json!({
            "status": "chunk",
            "frames": chunk.frames,
            "endOfStream": false,
            "underflow": false,
            "positionMs": snap.position_ms,
        }),
        StreamDecodeOutput::NeedMoreData => serde_json::json!({
            "status": "needMoreData",
            "frames": 0,
            "endOfStream": false,
            "underflow": true,
            "positionMs": snap.position_ms,
        }),
        StreamDecodeOutput::EndOfStream => serde_json::json!({
            "status": "endOfStream",
            "frames": 0,
            "endOfStream": true,
            "underflow": false,
            "positionMs": snap.position_ms,
        }),
    };

    Ok(res.to_string())
}

/// Seek stream decoder to target time in milliseconds.
#[napi]
pub fn stream_decoder_seek(decoder_id: String, target_ms: i64) -> napi::Result<String> {
    if target_ms < 0 {
        return Err(napi::Error::from_reason("target_ms cannot be negative"));
    }
    let mut decoders = stream_decoders()
        .lock()
        .map_err(|_| napi::Error::from_reason("stream decoder lock poisoned"))?;
    let decoder = decoders
        .get_mut(&decoder_id)
        .ok_or_else(|| napi::Error::from_reason("stream decoder not found"))?;

    decoder
        .seek_ms(target_ms as u64)
        .map_err(|err| napi::Error::from_reason(err.to_string()))?;

    serde_json::to_string(&decoder.snapshot())
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Retrieve snapshot of stream decoder.
#[napi]
pub fn stream_decoder_snapshot(decoder_id: String) -> napi::Result<Option<String>> {
    let decoders = stream_decoders()
        .lock()
        .map_err(|_| napi::Error::from_reason("stream decoder lock poisoned"))?;
    let Some(decoder) = decoders.get(&decoder_id) else {
        return Ok(None);
    };
    serde_json::to_string(&decoder.snapshot())
        .map(Some)
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Close and destroy a stream decoder.
#[napi]
pub fn stream_decoder_close(decoder_id: String) -> bool {
    stream_decoders()
        .lock()
        .map(|mut decoders| decoders.remove(&decoder_id).is_some())
        .unwrap_or(false)
}

/// Create a native DSP pipeline (10-Band EQ, ReplayGain, Gain Ramp, Soft Limiter).
#[napi]
pub fn dsp_pipeline_create(
    pipeline_id: String,
    sample_rate: i64,
    channels: i64,
    config_json: Option<String>,
) -> napi::Result<String> {
    if sample_rate <= 0 || channels <= 0 {
        return Err(napi::Error::from_reason("sample_rate and channels must be positive"));
    }
    let config = if let Some(json_str) = config_json {
        serde_json::from_str::<DspPipelineConfig>(&json_str)
            .map_err(|err| napi::Error::from_reason(format!("invalid DSP config JSON: {err}")))?
    } else {
        DspPipelineConfig::default()
    };

    let format = PcmFormat {
        sample_rate: sample_rate as u32,
        channels: channels as u16,
    };

    let mut pipelines = dsp_pipelines()
        .lock()
        .map_err(|_| napi::Error::from_reason("dsp pipeline lock poisoned"))?;

    if pipelines.len() >= MAX_DSP_PIPELINES && !pipelines.contains_key(&pipeline_id) {
        if let Some(oldest) = pipelines.keys().next().cloned() {
            pipelines.remove(&oldest);
        }
    }

    let pipeline = NativeDspPipeline::new(format, config);
    let snapshot = pipeline.snapshot();
    pipelines.insert(pipeline_id, pipeline);

    serde_json::to_string(&snapshot).map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Update configuration for an existing DSP pipeline.
#[napi]
pub fn dsp_pipeline_configure(pipeline_id: String, config_json: String) -> napi::Result<String> {
    let config = serde_json::from_str::<DspPipelineConfig>(&config_json)
        .map_err(|err| napi::Error::from_reason(format!("invalid DSP config JSON: {err}")))?;

    let mut pipelines = dsp_pipelines()
        .lock()
        .map_err(|_| napi::Error::from_reason("dsp pipeline lock poisoned"))?;
    let pipeline = pipelines
        .get_mut(&pipeline_id)
        .ok_or_else(|| napi::Error::from_reason("dsp pipeline not found"))?;

    pipeline.configure(config);
    serde_json::to_string(&pipeline.snapshot())
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// In-place process interleaved PCM f32 chunk buffer through the DSP pipeline.
#[napi]
pub fn dsp_pipeline_process(pipeline_id: String, mut chunk: Buffer) -> napi::Result<Option<Buffer>> {
    let bytes: &mut [u8] = chunk.as_mut();
    if bytes.is_empty() {
        return Ok(Some(chunk));
    }
    if bytes.len() % 4 != 0 {
        return Err(napi::Error::from_reason("chunk buffer size must be a multiple of 4 bytes (f32)"));
    }

    let mut pipelines = dsp_pipelines()
        .lock()
        .map_err(|_| napi::Error::from_reason("dsp pipeline lock poisoned"))?;
    let pipeline = pipelines
        .get_mut(&pipeline_id)
        .ok_or_else(|| napi::Error::from_reason("dsp pipeline not found"))?;

    let (prefix, samples, suffix) = unsafe { bytes.align_to_mut::<f32>() };
    if prefix.is_empty() && suffix.is_empty() {
        pipeline.process_interleaved(samples);
    } else {
        let mut sample_vec: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        pipeline.process_interleaved(&mut sample_vec);
        for (i, sample) in sample_vec.iter().enumerate() {
            let b = sample.to_le_bytes();
            bytes[i * 4..i * 4 + 4].copy_from_slice(&b);
        }
    }

    Ok(Some(chunk))
}

/// Reset internal state (biquad delay line and peak stats) of the DSP pipeline.
#[napi]
pub fn dsp_pipeline_reset(pipeline_id: String) -> napi::Result<bool> {
    let mut pipelines = dsp_pipelines()
        .lock()
        .map_err(|_| napi::Error::from_reason("dsp pipeline lock poisoned"))?;
    let pipeline = pipelines
        .get_mut(&pipeline_id)
        .ok_or_else(|| napi::Error::from_reason("dsp pipeline not found"))?;

    pipeline.reset();
    Ok(true)
}

/// Query snapshot status of a DSP pipeline.
#[napi]
pub fn dsp_pipeline_snapshot(pipeline_id: String) -> napi::Result<Option<String>> {
    let pipelines = dsp_pipelines()
        .lock()
        .map_err(|_| napi::Error::from_reason("dsp pipeline lock poisoned"))?;
    let Some(pipeline) = pipelines.get(&pipeline_id) else {
        return Ok(None);
    };
    serde_json::to_string(&pipeline.snapshot())
        .map(Some)
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Close and destroy a DSP pipeline.
#[napi]
pub fn dsp_pipeline_close(pipeline_id: String) -> bool {
    dsp_pipelines()
        .lock()
        .map(|mut pipelines| pipelines.remove(&pipeline_id).is_some())
        .unwrap_or(false)
}

/// Create an end-to-end native streaming audio sink (Decoder -> Resampler -> Native DSP -> Output/WASAPI).
#[napi]
pub fn audio_sink_create(
    sink_id: String,
    cache_id: String,
    container_hint: Option<String>,
    mime_hint: Option<String>,
    dsp_config_json: Option<String>,
    force_virtual: Option<bool>,
) -> napi::Result<String> {
    let cache = {
        let caches = stream_caches()
            .lock()
            .map_err(|_| napi::Error::from_reason("stream cache lock poisoned"))?;
        caches
            .get(&cache_id)
            .cloned()
            .ok_or_else(|| napi::Error::from_reason("associated stream cache not found"))?
    };

    let dsp_config = if let Some(json_str) = dsp_config_json {
        Some(
            serde_json::from_str::<DspPipelineConfig>(&json_str)
                .map_err(|err| napi::Error::from_reason(format!("invalid DSP config JSON: {err}")))?,
        )
    } else {
        None
    };

    let mut sinks = audio_sinks()
        .lock()
        .map_err(|_| napi::Error::from_reason("audio sink lock poisoned"))?;

    if sinks.len() >= MAX_AUDIO_SINKS && !sinks.contains_key(&sink_id) {
        if let Some(oldest) = sinks.keys().next().cloned() {
            sinks.remove(&oldest);
        }
    }

    let sink = Arc::new(NativeAudioSink::new(
        &sink_id,
        cache,
        container_hint,
        mime_hint,
        dsp_config,
        force_virtual.unwrap_or(false),
    ));
    let snapshot = sink.snapshot();
    sinks.insert(sink_id, sink);

    serde_json::to_string(&snapshot).map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Start or resume native audio playback pump.
#[napi]
pub fn audio_sink_play(sink_id: String) -> napi::Result<bool> {
    let sinks = audio_sinks()
        .lock()
        .map_err(|_| napi::Error::from_reason("audio sink lock poisoned"))?;
    let sink = sinks
        .get(&sink_id)
        .ok_or_else(|| napi::Error::from_reason("audio sink not found"))?;

    sink.play().map_err(napi::Error::from_reason)?;
    Ok(true)
}

/// Pause native audio playback pump.
#[napi]
pub fn audio_sink_pause(sink_id: String) -> napi::Result<bool> {
    let sinks = audio_sinks()
        .lock()
        .map_err(|_| napi::Error::from_reason("audio sink lock poisoned"))?;
    let sink = sinks
        .get(&sink_id)
        .ok_or_else(|| napi::Error::from_reason("audio sink not found"))?;

    sink.pause().map_err(napi::Error::from_reason)?;
    Ok(true)
}

/// Stop native audio playback pump and flush buffers.
#[napi]
pub fn audio_sink_stop(sink_id: String) -> napi::Result<bool> {
    let sinks = audio_sinks()
        .lock()
        .map_err(|_| napi::Error::from_reason("audio sink lock poisoned"))?;
    let sink = sinks
        .get(&sink_id)
        .ok_or_else(|| napi::Error::from_reason("audio sink not found"))?;

    sink.stop().map_err(napi::Error::from_reason)?;
    Ok(true)
}

/// Seek native audio sink to target time in milliseconds.
#[napi]
pub fn audio_sink_seek(sink_id: String, target_ms: i64) -> napi::Result<bool> {
    if target_ms < 0 {
        return Err(napi::Error::from_reason("target_ms cannot be negative"));
    }
    let sinks = audio_sinks()
        .lock()
        .map_err(|_| napi::Error::from_reason("audio sink lock poisoned"))?;
    let sink = sinks
        .get(&sink_id)
        .ok_or_else(|| napi::Error::from_reason("audio sink not found"))?;

    sink.seek_ms(target_ms as u64)
        .map_err(napi::Error::from_reason)?;
    Ok(true)
}

/// Adjust master volume and mute state for native audio sink.
#[napi]
pub fn audio_sink_set_volume(sink_id: String, volume: f64, muted: bool) -> napi::Result<bool> {
    let sinks = audio_sinks()
        .lock()
        .map_err(|_| napi::Error::from_reason("audio sink lock poisoned"))?;
    let sink = sinks
        .get(&sink_id)
        .ok_or_else(|| napi::Error::from_reason("audio sink not found"))?;

    sink.set_volume(volume as f32, muted);
    Ok(true)
}

/// Dynamically update DSP effect parameters (10-Band EQ, ReplayGain, Soft Limiter).
#[napi]
pub fn audio_sink_configure_dsp(sink_id: String, dsp_config_json: String) -> napi::Result<String> {
    let config = serde_json::from_str::<DspPipelineConfig>(&dsp_config_json)
        .map_err(|err| napi::Error::from_reason(format!("invalid DSP config JSON: {err}")))?;

    let sinks = audio_sinks()
        .lock()
        .map_err(|_| napi::Error::from_reason("audio sink lock poisoned"))?;
    let sink = sinks
        .get(&sink_id)
        .ok_or_else(|| napi::Error::from_reason("audio sink not found"))?;

    sink.configure_dsp(config);
    serde_json::to_string(&sink.snapshot()).map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Retrieve snapshot and real-time playback metrics of native audio sink.
#[napi]
pub fn audio_sink_snapshot(sink_id: String) -> napi::Result<Option<String>> {
    let sinks = audio_sinks()
        .lock()
        .map_err(|_| napi::Error::from_reason("audio sink lock poisoned"))?;
    let Some(sink) = sinks.get(&sink_id) else {
        return Ok(None);
    };
    serde_json::to_string(&sink.snapshot())
        .map(Some)
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Close and destroy native audio sink session.
#[napi]
pub fn audio_sink_close(sink_id: String) -> bool {
    audio_sinks()
        .lock()
        .map(|mut sinks| sinks.remove(&sink_id).is_some())
        .unwrap_or(false)
}

/// Enumerate host audio output devices and their supported format parameters.
#[napi]
pub fn audio_devices_enumerate() -> napi::Result<String> {
    serde_json::to_string(&enumerate_audio_devices())
        .map_err(|err| napi::Error::from_reason(err.to_string()))
}

/// Seamlessly switch native audio sink to a different output device or toggle exclusive mode.
#[napi]
pub fn audio_sink_switch_device(
    sink_id: String,
    device_name: Option<String>,
    exclusive: Option<bool>,
) -> napi::Result<bool> {
    let sinks = audio_sinks()
        .lock()
        .map_err(|_| napi::Error::from_reason("audio sink lock poisoned"))?;
    let sink = sinks
        .get(&sink_id)
        .ok_or_else(|| napi::Error::from_reason("audio sink not found"))?;

    sink.switch_device(device_name.as_deref(), exclusive.unwrap_or(false))
        .map_err(napi::Error::from_reason)?;
    Ok(true)
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

/// Parse LRC content into lines array JSON.
#[napi]
pub fn lyrics_parse_lrc(content: String) -> napi::Result<String> {
    let lines = lyrics::parse_lrc(&content);
    serde_json::to_string(&lines).map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Find active lyric line and calculate internal progress and surrounding lines.
#[napi]
pub fn lyrics_find_active_line(lines_json: String, time_ms: i64) -> napi::Result<Option<String>> {
    let lines: Vec<lyrics::LyricLine> = serde_json::from_str(&lines_json)
        .map_err(|e| napi::Error::from_reason(format!("invalid lines json: {e}")))?;
    let time = time_ms.max(0) as u64;
    let result = lyrics::find_active_line(&lines, time);
    match result {
        Some(m) => serde_json::to_string(&m)
            .map(Some)
            .map_err(|e| napi::Error::from_reason(e.to_string())),
        None => Ok(None),
    }
}

/// Calculate desktop lyric window appearance parameters.
#[napi]
pub fn desktop_lyric_compute_appearance(
    bg_opacity: f64,
    text_opacity: f64,
    font_weight: u32,
    shadow_opacity: f64,
    stroke_width: u32,
) -> napi::Result<String> {
    let appearance = lyrics::compute_desktop_lyric_appearance(
        bg_opacity,
        text_opacity,
        font_weight,
        shadow_opacity,
        stroke_width,
    );
    serde_json::to_string(&appearance).map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Calculate mini player window appearance parameters.
#[napi]
pub fn mini_player_compute_appearance(opacity_percent: f64) -> napi::Result<String> {
    let appearance = lyrics::compute_mini_player_appearance(opacity_percent);
    serde_json::to_string(&appearance).map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Generate optimized network headers for online music streams and lyrics.
#[napi]
pub fn network_build_headers(
    provider: String,
    target_url: String,
    range_start: Option<i64>,
    range_end: Option<i64>,
) -> napi::Result<String> {
    let start = range_start.map(|s| s.max(0) as u64);
    let end = range_end.map(|e| e.max(0) as u64);
    let headers = network::build_network_headers(&provider, &target_url, start, end);
    serde_json::to_string(&headers).map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Plan network stream chunk ranges.
#[napi]
pub fn network_plan_stream_range(
    start_byte: i64,
    total_bytes: i64,
    chunk_size: i64,
) -> napi::Result<String> {
    let plan = network::plan_stream_range(
        start_byte.max(0) as u64,
        total_bytes,
        chunk_size.max(64 * 1024) as u64,
    );
    serde_json::to_string(&plan).map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Validate and normalize network proxy setting.
#[napi]
pub fn network_validate_proxy(enable: bool, host: String, port: i32) -> napi::Result<String> {
    let res = network::validate_and_normalize_proxy(enable, &host, port).map_err(to_napi_error)?;
    Ok(res.unwrap_or_default())
}

/// Parse full LRC content into syllable lines array with word-level timestamps.
#[napi]
pub fn lyrics_parse_syllables(lrc_text: String) -> napi::Result<String> {
    let lines = lyrics::parse_syllables_lrc(&lrc_text);
    serde_json::to_string(&lines).map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Compute real-time karaoke syllable progress within a line.
#[napi]
pub fn lyrics_compute_syllable_progress(
    line_json: String,
    time_ms: i64,
) -> napi::Result<String> {
    let line: lyrics::SyllableLine = serde_json::from_str(&line_json)
        .map_err(|e| napi::Error::from_reason(format!("invalid syllable line json: {e}")))?;
    let time = time_ms.max(0) as u64;
    let progress = lyrics::compute_syllable_progress(&line, time);
    serde_json::to_string(&progress).map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Render structured desktop lyric frame with progress, color palette, and typography.
#[napi]
pub fn lyrics_render_desktop_frame(
    main_line: String,
    sub_line: Option<String>,
    progress: f64,
    active_color: String,
    normal_color: String,
    bg_opacity: f64,
    text_opacity: f64,
    font_size: u32,
    is_playing: bool,
) -> napi::Result<String> {
    let appearance = lyrics::compute_desktop_lyric_appearance(
        bg_opacity,
        text_opacity,
        600,
        0.72,
        2,
    );
    let frame = lyrics::render_desktop_lyric_frame(
        &main_line,
        sub_line.as_deref(),
        progress,
        &active_color,
        &normal_color,
        &appearance,
        font_size,
        is_playing,
    );
    serde_json::to_string(&frame).map_err(|e| napi::Error::from_reason(e.to_string()))
}
