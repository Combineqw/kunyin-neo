//! 歌词解析（R2-2）· LRC 行级。
//!
//! 对齐目标（渲染层 vendored 引擎 src/renderer/src/lyric/kit/）：
//!   · plugin-format-lrc/parser/utils/match.ts 的 matchLyric()
//!   · plugin-format-lrc/parser/line.ts 的 processNormal()
//!   · utils/time/index.ts 的 parseTime()
//!
//! 契约见 PROJECT_STATUS.md「lyrics 模块」表。关键点：
//!   · start / end 均为**毫秒**
//!   · end 由**下一行的 start 回填**，末行保持 0（最易漏）
//!   · 毫秒规范化是 padEnd(3,'0')[..3] 截断，不是四舍五入
//!   · 纯数字时间戳按**毫秒**解释（123 → 123ms，不是 123 秒）
//!   · 只有时间戳没内容的行被丢弃（LINE_REGEX 结尾是 .+）
//!
//! 本块只做行级：不含逐字音节时序 / TTML / 翻译罗马音三路对齐。
//! 全链路只读。

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{AuroraError, Result};

/// 单行歌词。字段与 PROJECT_STATUS 映射表一致。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LyricLine {
    /// 行开始，毫秒
    pub start: u64,
    /// 行结束，毫秒；由下一行 start 回填，末行为 0
    pub end: u64,
    pub text: String,
}

/// 一个标签 + 其后内容，对应 Node 侧的 MatchItem。
struct MatchItem {
    raw: String,
    tag: String,
    content: String,
}

/// 去掉全部空白，对应 removeTextSpaceAll。
fn remove_space_all(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// 毫秒片段规范化：padEnd(3,'0') 后取前 3 位（截断语义）。
///
/// `.5` → 500，`.51` → 510，`.5140` → 514。
fn parse_milli(frac: &str) -> u64 {
    let mut s: String = frac.chars().take(3).collect();
    while s.len() < 3 {
        s.push('0');
    }
    s.parse::<u64>().unwrap_or(0)
}

/// 解析时间字符串，对齐 parseTime。返回 None 表示不合法。
///
/// 支持 hh:mm:ss(.SSS) / mm:ss(.SSS) / ss.SSS / SSS / .SSS。
/// 分支顺序与 Node 端一致（顺序敏感）。
fn parse_time(content: &str) -> Option<u64> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return None;
    }

    // 分支 1：".SSS" 前缀
    if let Some(rest) = trimmed.strip_prefix('.') {
        if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        return Some(parse_milli(rest));
    }

    // 分支 2：纯数字 → 直接当毫秒（注意不是秒）
    if trimmed.chars().all(|c| c.is_ascii_digit()) {
        return trimmed.parse::<u64>().ok();
    }

    // 分支 3：(hh:)(mm:)ss(.SSS)
    // 先切出小数部分，再按 : 分段，等价于 Node 的 TIME_REGEXP
    let (main, frac) = match trimmed.split_once('.') {
        Some((m, f)) => {
            if f.is_empty() || !f.chars().all(|c| c.is_ascii_digit()) {
                return None;
            }
            (m, parse_milli(f))
        }
        None => (trimmed, 0),
    };

    let parts: Vec<&str> = main.split(':').collect();
    if parts.is_empty() || parts.len() > 3 {
        return None;
    }
    // 每段必须是非空纯数字
    for p in &parts {
        if p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
    }

    let nums: Vec<u64> = parts
        .iter()
        .map(|p| p.parse::<u64>().unwrap_or(0))
        .collect();
    let (hour, minute, second) = match nums.len() {
        1 => (0, 0, nums[0]),
        2 => (0, nums[0], nums[1]),
        _ => (nums[0], nums[1], nums[2]),
    };

    Some(((hour * 60 + minute) * 60 + second) * 1000 + frac)
}

/// 解析形如 `[1:14:514]` / `<1:14:514>` 的标签时间，对应 parseTagTime。
fn parse_tag_time(tag: &str) -> Option<u64> {
    let t = tag.trim();
    // 方括号与尖括号两种写法，取闭合符内的内容
    let inner = match t.chars().next() {
        Some('[') => t.strip_prefix('[').and_then(|x| x.strip_suffix(']'))?,
        Some('<') => t.strip_prefix('<').and_then(|x| x.strip_suffix('>'))?,
        _ => return None,
    };
    // 标签内容不得再含闭合符（对齐 ^[<\[]([^>\]]+)[>\]]$）
    if inner.is_empty() || inner.contains(']') || inner.contains('>') {
        return None;
    }
    parse_time(inner)
}

/// 判定一个 `[...]` 是否是合法时间标签（对应 LINE_REGEXP 的时间分支）。
fn is_time_tag_body(body: &str) -> bool {
    // (?:\d+:)?\d+:\d+(?:\.\d+)?  —— 至少要有一个冒号
    let (main, frac) = match body.split_once('.') {
        Some((m, f)) => (m, Some(f)),
        None => (body, None),
    };
    if let Some(f) = frac {
        if f.is_empty() || !f.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }
    let parts: Vec<&str> = main.split(':').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return false;
    }
    parts
        .iter()
        .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

