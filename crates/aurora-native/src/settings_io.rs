//! 设置读写（R2-3）· 序列化层。
//!
//! 对齐目标：`src/main/store/settings.ts` 的 `deepMerge` + `persist` 的
//! 序列化部分（`JSON.stringify(settings, null, 2)`）。
//!
//! **本块范围：只做序列化层**（已与用户确认）。
//! 不做 `load()` 里内嵌的业务规则 —— v1→v2→v3 版本迁移链、字段删除、
//! `writeLyricMeta`→`embedLyric` 改名、11 处 `clampSetting` 钳制、
//! 干湿比例镜像回填、`irsProfileId` 有效性校验。
//! 那些是业务语义，双份实现后加 v4 要改两处，漏一处即数据损坏。
//!
//! 影子期只读约束 v2：本模块**有写路径**，但写只允许落沙箱临时文件。
//! 真实 settings.json 只读，由框架的防篡改校验兜底。

use std::path::Path;

use serde_json::{Map, Value};

use crate::error::{AuroraError, Result};

/// 读取 JSON 文件为 Value。只读操作。
pub fn read_json(path: &Path) -> Result<Value> {
    let bytes = std::fs::read(path).map_err(|e| AuroraError::tag(path, e))?;
    // 剥 BOM：settings.json 由 Node 写出时不带，但外部编辑器可能加上
    let text = String::from_utf8_lossy(&bytes);
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    serde_json::from_str(text).map_err(AuroraError::from)
}

/// 序列化为 2 空格缩进的 JSON，对齐 `JSON.stringify(v, null, 2)`。
///
/// serde_json 的 pretty 格式与 V8 的 2 空格缩进一致（对象/数组换行、
/// `": "` 分隔）。数字文本格式可能有差异，由影子框架按语义等值判定，
/// 并单列「格式差异」供审查。
pub fn to_pretty_json(value: &Value) -> Result<String> {
    serde_json::to_string_pretty(value).map_err(AuroraError::from)
}

/// 深合并：以 base 为骨架，用 patch 覆盖，缺省项保留 base。
///
/// 严格对齐 settings.ts 的 `deepMerge`：
///   · 只有「两侧都是普通对象」才递归合并
///   · 数组视为标量整体替换（`isObject` 排除 Array）
///   · patch 为 undefined 时保留 base；JSON 里无 undefined，
///     故此处等价于「patch 缺该键则保留 base」
pub fn deep_merge(base: &Value, patch: &Value) -> Value {
    match (base, patch) {
        (Value::Object(b), Value::Object(p)) => {
            let mut out: Map<String, Value> = b.clone();
            for (k, pv) in p {
                let merged = match b.get(k) {
                    // 两侧都是对象才递归；数组与标量直接覆盖
                    Some(bv) if bv.is_object() && pv.is_object() => deep_merge(bv, pv),
                    _ => pv.clone(),
                };
                out.insert(k.clone(), merged);
            }
            Value::Object(out)
        }
        // 任一侧非对象 → patch 整体取代 base
        _ => patch.clone(),
    }
}

/// 按点分路径设置一个键，用于回环测试「改一个键」。
///
/// 例：`set_by_path(v, "player.volume", json!(0.5))`。
/// 中间缺失的层级会被创建为对象；遇到非对象中间节点则报错而非静默覆盖。
pub fn set_by_path(root: &mut Value, path: &str, new_value: Value) -> Result<()> {
    let parts: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Err(AuroraError::InvalidInput("键路径为空".into()));
    }

    let mut cur = root;
    for (i, part) in parts.iter().enumerate() {
        let is_last = i + 1 == parts.len();
        if is_last {
            let Value::Object(map) = cur else {
                return Err(AuroraError::InvalidInput(format!(
                    "路径 {path} 的父级不是对象"
                )));
            };
            map.insert((*part).to_string(), new_value);
            return Ok(());
        }

        // 中间层：不存在则建空对象；存在但非对象则报错
        let Value::Object(map) = cur else {
            return Err(AuroraError::InvalidInput(format!(
                "路径 {path} 的中间节点 {part} 的父级不是对象"
            )));
        };
        cur = map
            .entry((*part).to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if !cur.is_object() {
            return Err(AuroraError::InvalidInput(format!(
                "路径 {path} 的中间节点 {part} 不是对象"
            )));
        }
    }
    Ok(())
}

/// 原子写：临时文件 + rename，对齐 settings.ts 的 `persist`。
///
/// **影子期约束**：调用方只允许传沙箱临时目录下的路径。
/// 这里不做路径白名单校验（那是调用方的责任），但函数本身不接触任何
/// 默认路径 —— 目标路径必须显式传入，杜绝「忘了改路径就写到真实文件」。
pub fn write_json_atomic(path: &Path, value: &Value) -> Result<()> {
    let text = to_pretty_json(value)?;
    let tmp = path.with_extension("tmp");
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| AuroraError::tag(dir, e))?;
    }
    std::fs::write(&tmp, text.as_bytes()).map_err(|e| AuroraError::tag(&tmp, e))?;
    std::fs::rename(&tmp, path).map_err(|e| AuroraError::tag(path, e))?;
    Ok(())
}

/// 回环测试：读 → 改一个键 → 序列化 → 读回。
///
/// 返回读回后的完整结构（JSON 字符串），供双引擎比对语义一致性。
/// `sandbox_path` 必须是沙箱临时文件路径；真实文件全程只读。
pub fn roundtrip(
    source: &Path,
    sandbox_path: &Path,
    key_path: &str,
    new_value: &str,
) -> Result<String> {
    // 1. 读真实文件（只读）
    let mut value = read_json(source)?;

    // 2. 改一个键。new_value 按 JSON 解析，解析失败则当字符串字面量
    let parsed: Value =
        serde_json::from_str(new_value).unwrap_or_else(|_| Value::String(new_value.to_string()));
    set_by_path(&mut value, key_path, parsed)?;

    // 3. 序列化并写入沙箱（绝不写 source）
    write_json_atomic(sandbox_path, &value)?;

    // 4. 读回
    let back = read_json(sandbox_path)?;
    to_pretty_json(&back)
}

/// 只读解析真实设置并回吐 pretty JSON，供「读路径」对答案。
pub fn read_settings_json(path: &Path) -> Result<String> {
    to_pretty_json(&read_json(path)?)
}