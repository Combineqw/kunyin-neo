//! 库扫描 / 元数据解析（R2-1）。
//!
//! 对齐目标：`src/main/modules/local-music/index.ts` 的 `parseLocalSong()`。
//! 字段契约见 PROJECT_STATUS.md「scan 模块」表，关键点：
//!   · duration 为**毫秒**（Node 端 format.duration 秒 × 1000 后 round）
//!   · 多艺术家用 `、`（U+3001 中文顿号）连接，不是 / 或 ,
//!   · 标签缺失回退文件名「艺术家 - 标题」，按首个 " - " 切分
//!   · 解析失败不丢文件，回退文件名 + duration 0
//!
//! 全链路只读：只有 read/open，无任何写操作（影子阶段约束）。

use std::path::Path;

use lofty::file::{AudioFile, TaggedFileExt};
use lofty::prelude::ItemKey;
use lofty::probe::Probe;
use lofty::tag::Accessor;
use serde::Serialize;
use walkdir::WalkDir;

use crate::error::{AuroraError, Result};

/// 支持的音频扩展名。与 Node 端 `AUDIO_EXTENSIONS` 保持一致。
const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "m4a", "aac", "ogg", "opus", "wma", "ape",
];

/// 多艺术家连接符：中文顿号。Node 端 `artists.filter(Boolean).join('、')`。
const ARTIST_SEPARATOR: &str = "、";

/// 单首歌的扫描结果。字段名与顺序对齐 PROJECT_STATUS 映射表。
#[derive(Debug, Serialize)]
pub struct ScannedTrack {
    pub title: String,
    pub artist: String,
    pub album: String,
    /// 毫秒
    pub duration: u64,
    pub path: String,
}

/// 判断扩展名是否为支持的音频格式（大小写不敏感）。
fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let lower = e.to_ascii_lowercase();
            AUDIO_EXTENSIONS.contains(&lower.as_str())
        })
        .unwrap_or(false)
}

/// 文件名回退解析：「艺术家 - 标题.ext」/「标题.ext」。
///
/// 对齐 Node 端 `parseFileName`：用首个 " - " 切分且要求下标 > 0，
/// 否则整个 basename 作 title、artist 留空。
fn parse_file_name(path: &Path) -> (String, String) {
    let base = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .trim();

    // find 返回字节下标；" - " 全为 ASCII，> 0 的语义与 JS indexOf 一致
    match base.find(" - ") {
        Some(idx) if idx > 0 => {
            let artist = base[..idx].trim().to_string();
            let title = base[idx + 3..].trim().to_string();
            (title, artist)
        }
        _ => (base.to_string(), String::new()),
    }
}

/// 解析单个音频文件。
///
/// 语义对齐 Node 端 `parseLocalSong`：先按文件名兜底，标签能读到就覆盖；
/// 读标签失败**不返回 Err**，而是回退文件名 + duration 0（不丢文件）。
pub fn parse_track(path: &Path) -> ScannedTrack {
    let (fallback_title, fallback_artist) = parse_file_name(path);
    let mut title = fallback_title;
    let mut artist = fallback_artist;
    let mut album = String::new();
    let mut duration: u64 = 0;

    // 读标签：任何失败都静默回退，与 Node 端 try/catch 同语义
    if let Ok(tagged) = Probe::open(path).and_then(|p| p.read()) {
        // 时长直接取毫秒，避免「秒(浮点) → 毫秒」的二次舍入误差
        duration = tagged.properties().duration().as_millis() as u64;

        // primary_tag 拿不到时退到 first_tag（对齐 music-metadata 的合并行为）
        if let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) {
            if let Some(t) = tag.title() {
                let t = t.trim();
                if !t.is_empty() {
                    title = t.to_string();
                }
            }

            // 多艺术家：lofty 用 get_strings 取同 key 的多个值，再按顺序 join
            let artists: Vec<String> = tag
                .get_strings(&ItemKey::TrackArtist)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let joined = artists.join(ARTIST_SEPARATOR);
            if !joined.is_empty() {
                artist = joined;
            }

            if let Some(a) = tag.album() {
                album = a.trim().to_string();
            }
        }
    }

    ScannedTrack {
        title,
        artist,
        album,
        duration,
        path: path.display().to_string(),
    }
}

/// 递归扫描目录，返回全部可识别音频的元数据。
///
/// 只读：仅 WalkDir 遍历 + Probe 读取，无任何写操作。
/// 非音频文件（含图片等）按扩展名直接跳过，不会导致失败。
pub fn scan_dir(root: &Path) -> Result<Vec<ScannedTrack>> {
    if !root.is_dir() {
        return Err(AuroraError::InvalidInput(format!(
            "{} 不是目录",
            root.display()
        )));
    }

    let mut out = Vec::new();
    for entry in WalkDir::new(root).follow_links(false) {
        // 单个条目读不到（权限等）不该中断整次扫描：记为遍历错误并跳过
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                // 不 unwrap、不 panic；构造出的错误信息用于诊断
                let _ = AuroraError::walk(root, e);
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if !is_audio(path) {
            continue;
        }
        out.push(parse_track(path));
    }

    // 稳定排序，保证两侧引擎输出顺序一致（对答案不受遍历顺序影响）
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// 序列化为 JSON 字符串，供 Node 侧解析。
pub fn scan_dir_json(root: &Path) -> Result<String> {
    let tracks = scan_dir(root)?;
    Ok(serde_json::to_string(&tracks)?)
}