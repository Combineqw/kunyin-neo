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

fn add_quality_without_media(qualities: &mut Map<String, Value>, id: &str, name: &str, size: f64) {
    if size <= 0.0 {
        return;
    }
    let mut quality = Map::new();
    quality.insert("id".to_string(), Value::String(id.to_string()));
    quality.insert("name".to_string(), Value::String(name.to_string()));
    quality.insert("filesize".to_string(), number_value(size));
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

/// Parse one Netease Cloud Music song using the provider's JSON contract.
pub fn parse_wy_track(value: &Value) -> Option<Value> {
    if !value.is_object() {
        return None;
    }
    let id = js_number(member(value, "id"), -1.0);
    if id == -1.0 {
        return None;
    }
    let name = member(value, "name").cloned().unwrap_or(Value::Null);
    if !js_truthy(&name) {
        return None;
    }

    let artist_values = member(value, "ar")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut singers = Vec::new();
    let mut artist_names = Vec::new();
    for artist in artist_values {
        let Some(artist_name) = member(&artist, "name").filter(|value| js_truthy(value)) else {
            continue;
        };
        let mut output = Map::new();
        output.insert("name".to_string(), artist_name.clone());
        if let Some(pic_url) = member(&artist, "picUrl").filter(|value| !value.is_null()) {
            output.insert("headimg".to_string(), pic_url.clone());
        }
        output.insert(
            "singerId".to_string(),
            number_value(js_number(member(&artist, "id"), 0.0)),
        );
        artist_names.push(js_string(artist_name));
        singers.push(Value::Object(output));
    }

    let album = member(value, "al")
        .filter(|value| !value.is_null())
        .unwrap_or(&Value::Null);
    let mut output = Map::new();
    output.insert("type".to_string(), Value::String("wy".to_string()));
    output.insert("id".to_string(), number_value(id));
    output.insert("title".to_string(), name);
    output.insert(
        "artist".to_string(),
        Value::String(if artist_names.is_empty() {
            "Unknown".to_string()
        } else {
            artist_names.join("、")
        }),
    );
    output.insert(
        "album".to_string(),
        member(album, "name")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or_else(|| Value::String(String::new())),
    );
    if let Some(album_id) = member(album, "id").filter(|value| !value.is_null()) {
        output.insert("albumId".to_string(), Value::String(js_string(album_id)));
    }
    output.insert(
        "cover".to_string(),
        member(album, "picUrl")
            .filter(|value| !value.is_null())
            .cloned()
            .unwrap_or_else(|| Value::String(String::new())),
    );
    output.insert(
        "duration".to_string(),
        number_value(js_number(member(value, "dt"), 0.0)),
    );
    let mut qualities = Map::new();
    add_quality_without_media(
        &mut qualities,
        "128k",
        "普通音质 128K",
        js_number(member(value, "l").and_then(|v| member(v, "size")), 0.0),
    );
    add_quality_without_media(
        &mut qualities,
        "320k",
        "高品音质 320K",
        js_number(member(value, "h").and_then(|v| member(v, "size")), 0.0),
    );
    add_quality_without_media(
        &mut qualities,
        "flac",
        "无损音质 FLAC",
        js_number(member(value, "sq").and_then(|v| member(v, "size")), 0.0),
    );
    add_quality_without_media(
        &mut qualities,
        "hires",
        "无损音质 HiRes",
        js_number(member(value, "hr").and_then(|v| member(v, "size")), 0.0),
    );
    output.insert("qualities".to_string(), Value::Object(qualities));
    if !singers.is_empty() {
        output.insert("singers".to_string(), Value::Array(singers));
    }
    let mv = js_number(member(value, "mv"), 0.0);
    if mv != 0.0 {
        output.insert(
            "mvid".to_string(),
            Value::String(js_string(&number_value(mv))),
        );
    }
    Some(Value::Object(output))
}

pub fn parse_wy_track_json(input: &str) -> Result<String> {
    let value: Value = serde_json::from_str(input)?;
    Ok(serde_json::to_string(&parse_wy_track(&value))?)
}

use std::collections::BTreeMap;

pub const KG_SALT: &str = "OIlwieks28dk2k092lksi2UIkp";
pub const SQ_ZERO_HASH: &str = "00000000000000000000000000000000";

/// Pure RFC 1321 MD5 message-digest algorithm implementation in Rust.
pub fn md5_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee,
        0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
        0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
        0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
        0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa,
        0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
        0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
        0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c,
        0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
        0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05,
        0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
        0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039,
        0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
        0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
    ];
    const S: [u32; 64] = [
        7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,
        5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,
        4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,
        6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,
    ];

    let mut msg = data.to_vec();
    let original_len_bits = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&original_len_bits.to_le_bytes());

    let mut a0: u32 = 0x67452301;
    let mut b0: u32 = 0xefcdab89;
    let mut c0: u32 = 0x98badcfe;
    let mut d0: u32 = 0x10325476;

    for chunk in msg.chunks_exact(64) {
        let mut m = [0u32; 16];
        for (i, word) in chunk.chunks_exact(4).enumerate() {
            m[i] = u32::from_le_bytes([word[0], word[1], word[2], word[3]]);
        }

        let mut a = a0;
        let mut b = b0;
        let mut c = c0;
        let mut d = d0;

        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((b & c) | (!b & d), i),
                16..=31 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let temp = d;
            d = c;
            c = b;
            b = b.wrapping_add((a.wrapping_add(f).wrapping_add(K[i]).wrapping_add(m[g])).rotate_left(S[i]));
            a = temp;
        }

        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }

    let mut digest = [0u8; 16];
    digest[0..4].copy_from_slice(&a0.to_le_bytes());
    digest[4..8].copy_from_slice(&b0.to_le_bytes());
    digest[8..12].copy_from_slice(&c0.to_le_bytes());
    digest[12..16].copy_from_slice(&d0.to_le_bytes());

    let mut hex = String::with_capacity(32);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(hex, "{:02x}", b);
    }
    hex
}