/// 判定一个 `[...]` 是否是 meta 标签（对应 [a-zA-Z]+\s*:\s*[^\]]+）。
fn is_meta_tag_body(body: &str) -> bool {
    let Some((k, v)) = body.split_once(':') else {
        return false;
    };
    let k = k.trim();
    !k.is_empty() && k.chars().all(|c| c.is_ascii_alphabetic()) && !v.trim().is_empty()
}

/// 在一行内提取所有「标签 + 后续内容」，对应 matchLine 的 LINE_REGEXP。
///
/// 只承认 meta 或时间两类标签；其它 `[...]` 不作为切分点（留在内容里）。
fn match_line(line: &str) -> Vec<MatchItem> {
    let chars: Vec<char> = line.chars().collect();
    // 先找出所有合法标签的 [起,止) 区间
    let mut tags: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '[' {
            if let Some(rel) = chars[i + 1..].iter().position(|c| *c == ']') {
                let close = i + 1 + rel;
                let body: String = chars[i + 1..close].iter().collect();
                if is_time_tag_body(&body) || is_meta_tag_body(&body) {
                    tags.push((i, close + 1));
                    i = close + 1;
                    continue;
                }
            }
        }
        i += 1;
    }

    let mut out = Vec::new();
    for (n, (s, e)) in tags.iter().enumerate() {
        // 内容延伸到下一个标签起点，或行尾
        let content_end = tags.get(n + 1).map(|(ns, _)| *ns).unwrap_or(chars.len());
        let tag: String = chars[*s..*e].iter().collect();
        let content: String = chars[*e..content_end].iter().collect();
        let raw: String = chars[*s..content_end].iter().collect();
        let tag = tag.trim().to_string();
        if tag.is_empty() {
            continue;
        }
        out.push(MatchItem {
            raw,
            tag,
            content: content.trim().to_string(),
        });
    }
    out
}

/// meta 二次判定：raw 去空白后整体形如 `[key:value]`。
fn check_is_valid_meta(raw: &str) -> bool {
    let s = remove_space_all(raw);
    let Some(inner) = s.strip_prefix('[').and_then(|x| x.strip_suffix(']')) else {
        return false;
    };
    if inner.contains(']') {
        return false;
    }
    is_meta_tag_body(inner)
}

/// 歌词行二次判定：raw 去空白后形如 `[时间]非空内容`。
///
/// 对应 LINE_REGEX = ^\[(\d+:)?\d+:\d+(\.\d+)?\].+$ —— 结尾 `.+`
/// 意味着只有时间戳、没有正文的行会被丢弃。
fn check_is_valid_line(raw: &str) -> bool {
    let s = remove_space_all(raw);
    if !s.starts_with('[') {
        return false;
    }
    let Some(close) = s.find(']') else {
        return false;
    };
    let body = &s[1..close];
    if !is_time_tag_body(body) {
        return false;
    }
    // 时间戳之后必须还有内容
    !s[close + 1..].is_empty()
}

/// 解析 LRC 全文为行级结果。
///
/// 流程严格对齐 Node：按 \n 切分 → 每行提标签 → meta/line 二次分流 →
/// 取时间与文本 → 用下一行 start 回填 end。
pub fn parse_lrc(content: &str) -> Vec<LyricLine> {
    // BOM 会让首个标签匹配失败，先剥离（Node 侧由 fs 读取时的编码处理吃掉）
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    if content.trim().is_empty() {
        return Vec::new();
    }

    let mut lines: Vec<LyricLine> = Vec::new();
    for src in content.split('\n') {
        if src.trim().is_empty() {
            continue;
        }
        for item in match_line(src) {
            // meta 先行拦截，再判歌词行；两者都不是则丢弃
            if check_is_valid_meta(&item.raw) {
                continue;
            }
            if !check_is_valid_line(&item.raw) {
                continue;
            }
            // parseTagTime 失败时 Node 用 || 0 兜底，不丢行
            let start = parse_tag_time(&item.tag).unwrap_or(0);
            lines.push(LyricLine {
                start,
                end: 0,
                text: item.content,
            });
        }
    }

    // end 回填：用下一行 start 覆盖当前行 end；末行保持 0
    for i in 0..lines.len().saturating_sub(1) {
        lines[i].end = lines[i + 1].start;
    }

    lines
}

/// 读取 .lrc 文件并解析。只读操作。
pub fn parse_lrc_file(path: &Path) -> Result<Vec<LyricLine>> {
    let bytes = std::fs::read(path).map_err(|e| AuroraError::tag(path, e))?;
    // 歌词一律按 UTF-8 处理；非法字节用替换符兜底，不让整首歌失败
    let text = String::from_utf8_lossy(&bytes);
    Ok(parse_lrc(&text))
}

/// 递归扫描目录下所有 .lrc，返回 路径 → 行数组 的映射。
///
/// 以路径为键，供影子框架按 keyField 配对。
#[derive(Debug, Serialize)]
pub struct LyricFile {
    pub path: String,
    pub lines: Vec<LyricLine>,
}

