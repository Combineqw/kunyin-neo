//! Read-only audio metadata extraction shared by the Electron and Tauri hosts.
//!
//! This is deliberately a read-only boundary.  The existing TypeScript tag
//! writers remain responsible for mutation until the Rust writer has parity
//! tests for every supported container.  The shape of [`AudioMetadata`] keeps
//! the same camelCase field names used by `src/main/tag/meta.ts` and the
//! scanner DTOs.

use std::path::Path;

use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::PictureType;
use lofty::probe::Probe;
use lofty::tag::{Accessor, ItemKey, Tag};
use serde::Serialize;

use crate::error::{AuroraError, Result};

/// ReplayGain values in their source units: dB for gains and linear values
/// for peaks.  These names intentionally match the shared renderer type.
#[derive(Debug, Clone, PartialEq, Serialize)]
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
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
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
}
