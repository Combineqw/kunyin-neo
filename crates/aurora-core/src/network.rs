//! 在线网络模块（R2-Online）。
//!
//! 提供跨源在线网络请求调度、音源请求头构建、分段流规划与网络代理标准化：
//!   · 覆盖六大在线音源（QQ、网易云、酷狗、酷我、咪咕、Bilibili）的伪装头与防盗链 Referer
//!   · 支持 HTTP Range 范围切片规划
//!   · 提供网络代理有效性检验与 URI 规范化

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

use crate::error::{AuroraError, Result};

/// 在线音源平台标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnlineProvider {
    Qq,
    Wy,
    Kg,
    Kw,
    Mg,
    Bili,
    General,
}

impl OnlineProvider {
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "qq" | "tx" | "tencent" => Self::Qq,
            "wy" | "163" | "netease" => Self::Wy,
            "kg" | "kugou" => Self::Kg,
            "kw" | "kuwo" => Self::Kw,
            "mg" | "migu" => Self::Mg,
            "bili" | "bilibili" => Self::Bili,
            _ => Self::General,
        }
    }
}

/// 在线网络请求头生成器。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkHeaders {
    pub provider: OnlineProvider,
    pub target_url: String,
    pub headers: BTreeMap<String, String>,
}

/// 构建指定音源与目标 URL 的标准请求头映射。
pub fn build_network_headers(
    provider_str: &str,
    target_url: &str,
    range_start: Option<u64>,
    range_end: Option<u64>,
) -> NetworkHeaders {
    let provider = OnlineProvider::from_str_loose(provider_str);
    let mut headers = BTreeMap::new();

    // 基础通配头
    headers.insert(
        "Accept".to_string(),
        "*/*".to_string(),
    );
    headers.insert(
        "Accept-Language".to_string(),
        "zh-CN,zh;q=0.9,en;q=0.8".to_string(),
    );
    headers.insert(
        "Connection".to_string(),
        "keep-alive".to_string(),
    );

    // 平台专有 User-Agent 与防盗链头
    match provider {
        OnlineProvider::Qq => {
            headers.insert(
                "User-Agent".to_string(),
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36 Edg/126.0.0.0".to_string(),
            );
            headers.insert("Referer".to_string(), "https://y.qq.com/".to_string());
            headers.insert("Origin".to_string(), "https://y.qq.com".to_string());
        }
        OnlineProvider::Wy => {
            headers.insert(
                "User-Agent".to_string(),
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36".to_string(),
            );
            headers.insert("Referer".to_string(), "https://music.163.com/".to_string());
            headers.insert("Origin".to_string(), "https://music.163.com".to_string());
        }
        OnlineProvider::Kg => {
            headers.insert(
                "User-Agent".to_string(),
                "KuGou2012-9020-Metadata".to_string(),
            );
            headers.insert("Referer".to_string(), "https://www.kugou.com/".to_string());
        }
        OnlineProvider::Kw => {
            headers.insert(
                "User-Agent".to_string(),
                "kwplayer_ar_10.5.2.0_auto.apk".to_string(),
            );
            headers.insert("Referer".to_string(), "https://www.kuwo.cn/".to_string());
            headers.insert("csrf".to_string(), "KUWO_DESKTOP_AURORA".to_string());
        }
        OnlineProvider::Mg => {
            headers.insert(
                "User-Agent".to_string(),
                "Mozilla/5.0 (Linux; U; Android 11.0.0) AppleWebKit/537.36 (KHTML, like Gecko)".to_string(),
            );
            headers.insert("Referer".to_string(), "https://music.migu.cn/".to_string());
        }
        OnlineProvider::Bili => {
            headers.insert(
                "User-Agent".to_string(),
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0 Safari/537.36".to_string(),
            );
            headers.insert("Referer".to_string(), "https://www.bilibili.com/".to_string());
        }
        OnlineProvider::General => {
            headers.insert(
                "User-Agent".to_string(),
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36".to_string(),
            );
        }
    }

    // HTTP Range 分段头
    if let Some(start) = range_start {
        let range_val = match range_end {
            Some(end) => format!("bytes={}-{}", start, end),
            None => format!("bytes={}-", start),
        };
        headers.insert("Range".to_string(), range_val);
    }

    NetworkHeaders {
        provider,
        target_url: target_url.to_string(),
        headers,
    }
}

/// 在线流式分段切片计划。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NetworkStreamRange {
    pub start: u64,
    pub end: u64,
    pub length: u64,
    pub range_header: String,
    pub is_final: bool,
}

/// 计算在线流请求的推荐范围（支持按固定 chunk_size 切片并对齐）。
pub fn plan_stream_range(start_byte: u64, total_bytes: i64, chunk_size: u64) -> NetworkStreamRange {
    let step = chunk_size.max(64 * 1024);
    if total_bytes > 0 {
        let total = total_bytes as u64;
        let end = (start_byte + step - 1).min(total.saturating_sub(1));
        let length = end.saturating_sub(start_byte) + 1;
        let is_final = end + 1 >= total;
        NetworkStreamRange {
            start: start_byte,
            end,
            length,
            range_header: format!("bytes={}-{}", start_byte, end),
            is_final,
        }
    } else {
        let end = start_byte + step - 1;
        NetworkStreamRange {
            start: start_byte,
            end,
            length: step,
            range_header: format!("bytes={}-{}", start_byte, end),
            is_final: false,
        }
    }
}

/// 规范化网络代理配置。
pub fn validate_and_normalize_proxy(
    enable: bool,
    host: &str,
    port: i32,
) -> Result<Option<String>> {
    if !enable {
        return Ok(None);
    }

    let h = host.trim();
    if h.is_empty() {
        return Err(AuroraError::InvalidInput("代理服务器地址不能为空".to_string()));
    }

    if port <= 0 || port > 65535 {
        return Err(AuroraError::InvalidInput(format!("代理端口 {} 不合法 (1..65535)", port)));
    }

    // 剥除协议前缀以统一格式
    let clean_host = if let Some(rest) = h.strip_prefix("http://") {
        rest
    } else if let Some(rest) = h.strip_prefix("https://") {
        rest
    } else {
        h
    };

    Ok(Some(format!("http://{}:{}", clean_host.trim_end_matches('/'), port)))
}