/// Calculate KuGou MD5 signature from sorted parameters and optional body.
pub fn kg_sign(params: &BTreeMap<String, String>, body: &str) -> String {
    let mut joined = String::new();
    for (k, v) in params {
        joined.push_str(k);
        joined.push('=');
        joined.push_str(v);
    }
    let payload = format!("{}{}{}{}", KG_SALT, joined, body, KG_SALT);
    md5_hex(payload.as_bytes())
}

/// Helper function to clean text: strip <em> tags, replace '/' with space, trim whitespace.
fn clean_text(s: &str) -> String {
    s.replace("<em>", "")
        .replace("</em>", "")
        .replace('/', " ")
        .trim()
        .to_string()
}

/// Helper function to limit singers to max 5.
fn limit_singers(s: &str) -> String {
    if !s.contains('、') {
        return s.to_string();
    }
    let parts: Vec<&str> = s.split('、').collect();
    if parts.len() > 5 {
        parts[..5].join("、")
    } else {
        s.to_string()
    }
}

/// Parse one KuGou Music API song object.
pub fn parse_kg_track(value: &Value) -> Option<Value> {
    if !value.is_object() {
        return None;
    }
    let hash = member(value, "FileHash")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())?;

    let file_size = js_number(member(value, "FileSize"), 0.0);
    let hq_hash = member(value, "HQFileHash").and_then(Value::as_str).unwrap_or("");
    let hq_size = js_number(member(value, "HQFileSize"), 0.0);
    let sq_hash = member(value, "SQFileHash").and_then(Value::as_str).unwrap_or("");
    let sq_size = js_number(member(value, "SQFileSize"), 0.0);
    let res_hash = member(value, "ResFileHash").and_then(Value::as_str).unwrap_or("");
    let res_size = js_number(member(value, "ResFileSize"), 0.0);

    let mut qualities = Map::new();
    let mut all_hash = vec![Value::String(hash.to_string())];

    if file_size > 0.0 {
        add_quality_without_media(&mut qualities, "128k", "普通音质 128K", file_size);
    }
    if !hq_hash.is_empty() {
        if hq_size > 0.0 {
            add_quality_without_media(&mut qualities, "320k", "高品音质 320K", hq_size);
        }
        all_hash.push(Value::String(hq_hash.to_string()));
    }
    if !sq_hash.is_empty() && sq_hash != SQ_ZERO_HASH {
        if sq_size > 0.0 {
            add_quality_without_media(&mut qualities, "flac", "无损音质 FLAC", sq_size);
        }
        all_hash.push(Value::String(sq_hash.to_string()));
    }
    if !res_hash.is_empty() {
        if res_size > 0.0 {
            add_quality_without_media(&mut qualities, "hires", "无损音质 HiRes", res_size);
        }
        all_hash.push(Value::String(res_hash.to_string()));
    }

    if qualities.is_empty() {
        return None;
    }

    let audio_id = js_string(member(value, "MixSongID").unwrap_or(&Value::String(String::new())));
    let id = js_number(member(value, "MixSongID"), 0.0);

    let raw_song_name = js_string(member(value, "SongName").unwrap_or(&Value::String(String::new())));
    let suffix = js_string(member(value, "Suffix").unwrap_or(&Value::String(String::new())));
    let combined_title = if !suffix.is_empty() && !raw_song_name.contains(&suffix) {
        format!("{raw_song_name}{suffix}")
    } else {
        raw_song_name
    };
    let title = clean_text(&combined_title);

    let raw_singer = js_string(member(value, "SingerName").unwrap_or(&Value::String(String::new())));
    let artist = limit_singers(&clean_text(&raw_singer));

    let album_name = clean_text(&js_string(member(value, "AlbumName").unwrap_or(&Value::String(String::new()))));
    let album_id = member(value, "AlbumID")
        .filter(|v| js_truthy(v))
        .map(js_string);

    let duration_sec = js_number(member(value, "Duration"), 0.0);
    let duration_ms = duration_sec * 1000.0;

    let mut singers = Vec::new();
    if let Some(singer_arr) = member(value, "Singers").and_then(Value::as_array).filter(|a| !a.is_empty()) {
        for s in singer_arr {
            let name = clean_text(&js_string(member(s, "name").unwrap_or(&Value::String(String::new()))));
            if !name.is_empty() {
                let sid = js_number(member(s, "id"), 0.0);
                let mut singer_obj = Map::new();
                singer_obj.insert("name".to_string(), Value::String(name));
                singer_obj.insert("singerId".to_string(), number_value(sid));
                singers.push(Value::Object(singer_obj));
            }
        }
    } else {
        for part in artist.split('、') {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                let mut singer_obj = Map::new();
                singer_obj.insert("name".to_string(), Value::String(trimmed.to_string()));
                singer_obj.insert("singerId".to_string(), number_value(0.0));
                singers.push(Value::Object(singer_obj));
            }
        }
    }

    let mut output = Map::new();
    output.insert("type".to_string(), Value::String("kg".to_string()));
    output.insert("id".to_string(), number_value(id));
    output.insert("title".to_string(), Value::String(title));
    output.insert("artist".to_string(), Value::String(artist));
    output.insert("album".to_string(), Value::String(album_name));
    if let Some(aid) = album_id {
        output.insert("albumId".to_string(), Value::String(aid));
    }
    output.insert("cover".to_string(), Value::String(String::new()));
    output.insert("duration".to_string(), number_value(duration_ms));
    output.insert("qualities".to_string(), Value::Object(qualities));
    output.insert("hash".to_string(), Value::String(hash.to_string()));
    output.insert("allHash".to_string(), Value::Array(all_hash));
    output.insert("audioId".to_string(), Value::String(audio_id));
    output.insert("mixsongmid".to_string(), number_value(id));
    if !singers.is_empty() {
        output.insert("singers".to_string(), Value::Array(singers));
    }
    if let Some(mv_id) = member(value, "mvdata")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(|m| member(m, "id"))
        .filter(|v| js_truthy(v))
    {
        output.insert("mvid".to_string(), Value::String(js_string(mv_id)));
    }

    Some(Value::Object(output))
}

