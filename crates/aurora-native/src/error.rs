//! 统一错误类型。
//!
//! 约定（tasks/R2-1.md）：关键路径禁裸 unwrap，一律走 Result。
//! 对 Node 侧暴露时统一转成 napi::Error，带上可读的中文上下文。

use std::fmt;
use std::path::Path;

/// 原生模块的统一错误枚举。
///
/// 刻意不引入 thiserror：依赖白名单只允许 napi / lofty / walkdir / serde / serde_json，
/// 手写 Display + From 足够，也少一个依赖。
#[derive(Debug)]
pub enum AuroraError {
    /// 目录遍历失败（权限、路径不存在等）
    WalkFailed { path: String, reason: String },
    /// 音频标签解析失败（损坏文件、不支持的容器）
    TagParseFailed { path: String, reason: String },
    /// 入参不合法（空路径、目标不是目录等）
    InvalidInput(String),
    /// JSON 序列化失败
    Serialize(String),
}

impl AuroraError {
    /// 构造遍历失败错误。
    pub fn walk(path: impl AsRef<Path>, reason: impl fmt::Display) -> Self {
        Self::WalkFailed {
            path: path.as_ref().display().to_string(),
            reason: reason.to_string(),
        }
    }

    /// 构造标签解析失败错误。
    pub fn tag(path: impl AsRef<Path>, reason: impl fmt::Display) -> Self {
        Self::TagParseFailed {
            path: path.as_ref().display().to_string(),
            reason: reason.to_string(),
        }
    }
}

impl fmt::Display for AuroraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WalkFailed { path, reason } => {
                write!(f, "目录遍历失败：{path}（{reason}）")
            }
            Self::TagParseFailed { path, reason } => {
                write!(f, "音频标签解析失败：{path}（{reason}）")
            }
            Self::InvalidInput(msg) => write!(f, "入参不合法：{msg}"),
            Self::Serialize(msg) => write!(f, "JSON 序列化失败：{msg}"),
        }
    }
}

impl std::error::Error for AuroraError {}

/// 转成 napi 错误，让 Node 侧 catch 到可读信息。
impl From<AuroraError> for napi::Error {
    fn from(e: AuroraError) -> Self {
        napi::Error::from_reason(e.to_string())
    }
}

impl From<serde_json::Error> for AuroraError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serialize(e.to_string())
    }
}

/// 模块内部统一 Result。
pub type Result<T> = std::result::Result<T, AuroraError>;