/// 扫描目录内所有 .lrc 文件并解析。只读。
pub fn scan_lyrics(root: &Path) -> Result<Vec<LyricFile>> {
    if !root.is_dir() {
        return Err(AuroraError::InvalidInput(format!(
            "{} 不是目录",
            root.display()
        )));
    }

    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // 单条读不到不中断整次扫描
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let is_lrc = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("lrc"))
            .unwrap_or(false);
        if !is_lrc {
            continue;
        }
        out.push(LyricFile {
            path: path.display().to_string(),
            lines: parse_lrc_file(path)?,
        });
    }

    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// 序列化为 JSON，供 Node 侧解析。
pub fn scan_lyrics_json(root: &Path) -> Result<String> {
    Ok(serde_json::to_string(&scan_lyrics(root)?)?)
}

/// 匹配成功的歌词行及前后文窗口与进度。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LyricLineMatch {
    pub index: usize,
    pub start: u64,
    pub end: u64,
    pub text: String,
    pub progress: f64,
    pub prev_text: Option<String>,
    pub next_text: Option<String>,
}

/// 在歌词行数组中按当前播放毫秒二分查找高亮行，并计算行内进度与前后行。
pub fn find_active_line(lines: &[LyricLine], time_ms: u64) -> Option<LyricLineMatch> {
    if lines.is_empty() {
        return None;
    }

    // 若当前播放时间早于首行，返回首行但进度为 0
    if time_ms < lines[0].start {
        return Some(LyricLineMatch {
            index: 0,
            start: lines[0].start,
            end: lines[0].end,
            text: lines[0].text.clone(),
            progress: 0.0,
            prev_text: None,
            next_text: lines.get(1).map(|l| l.text.clone()),
        });
    }

    // 二分查找满足 start <= time_ms 的最大下标
    let mut low = 0;
    let mut high = lines.len() - 1;
    let mut best = 0;

    while low <= high {
        let mid = low + (high - low) / 2;
        if lines[mid].start <= time_ms {
            best = mid;
            low = mid + 1;
        } else {
            if mid == 0 {
                break;
            }
            high = mid - 1;
        }
    }

    let line = &lines[best];
    let progress = if line.end > line.start {
        ((time_ms.saturating_sub(line.start)) as f64 / (line.end - line.start) as f64)
            .clamp(0.0, 1.0)
    } else {
        1.0
    };

    Some(LyricLineMatch {
        index: best,
        start: line.start,
        end: line.end,
        text: line.text.clone(),
        progress,
        prev_text: if best > 0 {
            lines.get(best - 1).map(|l| l.text.clone())
        } else {
            None
        },
        next_text: lines.get(best + 1).map(|l| l.text.clone()),
    })
}

/// 桌面歌词窗口外观合成参数（对齐 Apple Design 毛玻璃规范）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DesktopLyricAppearance {
    pub bg_opacity: f64,
    pub text_opacity: f64,
    pub font_weight: u32,
    pub background_rgba: String,
    pub text_opacity_str: String,
    pub shadow_rgba: String,
    pub stroke_width_px: u32,
}

/// 计算桌面歌词合成外观与 CSS 变量。
pub fn compute_desktop_lyric_appearance(
    bg_opacity: f64,
    text_opacity: f64,
    font_weight: u32,
    shadow_opacity: f64,
    stroke_width: u32,
) -> DesktopLyricAppearance {
    let bg_clamped = bg_opacity.clamp(0.0, 1.0);
    let text_clamped = (text_opacity / 100.0).clamp(0.06, 1.0);
    let fw_clamped = font_weight.clamp(100, 900);
    let shadow_clamped = shadow_opacity.clamp(0.0, 1.0);
    let stroke_clamped = stroke_width.clamp(0, 12);

    DesktopLyricAppearance {
        bg_opacity: bg_clamped,
        text_opacity: text_clamped,
        font_weight: fw_clamped,
        background_rgba: format!("rgba(9, 12, 11, {:.2})", bg_clamped),
        text_opacity_str: format!("{:.2}", text_clamped),
        shadow_rgba: format!("rgba(0, 0, 0, {:.2})", shadow_clamped),
        stroke_width_px: stroke_clamped,
    }
}

/// 迷你播放器窗口外观合成参数。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MiniPlayerAppearance {
    pub opacity: f64,
    pub bg_rgba: String,
    pub hover_rgba: String,
    pub border_rgba: String,
}

/// 计算迷你播放器合成外观与 CSS 变量。
pub fn compute_mini_player_appearance(opacity_percent: f64) -> MiniPlayerAppearance {
    let op = (opacity_percent / 100.0).clamp(0.10, 1.0);
    let hover_op = (op + 0.18).min(1.0);

    MiniPlayerAppearance {
        opacity: op,
        bg_rgba: format!("rgba(31, 33, 31, {:.2})", op),
        hover_rgba: format!("rgba(35, 37, 34, {:.2})", hover_op),
        border_rgba: "rgba(255, 255, 255, 0.20)".to_string(),
    }
}
