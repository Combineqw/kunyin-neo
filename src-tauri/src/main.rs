use aurora_core::{
    lyrics::{parse_lrc, LyricLine},
    scan::{scan_dir, ScanResult},
    settings_io::{deep_merge, read_json, write_json_atomic},
    sync,
};
use aurora_library::{Library, PlaylistOptions};
use aurora_player::{PlaybackSession, PlaybackSnapshot};
use serde_json::{json, Value};
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Manager, State};

#[derive(Default)]
struct LibraryState {
    library: Mutex<Option<Library>>,
}

#[derive(Default)]
struct PlaybackState {
    session: Mutex<PlaybackSession>,
}

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法定位应用数据目录: {error}"))?
        .join("data");
    Ok(data_dir.join("settings.json"))
}

fn library_path(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法定位应用数据目录: {error}"))?
        .join("data");
    fs::create_dir_all(&data_dir).map_err(|error| format!("无法创建应用数据目录: {error}"))?;
    Ok(data_dir.join("kunyin_music.db"))
}

fn with_library<T>(
    app: &AppHandle,
    state: &State<'_, LibraryState>,
    f: impl FnOnce(&mut Library) -> Result<T, String>,
) -> Result<T, String> {
    let mut guard = state
        .library
        .lock()
        .map_err(|_| "本地曲库锁已失效".to_string())?;
    if guard.is_none() {
        let path = library_path(app)?;
        *guard = Some(Library::open(path).map_err(|error| error.to_string())?);
    }
    f(guard.as_mut().expect("library initialized"))
}

#[tauri::command]
fn parse_lyrics(input: String) -> Vec<LyricLine> {
    parse_lrc(&input)
}

#[tauri::command]
fn read_settings(app: AppHandle) -> Result<Value, String> {
    let path = settings_path(&app)?;
    if !path.exists() {
        return Ok(json!({}));
    }
    read_json(&path).map_err(|error| error.to_string())
}

#[tauri::command]
fn update_settings(app: AppHandle, patch: Value) -> Result<Value, String> {
    let path = settings_path(&app)?;
    let current = if path.exists() {
        read_json(&path).map_err(|error| error.to_string())?
    } else {
        json!({})
    };
    let next = deep_merge(&current, &patch);
    write_json_atomic(&path, &next).map_err(|error| error.to_string())?;
    Ok(next)
}

#[tauri::command]
fn scan_library(path: String) -> Result<ScanResult, String> {
    scan_dir(PathBuf::from(path).as_path()).map_err(|error| error.to_string())
}

#[tauri::command]
fn read_audio_metadata(path: String) -> Result<Value, String> {
    let metadata = aurora_core::metadata::read_audio_tags(PathBuf::from(path).as_path())
        .map_err(|error| error.to_string())?;
    serde_json::to_value(metadata).map_err(|error| error.to_string())
}

#[tauri::command]
fn library_list_playlists(
    app: AppHandle,
    state: State<'_, LibraryState>,
) -> Result<Vec<aurora_library::Playlist>, String> {
    with_library(&app, &state, |library| {
        library.list_playlists().map_err(|error| error.to_string())
    })
}