pub fn parse_kg_track_json(input: &str) -> Result<String> {
    let value: Value = serde_json::from_str(input)?;
    Ok(serde_json::to_string(&parse_kg_track(&value))?)
}

/// Helper function to parse KuWo minfo string:
/// "level:ff,bitrate:2000,format:flac,size:28.5MB;level:p,bitrate:320,format:mp3,size:8.5MB"
fn parse_kw_minfo(minfo: &str) -> Map<String, Value> {
    let mut qualities = Map::new();
    for seg in minfo.split(';') {
        let seg = seg.trim();
        if seg.is_empty() {
            continue;
        }
        let mut bitrate_opt = None;
        let mut size_opt = None;
        for part in seg.split(',') {
            let part = part.trim();
            if let Some(idx) = part.find(':') {
                let key = &part[..idx];
                let val = &part[idx + 1..];
                if key == "bitrate" {
                    bitrate_opt = Some(val);
                } else if key == "size" {
                    size_opt = Some(val);
                }
            }
        }
        let Some(bitrate) = bitrate_opt else { continue; };
        let (qid, name, br) = match bitrate {
            "128" => ("128k", "普通音质 128K", 128),
            "320" => ("320k", "高品音质 320K", 320),
            "2000" => ("flac", "无损音质 FLAC", 2000),
            "4000" => ("hires", "无损音质 HiRes", 4000),
            "20201" => ("atmos", "臻品全景声", 20201),
            "20501" => ("atmos_plus", "臻品全景声 2.0", 20501),
            "20900" => ("master", "臻品母带", 20900),
            _ => continue,
        };
        let mut q_obj = Map::new();
        q_obj.insert("id".to_string(), Value::String(qid.to_string()));
        q_obj.insert("name".to_string(), Value::String(name.to_string()));
        q_obj.insert("filesize".to_string(), Value::Number(Number::from(0)));
        q_obj.insert("bitrate".to_string(), Value::Number(Number::from(br)));
        if let Some(size) = size_opt {
            q_obj.insert("displaySize".to_string(), Value::String(size.to_uppercase()));
        }
        qualities.insert(qid.to_string(), Value::Object(q_obj));
    }
    qualities
}

