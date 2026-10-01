//! Read-only audio metadata extraction shared by the Electron and Tauri hosts.
//!
//! This is deliberately a read-only boundary.  The existing TypeScript tag
//! writers remain responsible for mutation until the Rust writer has parity
//! tests for every supported container.  The shape of [`AudioMetadata`] keeps
//! the same camelCase field names used by `src/main/tag/meta.ts` and the
//! scanner DTOs.

use std::path::Path;

use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey, Tag};
use serde::{Deserialize, Serialize};

use crate::error::{AuroraError, Result};

/// ReplayGain values in their source units: dB for gains and linear values
/// for peaks.  These names intentionally match the shared renderer type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayGainInfo {
    #[serde(rename = "trackDb", skip_serializing_if = "Option::is_none")]
    pub track_db: Option<f64>,
    #[serde(rename = "albumDb", skip_serializing_if = "Option::is_none")]
    pub album_db: Option<f64>,
    #[serde(rename = "trackPeak", skip_serializing_if = "Option::is_none")]
    pub track_peak: Option<f64>,
    #[serde(rename = "albumPeak", skip_serializing_if = "Option::is_none")]
    pub album_peak: Option<f64>,
}

impl ReplayGainInfo {
    fn is_empty(&self) -> bool {
        self.track_db.is_none()
            && self.album_db.is_none()
            && self.track_peak.is_none()
            && self.album_peak.is_none()
    }
}

/// Rust equivalent of the renderer's `MusicMeta` plus read-only properties
/// already returned by the native scanner.  Optional tag fields are omitted
/// when absent, preserving the current `{}`/partial-object behaviour.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_number: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lyrics: Option<String>,
    /// Duration in milliseconds, matching `BaseMusicItem.duration`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay_gain: Option<ReplayGainInfo>,
    /// Embedded cover bytes. JSON adapters serialize this as a byte array;
    /// native callers can use the strongly typed `Vec<u8>` value directly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub picture_data: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub picture_mime_type: Option<String>,
}

/// Compatibility alias for callers that use the TypeScript tag module name.
pub type MusicMeta = AudioMetadata;

fn parse_number(tag: &Tag, key: &ItemKey) -> Option<f64> {
    tag.get_string(key)
        .and_then(|value| value.split_whitespace().next())
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
}

fn replay_gain(tag: &Tag) -> Option<ReplayGainInfo> {
    let value = ReplayGainInfo {
        track_db: parse_number(tag, &ItemKey::ReplayGainTrackGain),
        album_db: parse_number(tag, &ItemKey::ReplayGainAlbumGain),
        track_peak: parse_number(tag, &ItemKey::ReplayGainTrackPeak),
        album_peak: parse_number(tag, &ItemKey::ReplayGainAlbumPeak),
    };
    (!value.is_empty()).then_some(value)
}

fn first_non_empty<'a>(values: impl Iterator<Item = &'a str>) -> Option<String> {
    values
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn track_number(tag: &Tag) -> Option<u32> {
    tag.get_string(&ItemKey::TrackNumber)
        .and_then(|value| value.split('/').next())
        .and_then(|value| value.trim().parse::<u32>().ok())
}

fn from_tag(tag: &Tag, duration: Option<u64>) -> AudioMetadata {
    let mut artists: Vec<String> = tag
        .get_strings(&ItemKey::TrackArtist)
        .chain(tag.get_strings(&ItemKey::TrackArtists))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    artists.dedup();
    let artist = (!artists.is_empty()).then(|| artists.join("、"));
    let lyrics = first_non_empty(tag.get_strings(&ItemKey::Lyrics));

    // Prefer a front cover, then retain the first picture for formats that do
    // not classify pictures (some MP4/APE files use `Other`).
    let picture = tag
        .get_picture_type(PictureType::CoverFront)
        .or_else(|| tag.pictures().first());

    AudioMetadata {
        title: tag
            .title()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty()),
        artist,
        album: tag
            .album()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty()),
        track_number: track_number(tag),
        lyrics,
        duration,
        replay_gain: replay_gain(tag),
        picture_data: picture.map(|value| value.data().to_vec()),
        picture_mime_type: picture
            .and_then(|value| value.mime_type())
            .map(|value| value.as_str().to_owned()),
    }
}

