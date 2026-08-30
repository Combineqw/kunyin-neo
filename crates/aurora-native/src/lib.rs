//! aurora-native · 坤音 neo 原生模块（R2 Rust 原生化）
//!
//! 影子阶段定位：与 Node 侧同源输入、各自算一遍、逐字段对答案。
//! 本 crate 不接入主进程 / UI（那是 R2-4），只导出纯函数供测试脚本调用。
//!
//! 模块划分：
//!   · scan        库扫描 / 元数据解析（R2-1 已实现）
//!   · lyrics      歌词解析 · LRC 行级（R2-2 已实现）
//!   · settings_io 设置读写（R2-3 占位）
//!
//! 全局约定：所有对外函数返回 JSON 字符串，字段名与 Node 端完全一致；
//! 关键路径禁裸 unwrap，错误一律经 AuroraError 转 napi::Error。

#![deny(clippy::unwrap_used)]

use std::path::Path;

use napi_derive::napi;

pub mod error;
pub mod lyrics;
pub mod scan;
pub mod settings_io;

/// 递归扫描目录，返回 JSON 数组字符串。
///
/// 每项字段：title / artist / album / duration(毫秒) / path。
/// 只读操作；非音频文件自动跳过；标签缺失回退文件名。
///
/// 返回 JSON 字符串而非对象数组，是为了让「字段名与 Node 端完全一致」这件事
/// 由 Rust 侧的 serde 契约唯一决定，不受 napi 自动转换的命名习惯影响。
#[napi]
pub fn scan_directory(dir: String) -> napi::Result<String> {
    let json = scan::scan_dir_json(Path::new(&dir))?;
    Ok(json)
}

/// 递归扫描目录下所有 .lrc，返回 JSON 数组字符串。
///
/// 每项：path + lines[]，每行 start / end（毫秒）/ text。
/// 只读操作；end 由下一行 start 回填，末行为 0（对齐 Node 引擎）。
#[napi]
pub fn scan_lyrics(dir: String) -> napi::Result<String> {
    let json = lyrics::scan_lyrics_json(Path::new(&dir))?;
    Ok(json)
}

/// 返回本模块版本，供测试脚本确认加载到的是最新产物。
#[napi]
pub fn native_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}