#[tauri::command]
fn library_create_playlist(
    app: AppHandle,
    state: State<'_, LibraryState>,
    name: String,
    remote_source: Option<String>,
    remote_id: Option<String>,
    auto_refresh: bool,
) -> Result<i64, String> {
    with_library(&app, &state, |library| {
        let options = PlaylistOptions {
            remote_source,
            remote_id,
            auto_refresh,
        };
        library
            .create_playlist(&name, chrono_like_now(), &options)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
fn library_add_song(
    app: AppHandle,
    state: State<'_, LibraryState>,
    playlist_id: i64,
    song_json: String,
    added_at: Option<i64>,
) -> Result<bool, String> {
    let song =
        aurora_library::SongRecord::from_json(song_json).map_err(|error| error.to_string())?;
    with_library(&app, &state, |library| {
        library
            .add_to_playlist(playlist_id, &song, added_at.unwrap_or_else(chrono_like_now))
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
fn library_query_songs(
    app: AppHandle,
    state: State<'_, LibraryState>,
    playlist_id: i64,
) -> Result<Vec<Value>, String> {
    with_library(&app, &state, |library| {
        library
            .query_playlist_songs(playlist_id)
            .map_err(|error| error.to_string())
            .and_then(|songs| {
                songs
                    .into_iter()
                    .map(|song| {
                        serde_json::from_str(&song.song_json).map_err(|error| error.to_string())
                    })
                    .collect()
            })
    })
}

#[tauri::command]
fn library_search_songs(
    app: AppHandle,
    state: State<'_, LibraryState>,
    query: String,
    limit: usize,
    offset: usize,
) -> Result<Vec<Value>, String> {
    with_library(&app, &state, |library| {
        library
            .search_songs(&query, limit, offset)
            .map_err(|error| error.to_string())
            .and_then(songs_to_values)
    })
}

fn songs_to_values(songs: Vec<aurora_library::SongRecord>) -> Result<Vec<Value>, String> {
    songs
        .into_iter()
        .map(|song| serde_json::from_str(&song.song_json).map_err(|error| error.to_string()))
        .collect()
}

#[tauri::command]
fn library_remove_song(
    app: AppHandle,
    state: State<'_, LibraryState>,
    playlist_id: i64,
    song_id: i64,
    source: String,
) -> Result<bool, String> {
    with_library(&app, &state, |library| {
        library
            .remove_from_playlist(playlist_id, song_id, &source)
            .map_err(|error| error.to_string())
    })
}

#[tauri::command]
fn library_move_song(
    app: AppHandle,
    state: State<'_, LibraryState>,
    playlist_id: i64,
    song_id: i64,
    source: String,
    target_position: i64,
) -> Result<(), String> {
    with_library(&app, &state, |library| {
        library
            .move_song(playlist_id, song_id, &source, target_position)
            .map_err(|error| error.to_string())
    })
}

fn with_playback<T>(
    state: &State<'_, PlaybackState>,
    f: impl FnOnce(&mut PlaybackSession) -> T,
) -> Result<T, String> {
    let mut session = state
        .session
        .lock()
        .map_err(|_| "播放状态锁已失效".to_string())?;
    Ok(f(&mut session))
}

#[tauri::command]
fn player_snapshot(state: State<'_, PlaybackState>) -> Result<PlaybackSnapshot, String> {
    with_playback(&state, |session| session.snapshot())
}

#[tauri::command]
fn player_load(
    state: State<'_, PlaybackState>,
    track_id: String,
    duration_ms: u64,
) -> Result<PlaybackSnapshot, String> {
    with_playback(&state, |session| session.load(track_id, duration_ms))
}

#[tauri::command]
fn player_play(state: State<'_, PlaybackState>) -> Result<PlaybackSnapshot, String> {
    with_playback(&state, PlaybackSession::play)
}

#[tauri::command]
fn player_pause(state: State<'_, PlaybackState>) -> Result<PlaybackSnapshot, String> {
    with_playback(&state, PlaybackSession::pause)
}

#[tauri::command]
fn player_seek(
    state: State<'_, PlaybackState>,
    position_ms: u64,
) -> Result<PlaybackSnapshot, String> {
    with_playback(&state, |session| session.seek(position_ms))
}

#[tauri::command]
fn player_tick(
    state: State<'_, PlaybackState>,
    elapsed_ms: u64,
) -> Result<PlaybackSnapshot, String> {
    with_playback(&state, |session| session.tick(elapsed_ms))
}

#[tauri::command]
fn player_stop(state: State<'_, PlaybackState>) -> Result<PlaybackSnapshot, String> {
    with_playback(&state, PlaybackSession::stop)
}

fn chrono_like_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

#[tauri::command]
fn native_capabilities() -> Value {
    json!({
        "host": "tauri",
        "backend": "rust",
        "implemented": ["settings", "scan", "lyrics", "library", "audio_metadata", "playback_session"],
        "pending": ["playback_audio_backend", "downloads", "desktop_windows"],
        "productionReady": false
    })
}

#[tauri::command]
fn audio_backend_capabilities() -> Value {
    serde_json::to_value(aurora_audio::probe_capabilities()).unwrap_or_else(|_| json!({}))
}

#[tauri::command]
fn sync_normalize_base_url(url: String) -> Result<String, String> {
    sync::normalize_base_url(&url).map_err(|error| error.to_string())
}

#[tauri::command]
fn sync_validate_session(session_json: String) -> Result<Option<String>, String> {
    sync::session_json(&session_json).map_err(|error| error.to_string())
}

fn main() {
    tauri::Builder::default()
        .manage(LibraryState::default())
        .manage(PlaybackState::default())
        .invoke_handler(tauri::generate_handler![
            parse_lyrics,
            read_settings,
            update_settings,
            scan_library,
            read_audio_metadata,
            library_list_playlists,
            library_create_playlist,
            library_add_song,
            library_query_songs,
            library_search_songs,
            library_remove_song,
            library_move_song,
            player_snapshot,
            player_load,
            player_play,
            player_pause,
            player_seek,
            player_tick,
            player_stop,
            native_capabilities,
            audio_backend_capabilities,
            sync_normalize_base_url,
            sync_validate_session
        ])
        .run(tauri::generate_context!())
        .expect("failed to run the Tauri shell");
}

#[cfg(test)]
mod tests {
    use super::songs_to_values;
    use aurora_library::SongRecord;

    #[test]
    fn search_results_keep_provider_json_shape() {
        let songs =
            vec![SongRecord::from_json(r#"{"id":7,"type":"local","title":"江南"}"#).unwrap()];
        let values = songs_to_values(songs).unwrap();
        assert_eq!(values[0]["id"], 7);
        assert_eq!(values[0]["title"], "江南");
    }
}