/// Extract metadata from a path. Unsupported extensions return `Ok(None)` to
/// match the existing TypeScript `readAudioTags()` facade. A recognized
/// extension with malformed bytes returns an error, allowing callers to keep
/// the existing fallback path and emit a diagnostic.
pub fn read_audio_tags(path: &Path) -> Result<Option<AudioMetadata>> {
    let supported = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "mp3" | "flac" | "ogg" | "opus" | "m4a" | "mp4" | "aac" | "wav" | "wma" | "ape"
            )
        })
        .unwrap_or(false);
    if !supported {
        return Ok(None);
    }

    let tagged = Probe::open(path)
        .and_then(|probe| probe.read())
        .map_err(|error| AuroraError::tag(path, error))?;
    let duration_ms = tagged.properties().duration().as_millis() as u64;
    // Some containers expose no duration until a decoder opens the stream.
    // Return it as absent so the host can retain its normal fallback path.
    let duration = (duration_ms > 0).then_some(duration_ms);
    Ok(tagged
        .primary_tag()
        .or_else(|| tagged.first_tag())
        .map(|tag| from_tag(tag, duration)))
}

/// JSON adapter used by N-API and Tauri commands.
pub fn read_audio_tags_json(path: &Path) -> Result<String> {
    Ok(serde_json::to_string(&read_audio_tags(path)?)?)
}

fn writable_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "mp3" | "flac" | "ogg" | "opus"
            )
        })
        .unwrap_or(false)
}

fn picture_mime(metadata: &AudioMetadata, data: &[u8]) -> MimeType {
    if let Some(mime) = metadata
        .picture_mime_type
        .as_deref()
        .map(MimeType::from_str)
    {
        return mime;
    }

    if data.starts_with(&[0xff, 0xd8, 0xff]) {
        MimeType::Jpeg
    } else if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        MimeType::Png
    } else {
        MimeType::Unknown("application/octet-stream".to_owned())
    }
}

fn write_optional_text(tag: &mut Tag, key: ItemKey, value: Option<&String>) {
    if let Some(value) = value
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        tag.insert_text(key, value.to_owned());
    }
}

/// Write the metadata fields supported by the native shadow path.
///
/// Only fields supplied by the caller are changed. Empty strings and invalid
/// track numbers are ignored so existing tags are retained, matching the
/// current TypeScript writers' partial-update semantics. A `false` result
/// means the container is intentionally left to the TypeScript fallback.
pub fn write_audio_tags(path: &Path, metadata: &AudioMetadata) -> Result<bool> {
    if !writable_extension(path) {
        return Ok(false);
    }

    let mut tagged = Probe::open(path)
        .and_then(|probe| probe.read())
        .map_err(|error| AuroraError::tag(path, error))?;

    if tagged.primary_tag().is_none() {
        let tag_type = tagged.primary_tag_type();
        if !tagged.supports_tag_type(tag_type) {
            return Ok(false);
        }
        tagged.insert_tag(Tag::new(tag_type));
    }

    let Some(tag) = tagged.primary_tag_mut() else {
        return Err(AuroraError::tag(path, "primary tag is unavailable"));
    };
    write_optional_text(tag, ItemKey::TrackTitle, metadata.title.as_ref());
    write_optional_text(tag, ItemKey::TrackArtist, metadata.artist.as_ref());
    write_optional_text(tag, ItemKey::AlbumTitle, metadata.album.as_ref());
    write_optional_text(tag, ItemKey::Lyrics, metadata.lyrics.as_ref());
    if let Some(track_number) = metadata.track_number.filter(|value| *value > 0) {
        tag.insert_text(ItemKey::TrackNumber, track_number.to_string());
    }

    if let Some(data) = metadata
        .picture_data
        .as_deref()
        .filter(|data| !data.is_empty())
    {
        tag.remove_picture_type(PictureType::CoverFront);
        tag.push_picture(Picture::new_unchecked(
            PictureType::CoverFront,
            Some(picture_mime(metadata, data)),
            None,
            data.to_vec(),
        ));
    }

    tagged
        .save_to_path(path, WriteOptions::default())
        .map_err(|error| AuroraError::tag(path, error))?;
    Ok(true)
}

