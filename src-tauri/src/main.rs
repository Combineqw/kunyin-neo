use aurora_core::{
    lyrics::{parse_lrc, LyricLine},
    scan::{scan_dir, ScanResult},
    settings_io::{deep_merge, read_json, write_json_atomic},
};
use serde_json::{json, Value};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法定位应用数据目录: {error}"))?
        .join("data");
    Ok(data_dir.join("settings.json"))
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
fn native_capabilities() -> Value {
    json!({
        "host": "tauri",
        "backend": "rust",
        "implemented": ["settings", "scan", "lyrics"],
        "pending": ["library", "search", "playback", "downloads", "desktop_windows"],
        "productionReady": false
    })
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            parse_lyrics,
            read_settings,
            update_settings,
            scan_library,
            native_capabilities
        ])
        .run(tauri::generate_context!())
        .expect("failed to run the Tauri shell");
}
