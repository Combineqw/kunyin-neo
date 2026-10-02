//! Pure remote-provider payload parsers shared by the Electron and Tauri hosts.
//!
//! The host owns HTTP, authentication, and request scheduling. Keeping the QQ
//! item mapping here removes repeated JSON walking from the hot search path
//! while retaining the TypeScript parser as an exact fallback.

use serde_json::{Map, Number, Value};

use crate::error::Result;

fn member<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.as_object().and_then(|object| object.get(key))
}

fn js_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value
            .as_f64()
            .is_some_and(|value| value != 0.0 && value.is_finite()),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn js_number(value: Option<&Value>, default: f64) -> f64 {
    let Some(value) = value else {
        return default;
    };
    if value.is_null() {
        return default;
    }
    let number = match value {
        Value::Number(value) => value.as_f64().unwrap_or(default),
        Value::Bool(value) => u8::from(*value) as f64,
        Value::String(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                0.0
            } else {
                trimmed.parse::<f64>().unwrap_or(default)
            }
        }
        Value::Array(values) if values.is_empty() => 0.0,
        Value::Array(values) if values.len() == 1 => js_number(values.first(), default),
        Value::Array(_) | Value::Object(_) => default,
        Value::Null => default,
    };
    if number.is_finite() {
        number
    } else {
        default
    }
}

fn number_value(value: f64) -> Value {
    if value.is_finite() && value.fract() == 0.0 {
        if value >= i64::MIN as f64 && value <= i64::MAX as f64 {
            return Value::Number(Number::from(value as i64));
        }
    }
    Number::from_f64(value)
        .map(Value::Number)
        .unwrap_or_else(|| Value::Number(Number::from(0)))
}

fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => {
            let number = value.as_f64().unwrap_or(0.0);
            if number.fract() == 0.0 {
                format!("{number:.0}")
            } else {
                value.to_string()
            }
        }
        Value::String(value) => value.clone(),
        Value::Array(values) => values.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

fn add_quality(qualities: &mut Map<String, Value>, id: &str, name: &str, size: f64, media: &Value) {
    if size <= 0.0 {
        return;
    }
    let mut quality = Map::new();
    quality.insert("id".to_string(), Value::String(id.to_string()));
    quality.insert("name".to_string(), Value::String(name.to_string()));
    quality.insert("filesize".to_string(), number_value(size));
    quality.insert("mediaInfo".to_string(), media.clone());
    qualities.insert(id.to_string(), Value::Object(quality));
}

fn add_special(
    qualities: &mut Map<String, Value>,
    id: &str,
    name: &str,
    size_new: &[Value],
    size_index: usize,
    versions: &[Value],
    version_index: usize,
) {
    let Some(size) = size_new.get(size_index) else {
        return;
    };
    let size = js_number(Some(size), 0.0);
    let Some(media) = versions.get(version_index) else {
        return;
    };
    if size > 0.0 && js_truthy(media) {
        let mut quality = Map::new();
        quality.insert("id".to_string(), Value::String(id.to_string()));
        quality.insert("name".to_string(), Value::String(name.to_string()));
        quality.insert("filesize".to_string(), number_value(size));
        quality.insert("mediaInfo".to_string(), media.clone());
        qualities.insert(id.to_string(), Value::Object(quality));
    }
}