/// JSON adapter used by the N-API writer bridge.
pub fn write_audio_tags_json(path: &Path, json: &str) -> Result<bool> {
    let metadata: AudioMetadata = serde_json::from_str(json)?;
    write_audio_tags(path, &metadata)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lofty::tag::{Tag, TagType};

    #[test]
    fn extracts_music_meta_shape_and_replaygain() {
        let mut tag = Tag::new(TagType::VorbisComments);
        tag.insert_text(ItemKey::TrackTitle, "  Song  ".into());
        tag.insert_text(ItemKey::TrackArtist, "Artist A".into());
        tag.insert_text(ItemKey::TrackArtists, "Artist B".into());
        tag.insert_text(ItemKey::AlbumTitle, "Album".into());
        tag.insert_text(ItemKey::TrackNumber, "2/10".into());
        tag.insert_text(ItemKey::Lyrics, "[00:01.00]Hello".into());
        tag.insert_text(ItemKey::ReplayGainTrackGain, "-7.5 dB".into());
        tag.insert_text(ItemKey::ReplayGainAlbumPeak, "0.98".into());

        let parsed = from_tag(&tag, Some(12_345));
        assert_eq!(parsed.title.as_deref(), Some("Song"));
        assert_eq!(parsed.artist.as_deref(), Some("Artist A、Artist B"));
        assert_eq!(parsed.album.as_deref(), Some("Album"));
        assert_eq!(parsed.track_number, Some(2));
        assert_eq!(parsed.lyrics.as_deref(), Some("[00:01.00]Hello"));
        assert_eq!(parsed.duration, Some(12_345));
        assert_eq!(
            parsed.replay_gain.as_ref().and_then(|v| v.track_db),
            Some(-7.5)
        );
        assert_eq!(
            parsed.replay_gain.as_ref().and_then(|v| v.album_peak),
            Some(0.98)
        );
    }

    #[test]
    fn unsupported_extension_is_a_noop() {
        assert_eq!(read_audio_tags(Path::new("cover.jpg")).unwrap(), None);
    }

    #[test]
    fn json_uses_renderer_field_names() {
        let mut tag = Tag::new(TagType::VorbisComments);
        tag.insert_text(ItemKey::TrackNumber, "3".into());
        let json = serde_json::to_value(from_tag(&tag, Some(1))).unwrap();
        assert_eq!(json.get("trackNumber").and_then(|v| v.as_u64()), Some(3));
        assert_eq!(json.get("duration").and_then(|v| v.as_u64()), Some(1));
        assert!(json.get("track_number").is_none());
    }

    #[test]
    fn writes_supported_mp3_and_flac_without_mutating_fixtures() {
        let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-assets/KUNYIN");
        let cases = [
            ("陈奕迅 - K歌之王.mp3", "aurora-core-write-test.mp3"),
            (
                "陈奕迅 - 富士山下 [16Bit-44.1kHz].flac",
                "aurora-core-write-test.flac",
            ),
        ];

        for (fixture, output_name) in cases {
            let source = fixture_root.join(fixture);
            if !source.is_file() {
                // Keep the library crate testable from a source-only checkout.
                continue;
            }
            let output =
                std::env::temp_dir().join(format!("{}-{}", std::process::id(), output_name));
            let _ = std::fs::remove_file(&output);
            std::fs::copy(&source, &output).expect("copy metadata fixture");

            let metadata = AudioMetadata {
                title: Some("Rust 写入标题".to_owned()),
                artist: Some("Rust 写入歌手".to_owned()),
                album: Some("Rust 写入专辑".to_owned()),
                track_number: Some(7),
                lyrics: Some("[00:01.00]Rust lyrics".to_owned()),
                ..AudioMetadata::default()
            };
            assert!(matches!(write_audio_tags(&output, &metadata), Ok(true)));
            let parsed = read_audio_tags(&output)
                .expect("read written metadata")
                .expect("written fixture should be supported");
            assert_eq!(parsed.title.as_deref(), metadata.title.as_deref());
            assert_eq!(parsed.artist.as_deref(), metadata.artist.as_deref());
            assert_eq!(parsed.album.as_deref(), metadata.album.as_deref());
            assert_eq!(parsed.track_number, metadata.track_number);
            assert_eq!(parsed.lyrics.as_deref(), metadata.lyrics.as_deref());
            std::fs::remove_file(output).expect("remove metadata fixture copy");
        }
    }
}