/// Parse one KuWo Music API song object.
pub fn parse_kw_track(value: &Value) -> Option<Value> {
    if !value.is_object() {
        return None;
    }
    let rid_str = js_string(member(value, "MUSICRID")?);
    if rid_str.is_empty() {
        return None;
    }
    let id_cleaned = rid_str.replace("MUSIC_", "");
    let id = id_cleaned.parse::<f64>().ok()?;
    if id == 0.0 {
        return None;
    }

    let minfo_str = member(value, "N_MINFO").and_then(Value::as_str)?;
    let qualities = parse_kw_minfo(minfo_str);
    if qualities.is_empty() {
        return None;
    }

    let title = js_string(member(value, "SONGNAME").unwrap_or(&Value::String(String::new())));
    if title.is_empty() {
        return None;
    }

    let artist = js_string(member(value, "ARTIST").unwrap_or(&Value::String(String::new())));
    let album = js_string(member(value, "ALBUM").unwrap_or(&Value::String(String::new())));
    let album_id = member(value, "ALBUMID").filter(|v| js_truthy(v)).map(js_string);

    let duration_sec = js_number(member(value, "DURATION"), 0.0);
    let duration_ms = duration_sec * 1000.0;

    let all_artist_ids = member(value, "allartistid")
        .or_else(|| member(value, "ARTISTID"))
        .map(js_string)
        .unwrap_or_default();
    let ids: Vec<f64> = all_artist_ids
        .split('&')
        .map(|s| s.trim().parse::<f64>().unwrap_or(0.0))
        .collect();

    let mut singers = Vec::new();
    for (i, part) in artist.split('&').enumerate() {
        let trimmed = part.trim();
        if !trimmed.is_empty() {
            let sid = ids.get(i).copied().unwrap_or(0.0);
            let mut s_obj = Map::new();
            s_obj.insert("name".to_string(), Value::String(trimmed.to_string()));
            s_obj.insert("singerId".to_string(), number_value(sid));
            singers.push(Value::Object(s_obj));
        }
    }

    let mut output = Map::new();
    output.insert("type".to_string(), Value::String("kw".to_string()));
    output.insert("id".to_string(), number_value(id));
    output.insert("title".to_string(), Value::String(title));
    output.insert("artist".to_string(), Value::String(artist));
    output.insert("album".to_string(), Value::String(album));
    if let Some(aid) = album_id {
        output.insert("albumId".to_string(), Value::String(aid));
    }
    output.insert("cover".to_string(), Value::String(String::new()));
    output.insert("duration".to_string(), number_value(duration_ms));
    output.insert("qualities".to_string(), Value::Object(qualities));
    if !singers.is_empty() {
        output.insert("singers".to_string(), Value::Array(singers));
    }
    let has_mv = member(value, "HASMV").map(|v| js_string(v) == "1" || js_number(Some(v), 0.0) == 1.0).unwrap_or(false);
    if has_mv {
        output.insert("mvid".to_string(), Value::String(format!("{id:.0}")));
    }

    Some(Value::Object(output))
}