/// Parse one QQ Music API song object using the same output contract as the
/// TypeScript parser.
pub fn parse_qq_track(value: &Value) -> Option<Value> {
    if !value.is_object() {
        return None;
    }
    let id = js_number(member(value, "id"), 0.0);
    if id == 0.0 {
        return None;
    }

    let title = member(value, "title")
        .filter(|value| !value.is_null())
        .or_else(|| member(value, "name"))
        .cloned()
        .unwrap_or(Value::Null);
    if !js_truthy(&title) {
        return None;
    }

    let Some(file) = member(value, "file").filter(|value| js_truthy(value)) else {
        return None;
    };
    let singer_values = member(value, "singer")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut singers = Vec::new();
    let mut singer_names = Vec::new();
    for singer in singer_values {
        let Some(name) = member(&singer, "name").filter(|value| js_truthy(value)) else {
            continue;
        };
        let mut output = Map::new();
        output.insert("name".to_string(), name.clone());
        let mid = member(&singer, "mid");
        if let Some(mid) = mid.filter(|value| js_truthy(value)) {
            output.insert(
                "headimg".to_string(),
                Value::String(format!(
                    "https://y.gtimg.cn/music/photo_new/T001R800x800M000{}.jpg",
                    js_string(mid)
                )),
            );
        }
        output.insert(
            "singerId".to_string(),
            number_value(js_number(member(&singer, "id"), 0.0)),
        );
        if let Some(mid) = mid {
            output.insert("extra".to_string(), mid.clone());
        }
        singer_names.push(js_string(name));
        singers.push(Value::Object(output));
    }

    let artist = if singer_names.is_empty() {
        "Unknown".to_string()
    } else {
        singer_names.join("、")
    };
    let album = member(value, "album")
        .filter(|value| !value.is_null())
        .unwrap_or(&Value::Null);
    let album_mid = member(album, "mid")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or_else(|| Value::String(String::new()));
    let album_id = member(album, "id")
        .filter(|value| js_truthy(value) && js_number(Some(value), 0.0) != 0.0)
        .map(js_string)
        .map(Value::String)
        .unwrap_or_else(|| album_mid.clone());
    let media_mid = member(file, "media_mid")
        .filter(|value| !value.is_null())
        .cloned()
        .unwrap_or_else(|| Value::String(String::new()));
    let cover = if js_truthy(&album_mid) {
        Value::String(format!(
            "https://y.gtimg.cn/music/photo_new/T002R800x800M000{}.jpg",
            js_string(&album_mid)
        ))
    } else {
        singers
            .first()
            .and_then(|singer| member(singer, "headimg"))
            .cloned()
            .unwrap_or_else(|| Value::String(String::new()))
    };

    let versions = member(value, "vs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let size_new = member(file, "size_new")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut qualities = Map::new();
    add_quality(
        &mut qualities,
        "128k",
        "普通音质 128K",
        js_number(member(file, "size_128mp3"), 0.0),
        &media_mid,
    );
    add_quality(
        &mut qualities,
        "320k",
        "高品音质 320K",
        js_number(member(file, "size_320mp3"), 0.0),
        &media_mid,
    );
    add_quality(
        &mut qualities,
        "flac",
        "无损音质 FLAC",
        js_number(member(file, "size_flac"), 0.0),
        &media_mid,
    );
    add_quality(
        &mut qualities,
        "hires",
        "无损音质 HiRes",
        js_number(member(file, "size_hires"), 0.0),
        &media_mid,
    );
    add_special(
        &mut qualities,
        "master",
        "臻品母带",
        &size_new,
        0,
        &versions,
        3,
    );
    add_special(
        &mut qualities,
        "atmos",
        "臻品全景声",
        &size_new,
        1,
        &versions,
        4,
    );
    add_special(
        &mut qualities,
        "atmos_plus",
        "臻品全景声 2.0",
        &size_new,
        2,
        &versions,
        4,
    );

    let mut output = Map::new();
    output.insert("type".to_string(), Value::String("qq".to_string()));
    output.insert("id".to_string(), number_value(id));
    output.insert("title".to_string(), title);
    output.insert("artist".to_string(), Value::String(artist));
    output.insert(
        "album".to_string(),
        member(album, "name")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or_else(|| Value::String(String::new())),
    );
    output.insert("albumId".to_string(), album_id);
    output.insert("cover".to_string(), cover);
    output.insert(
        "duration".to_string(),
        number_value(js_number(member(value, "interval"), 0.0) * 1000.0),
    );
    output.insert("qualities".to_string(), Value::Object(qualities));
    output.insert(
        "mid".to_string(),
        member(value, "mid")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or_else(|| Value::String(String::new())),
    );
    output.insert("albumMid".to_string(), album_mid);
    output.insert("mediaMid".to_string(), media_mid);
    output.insert("vs".to_string(), Value::Array(versions));
    output.insert("sizeNew".to_string(), Value::Array(size_new));
    if !singers.is_empty() {
        output.insert("singers".to_string(), Value::Array(singers));
    }
    if let Some(vid) = member(value, "mv").and_then(|mv| member(mv, "vid")) {
        if js_truthy(vid) {
            output.insert("mvid".to_string(), Value::String(js_string(vid)));
        }
    }
    Some(Value::Object(output))
}

/// Parse a JSON-encoded QQ song, returning the JSON null envelope for an item
/// that does not meet the provider's required fields.
pub fn parse_qq_track_json(input: &str) -> Result<String> {
    let value: Value = serde_json::from_str(input)?;
    Ok(serde_json::to_string(&parse_qq_track(&value))?)
}

#[cfg(test)]
mod tests {
    use super::parse_qq_track_json;
    use serde_json::Value;

    #[test]
    fn maps_qq_track_and_quality_fields() {
        let output = parse_qq_track_json(
            r#"{
              "id": 12, "mid": "song-mid", "title": "Song", "interval": 231,
              "singer": [{"id": 7, "name": "Singer", "mid": "singer-mid"}],
              "album": {"id": 99, "name": "Album", "mid": "album-mid"},
              "file": {"media_mid":"media", "size_128mp3": 100, "size_flac": 500,
                "size_new": [1, 2, 3]}, "vs": ["", "", "", "master", "atmos"],
              "mv": {"vid": "mv-id"}
            }"#,
        )
        .unwrap();
        let output: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(output["type"], "qq");
        assert_eq!(output["duration"], 231000);
        assert_eq!(output["albumId"], "99");
        assert_eq!(output["qualities"]["128k"]["mediaInfo"], "media");
        assert_eq!(output["qualities"]["master"]["filesize"], 1);
        assert_eq!(
            output["singers"][0]["headimg"],
            "https://y.gtimg.cn/music/photo_new/T001R800x800M000singer-mid.jpg"
        );
        assert_eq!(output["mvid"], "mv-id");
    }

    #[test]
    fn rejects_missing_file_or_identity() {
        assert_eq!(
            parse_qq_track_json(r#"{"id":1,"title":"Song"}"#).unwrap(),
            "null"
        );
        assert_eq!(
            parse_qq_track_json(r#"{"id":0,"title":"Song","file":{}}"#).unwrap(),
            "null"
        );
        assert_eq!(
            parse_qq_track_json(r#"{"id":1,"title":"","file":{}}"#).unwrap(),
            "null"
        );
    }

    #[test]
    fn keeps_album_mid_when_album_id_is_zero() {
        let output = parse_qq_track_json(
            r#"{"id":1,"name":"Song","album":{"id":0,"mid":"album-mid"},"file":{}}"#,
        )
        .unwrap();
        let output: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(output["albumId"], "album-mid");
    }
}