pub fn parse_kw_track_json(input: &str) -> Result<String> {
    let value: Value = serde_json::from_str(input)?;
    Ok(serde_json::to_string(&parse_kw_track(&value))?)
}

/// Extracts the platform-specific musicId sent to the backend getUrl endpoint.
/// Corresponds to extractMusicId in getUrl.ts & Android MusicUrlHelper.kt:
/// - "qq": mid
/// - "joox": mid.unwrap_or(id)
/// - "kg": hash.to_lowercase()
/// - default: id
pub fn extract_backend_music_id(
    platform: &str,
    id: Option<&str>,
    mid: Option<&str>,
    hash: Option<&str>,
) -> String {
    match platform {
        "qq" => mid.unwrap_or("").to_string(),
        "joox" => mid.or(id).unwrap_or("").to_string(),
        "kg" => hash.map(|h| h.to_lowercase()).unwrap_or_default(),
        _ => id.unwrap_or("").to_string(),
    }
}

/// Selects the best available audio quality given a user preference.
/// Priority hierarchy (from highest to lowest):
/// 1. "master" / "atmos_plus" / "atmos" / "dolby" (50)
/// 2. "hires" (40)
/// 3. "flac" (30)
/// 4. "320k" (20)
/// 5. "128k" (10)
pub fn select_optimal_quality<'a>(available: &[&'a str], preferred: &str) -> &'a str {
    if available.is_empty() {
        return "128k";
    }
    if let Some(&exact) = available.iter().find(|&&q| q == preferred) {
        return exact;
    }

    let quality_rank = |q: &str| -> i32 {
        match q {
            "master" | "atmos_plus" | "atmos" | "dolby" => 50,
            "hires" => 40,
            "flac" => 30,
            "320k" => 20,
            "128k" => 10,
            _ => 0,
        }
    };

    let target_rank = quality_rank(preferred);

    let mut best_sub = None;
    let mut best_overall = available[0];
    let mut best_overall_rank = quality_rank(best_overall);

    for &q in available {
        let rank = quality_rank(q);
        if rank <= target_rank {
            match best_sub {
                None => best_sub = Some((q, rank)),
                Some((_, prev_rank)) if rank > prev_rank => best_sub = Some((q, rank)),
                _ => {}
            }
        }
        if rank > best_overall_rank {
            best_overall = q;
            best_overall_rank = rank;
        }
    }

    if let Some((sub_q, _)) = best_sub {
        sub_q
    } else {
        best_overall
    }
}

/// Parses the getUrl backend response JSON into a normalized MediaInfoResult JSON string.
/// Matches getUrl.ts:
/// - input: `{ code: 200, url: "https://...", ekey?: "...", _cacheTTL?: 600, message?: "..." }`
/// - output: `{ source, playUrl, expire, isSuccess, quality, encryptionInfo?: { isEncrypt: true, ekey }, rejectReason?: "..." }`
pub fn parse_media_info_response(
    source: &str,
    quality_id: &str,
    response_json: &str,
    now_ms: u64,
) -> Result<String> {
    let val: Value = serde_json::from_str(response_json)?;
    let code = val.get("code").and_then(|v| v.as_i64()).unwrap_or(0);
    let url_opt = val.get("url").and_then(|v| v.as_str());

    let mut out = Map::new();
    out.insert("source".to_string(), Value::String(source.to_string()));
    out.insert("quality".to_string(), Value::String(quality_id.to_string()));

    if code == 200 && url_opt.is_some() && !url_opt.unwrap().is_empty() {
        let play_url = url_opt.unwrap();
        let cache_ttl = val.get("_cacheTTL").and_then(|v| v.as_u64()).unwrap_or(600);
        let expire = now_ms + cache_ttl * 1000;

        out.insert("playUrl".to_string(), Value::String(play_url.to_string()));
        out.insert("expire".to_string(), Value::Number(Number::from(expire)));
        out.insert("isSuccess".to_string(), Value::Bool(true));

        if let Some(ekey) = val.get("ekey").and_then(|v| v.as_str()) {
            if !ekey.is_empty() && ekey != "null" {
                let mut enc = Map::new();
                enc.insert("isEncrypt".to_string(), Value::Bool(true));
                enc.insert("ekey".to_string(), Value::String(ekey.to_string()));
                out.insert("encryptionInfo".to_string(), Value::Object(enc));
            }
        }
    } else {
        let msg = val.get("message").and_then(|v| v.as_str()).unwrap_or("获取播放地址失败");
        out.insert("playUrl".to_string(), Value::String(String::new()));
        out.insert("expire".to_string(), Value::Number(Number::from(0)));
        out.insert("isSuccess".to_string(), Value::Bool(false));
        out.insert("rejectReason".to_string(), Value::String(msg.to_string()));
    }

    Ok(serde_json::to_string(&Value::Object(out))?)
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn extracts_backend_music_id_for_different_providers() {
        assert_eq!(extract_backend_music_id("qq", Some("123"), Some("003mid"), None), "003mid");
        assert_eq!(extract_backend_music_id("joox", Some("456"), Some("jx_mid"), None), "jx_mid");
        assert_eq!(extract_backend_music_id("joox", Some("456"), None, None), "456");
        assert_eq!(extract_backend_music_id("kg", Some("789"), None, Some("ABCDEF123456")), "abcdef123456");
        assert_eq!(extract_backend_music_id("wy", Some("999"), None, None), "999");
    }

    #[test]
    fn selects_optimal_quality_gracefully() {
        let available = ["128k", "320k", "flac"];
        assert_eq!(select_optimal_quality(&available, "flac"), "flac");
        assert_eq!(select_optimal_quality(&available, "hires"), "flac"); // degrades to flac
        assert_eq!(select_optimal_quality(&available, "320k"), "320k");
        assert_eq!(select_optimal_quality(&available, "128k"), "128k");

        let hires_available = ["128k", "320k", "flac", "hires"];
        assert_eq!(select_optimal_quality(&hires_available, "hires"), "hires");
        assert_eq!(select_optimal_quality(&hires_available, "master"), "hires");
    }

    #[test]
    fn parses_media_info_response_success_and_failure() {
        let success_resp = r#"{"code": 200, "url": "https://cdn.music.com/audio.flac", "ekey": "key123", "_cacheTTL": 300}"#;
        let parsed_json = parse_media_info_response("qq", "flac", success_resp, 1000).unwrap();
        let parsed: Value = serde_json::from_str(&parsed_json).unwrap();
        assert_eq!(parsed["isSuccess"], true);
        assert_eq!(parsed["source"], "qq");
        assert_eq!(parsed["playUrl"], "https://cdn.music.com/audio.flac");
        assert_eq!(parsed["expire"], 301000);
        assert_eq!(parsed["encryptionInfo"]["isEncrypt"], true);
        assert_eq!(parsed["encryptionInfo"]["ekey"], "key123");

        let fail_resp = r#"{"code": 404, "message": "Song copyright restricted"}"#;
        let fail_json = parse_media_info_response("wy", "320k", fail_resp, 1000).unwrap();
        let fail_parsed: Value = serde_json::from_str(&fail_json).unwrap();
        assert_eq!(fail_parsed["isSuccess"], false);
        assert_eq!(fail_parsed["rejectReason"], "Song copyright restricted");
    }

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

    #[test]
    fn maps_wy_track_and_keeps_millisecond_duration() {
        let output = parse_wy_track_json(
            r#"{"id":0,"name":"Song","dt":231000,"ar":[{"id":7,"name":"Singer","picUrl":"pic"}],"al":{"id":0,"name":"Album","picUrl":"cover"},"l":{"size":100},"h":{"size":200},"sq":{"size":500},"hr":{"size":800},"mv":"42"}"#,
        )
        .unwrap();
        let output: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(output["type"], "wy");
        assert_eq!(output["id"], 0);
        assert_eq!(output["duration"], 231000);
        assert_eq!(output["albumId"], "0");
        assert_eq!(output["qualities"]["hires"]["filesize"], 800);
        assert_eq!(output["mvid"], "42");
    }

    #[test]
    fn rejects_wy_missing_id_or_name_and_omits_zero_mv() {
        assert_eq!(
            parse_wy_track_json(r#"{"id":-1,"name":"Song"}"#).unwrap(),
            "null"
        );
        assert_eq!(parse_wy_track_json(r#"{"id":1}"#).unwrap(), "null");
        let output = parse_wy_track_json(r#"{"id":1,"name":"Song","mv":"0"}"#).unwrap();
        let output: Value = serde_json::from_str(&output).unwrap();
        assert!(output.get("mvid").is_none());
    }

    #[test]
    fn md5_matches_standard_rfc_vectors() {
        use super::md5_hex;
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5_hex(b"a"), "0cc175b9c0f1b6a831c399e269772661");
        assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(md5_hex(b"message digest"), "f96b697d7cb7938d525a2f31aaf161d0");
    }

    #[test]
    fn kg_sign_computes_expected_hash() {
        use super::kg_sign;
        use std::collections::BTreeMap;
        let mut params = BTreeMap::new();
        params.insert("appid".to_string(), "1005".to_string());
        params.insert("mid".to_string(), "123".to_string());
        let sig = kg_sign(&params, "");
        assert_eq!(sig.len(), 32);
        // Salted md5 should be deterministic
        assert_eq!(sig, kg_sign(&params, ""));
    }

    #[test]
    fn maps_kg_track_and_qualities() {
        use super::parse_kg_track_json;
        let json = r#"{
            "FileHash": "F1234567890ABCDEF",
            "FileSize": 3145728,
            "HQFileHash": "H1234567890ABCDEF",
            "HQFileSize": 8388608,
            "SongName": "七里香",
            "Suffix": " (Live)",
            "SingerName": "周杰伦",
            "AlbumName": "七里香",
            "AlbumID": 1001,
            "Duration": 299,
            "MixSongID": "10001",
            "Singers": [{"name": "周杰伦", "id": 77}]
        }"#;
        let output = parse_kg_track_json(json).unwrap();
        let parsed: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["type"], "kg");
        assert_eq!(parsed["title"], "七里香 (Live)");
        assert_eq!(parsed["artist"], "周杰伦");
        assert_eq!(parsed["album"], "七里香");
        assert_eq!(parsed["duration"], 299000);
        assert_eq!(parsed["qualities"]["128k"]["filesize"], 3145728);
        assert_eq!(parsed["qualities"]["320k"]["filesize"], 8388608);
        assert_eq!(parsed["singers"][0]["name"], "周杰伦");
        assert_eq!(parsed["singers"][0]["singerId"], 77);
    }

    #[test]
    fn maps_kw_track_and_minfo() {
        use super::parse_kw_track_json;
        let json = r#"{
            "MUSICRID": "MUSIC_228908",
            "SONGNAME": "晴天",
            "ARTIST": "周杰伦",
            "ALBUM": "叶惠美",
            "ALBUMID": "88",
            "DURATION": 269,
            "N_MINFO": "level:ff,bitrate:2000,format:flac,size:28.5MB;level:p,bitrate:320,format:mp3,size:8.5MB;level:df,bitrate:128,format:mp3,size:3.5MB",
            "allartistid": "77",
            "HASMV": "1"
        }"#;
        let output = parse_kw_track_json(json).unwrap();
        let parsed: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["type"], "kw");
        assert_eq!(parsed["id"], 228908);
        assert_eq!(parsed["title"], "晴天");
        assert_eq!(parsed["artist"], "周杰伦");
        assert_eq!(parsed["album"], "叶惠美");
        assert_eq!(parsed["duration"], 269000);
        assert_eq!(parsed["qualities"]["flac"]["bitrate"], 2000);
        assert_eq!(parsed["qualities"]["320k"]["bitrate"], 320);
        assert_eq!(parsed["qualities"]["128k"]["bitrate"], 128);
        assert_eq!(parsed["singers"][0]["singerId"], 77);
        assert_eq!(parsed["mvid"], "228908");
    }
}
