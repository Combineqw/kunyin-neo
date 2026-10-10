//! Remote stream format negotiation, buffer watermarking, and shadow playback session.
//!
//! This module forms the contract-level boundary for remote streaming audio
//! migration. It deliberately does NOT decode remote streams or take over
//! Chromium's HTMLAudio playback. Instead, it provides a deterministic state
//! machine for URL/MIME negotiation, buffer watermarks (low/high watermark),
//! and seek state tracking that both the host protocol bridge and future native
//! pipelines can share.

use serde::{Deserialize, Serialize};

/// Supported or detected audio container types for remote streams.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioContainer {
    Mp3,
    Flac,
    Aac,
    M4a,
    Ogg,
    Opus,
    Wav,
    Wma,
    Ape,
    Unknown,
}

impl AudioContainer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Flac => "flac",
            Self::Aac => "aac",
            Self::M4a => "m4a",
            Self::Ogg => "ogg",
            Self::Opus => "opus",
            Self::Wav => "wav",
            Self::Wma => "wma",
            Self::Ape => "ape",
            Self::Unknown => "unknown",
        }
    }
}

/// Encryption scheme required to decode or stream the audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EncryptionScheme {
    None,
    Qmc2,
}

/// Negotiated remote stream format specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NegotiatedFormat {
    pub mime_type: String,
    pub container: String,
    pub encryption: EncryptionScheme,
    pub normalized_url: String,
    pub source_hint: String,
}

/// Normalize an upstream remote URL.
///
/// In particular, QQ music CDN URLs on `http://music.tc.qq.com` often hit broken
/// IPv6 routes on HTTP while HTTPS resolves properly and supports HTTP Range.
pub fn normalize_upstream_url(raw: &str) -> String {
    let lower = raw.trim();
    if lower.starts_with("http://music.tc.qq.com")
        || lower.starts_with("http://") && lower.contains(".music.tc.qq.com")
    {
        return format!("https://{}", &lower["http://".len()..]);
    }
    raw.to_string()
}

fn extension_from_url_path(url_str: &str) -> Option<&str> {
    let path = if let Some(idx) = url_str.find("://") {
        let after_scheme = &url_str[idx + 3..];
        let path_start = after_scheme.find('/')?;
        &after_scheme[path_start..]
    } else {
        url_str
    };

    let path_no_query = path.split('?').next().unwrap_or(path);
    let path_clean = path_no_query.split('#').next().unwrap_or(path_no_query);

    let last_slash = path_clean.rfind('/').unwrap_or(0);
    let filename = &path_clean[last_slash..];
    let dot = filename.rfind('.')?;
    Some(&filename[dot..])
}

fn container_and_mime_from_ext(ext: &str) -> Option<(&'static str, AudioContainer)> {
    let ext_lower = ext.to_ascii_lowercase();
    match ext_lower.as_str() {
        ".mp3" => Some(("audio/mpeg", AudioContainer::Mp3)),
        ".flac" => Some(("audio/flac", AudioContainer::Flac)),
        ".mflac" => Some(("audio/flac", AudioContainer::Flac)),
        ".mgg" => Some(("audio/mpeg", AudioContainer::Mp3)),
        ".wav" => Some(("audio/wav", AudioContainer::Wav)),
        ".m4a" => Some(("audio/mp4", AudioContainer::M4a)),
        ".mp4" => Some(("audio/mp4", AudioContainer::M4a)),
        ".aac" => Some(("audio/aac", AudioContainer::Aac)),
        ".ogg" => Some(("audio/ogg", AudioContainer::Ogg)),
        ".opus" => Some(("audio/ogg", AudioContainer::Opus)),
        ".wma" => Some(("audio/x-ms-wma", AudioContainer::Wma)),
        ".ape" => Some(("audio/x-ape", AudioContainer::Ape)),
        _ => None,
    }
}

/// Negotiate the effective MIME type, container, encryption, and normalized URL.
///
/// Precedence rules:
/// 1. If an explicit `content_type_hint` is provided and non-empty, use it.
/// 2. If the URL pathname has a recognized extension (e.g. `.flac`, `.mflac`, `.mp3`),
///    the extension takes precedence over upstream CDN Content-Type (CDNs often mislabel FLAC as audio/mpeg).
/// 3. If upstream `Content-Type` is recognized and non-generic, use it.
/// 4. Fallback to `audio/mpeg` (mp3).
pub fn negotiate_remote_format(
    url: &str,
    content_type_hint: Option<&str>,
    upstream_content_type: Option<&str>,
    ekey: Option<&str>,
) -> NegotiatedFormat {
    let normalized_url = normalize_upstream_url(url);
    let ext = extension_from_url_path(&normalized_url);
    let ext_info = ext.and_then(container_and_mime_from_ext);

    let has_qmc2_ext = ext
        .map(|e| e.eq_ignore_ascii_case(".mflac") || e.eq_ignore_ascii_case(".mgg"))
        .unwrap_or(false);
    let has_ekey = ekey.map(|k| !k.trim().is_empty()).unwrap_or(false);
    let encryption = if has_ekey || has_qmc2_ext {
        EncryptionScheme::Qmc2
    } else {
        EncryptionScheme::None
    };

    if let Some(hint) = content_type_hint.map(str::trim).filter(|h| !h.is_empty()) {
        let container = match hint {
            "audio/flac" => AudioContainer::Flac,
            "audio/mp4" => AudioContainer::M4a,
            "audio/ogg" => AudioContainer::Ogg,
            "audio/wav" => AudioContainer::Wav,
            "audio/aac" => AudioContainer::Aac,
            "audio/mpeg" => AudioContainer::Mp3,
            "audio/x-ms-wma" => AudioContainer::Wma,
            "audio/x-ape" => AudioContainer::Ape,
            _ => ext_info.map(|(_, c)| c).unwrap_or(AudioContainer::Unknown),
        };
        return NegotiatedFormat {
            mime_type: hint.to_string(),
            container: container.as_str().to_string(),
            encryption,
            normalized_url,
            source_hint: "explicit".to_string(),
        };
    }

    if let Some((mime, container)) = ext_info {
        return NegotiatedFormat {
            mime_type: mime.to_string(),
            container: container.as_str().to_string(),
            encryption,
            normalized_url,
            source_hint: "url_extension".to_string(),
        };
    }

    if let Some(upstream) = upstream_content_type
        .map(str::trim)
        .filter(|ct| !ct.is_empty() && *ct != "application/octet-stream")
    {
        let container = match upstream {
            "audio/flac" => AudioContainer::Flac,
            "audio/mp4" => AudioContainer::M4a,
            "audio/ogg" => AudioContainer::Ogg,
            "audio/wav" => AudioContainer::Wav,
            "audio/aac" => AudioContainer::Aac,
            "audio/mpeg" => AudioContainer::Mp3,
            "audio/x-ms-wma" => AudioContainer::Wma,
            "audio/x-ape" => AudioContainer::Ape,
            _ => AudioContainer::Unknown,
        };
        return NegotiatedFormat {
            mime_type: upstream.to_string(),
            container: container.as_str().to_string(),
            encryption,
            normalized_url,
            source_hint: "upstream_header".to_string(),
        };
    }

    NegotiatedFormat {
        mime_type: "audio/mpeg".to_string(),
        container: AudioContainer::Mp3.as_str().to_string(),
        encryption,
        normalized_url,
        source_hint: "fallback".to_string(),
    }
}

/// Status of the stream buffer watermark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WatermarkStatus {
    Buffering,
    Ready,
    HighWatermark,
    Ended,
}

/// Buffer watermark tracker for remote audio streaming.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BufferWatermark {
    pub low_watermark_bytes: u64,
    pub high_watermark_bytes: u64,
    pub total_bytes: Option<u64>,
    pub current_offset: u64,
    pub buffered_bytes: u64,
    pub total_received_bytes: u64,
    pub underflow_count: u32,
    pub status: WatermarkStatus,
}

impl Default for BufferWatermark {
    fn default() -> Self {
        Self {
            low_watermark_bytes: 256 * 1024,       // 256 KiB
            high_watermark_bytes: 2 * 1024 * 1024, // 2 MiB
            total_bytes: None,
            current_offset: 0,
            buffered_bytes: 0,
            total_received_bytes: 0,
            underflow_count: 0,
            status: WatermarkStatus::Buffering,
        }
    }
}

impl BufferWatermark {
    pub fn new(low_watermark_bytes: u64, high_watermark_bytes: u64) -> Self {
        Self {
            low_watermark_bytes: low_watermark_bytes.max(16 * 1024),
            high_watermark_bytes: high_watermark_bytes.max(low_watermark_bytes),
            ..Default::default()
        }
    }

    fn update_status(&mut self) {
        if let Some(total) = self.total_bytes {
            if self.current_offset.saturating_add(self.buffered_bytes) >= total && total > 0 {
                self.status = WatermarkStatus::Ended;
                return;
            }
        }

        if self.buffered_bytes >= self.high_watermark_bytes {
            self.status = WatermarkStatus::HighWatermark;
        } else if self.buffered_bytes >= self.low_watermark_bytes {
            self.status = WatermarkStatus::Ready;
        } else {
            self.status = WatermarkStatus::Buffering;
        }
    }

    pub fn set_total_bytes(&mut self, total: u64) {
        if total > 0 {
            self.total_bytes = Some(total);
            self.update_status();
        }
    }

    pub fn receive_chunk(&mut self, offset: u64, length: u64) {
        self.total_received_bytes = self.total_received_bytes.saturating_add(length);

        let current_end = self.current_offset.saturating_add(self.buffered_bytes);
        if offset == current_end {
            self.buffered_bytes = self.buffered_bytes.saturating_add(length);
        } else if offset >= self.current_offset && offset <= current_end {
            let chunk_end = offset.saturating_add(length);
            if chunk_end > current_end {
                self.buffered_bytes = chunk_end.saturating_sub(self.current_offset);
            }
        } else {
            // Non-contiguous chunk (e.g. following a seek or new Range request).
            self.current_offset = offset;
            self.buffered_bytes = length;
        }

        self.update_status();
    }

    pub fn consume_bytes(&mut self, length: u64) {
        let consumed = length.min(self.buffered_bytes);
        self.current_offset = self.current_offset.saturating_add(consumed);
        self.buffered_bytes = self.buffered_bytes.saturating_sub(consumed);

        if self.buffered_bytes < self.low_watermark_bytes
            && self.status != WatermarkStatus::Ended
            && self.status != WatermarkStatus::Buffering
        {
            self.underflow_count = self.underflow_count.saturating_add(1);
        }

        self.update_status();
    }

    pub fn reset_at_offset(&mut self, offset: u64) {
        self.current_offset = offset;
        self.buffered_bytes = 0;
        self.status = WatermarkStatus::Buffering;
    }
}

/// Lifecycle status for stream seeking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SeekStatus {
    Idle,
    Seeking,
    Buffering,
    Complete,
}

/// Tracks seek state and range requests for remote streams.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSeekState {
    pub status: SeekStatus,
    pub generation: u64,
    pub target_ms: u64,
    pub target_byte: Option<u64>,
    pub range_start: u64,
}

impl Default for RemoteSeekState {
    fn default() -> Self {
        Self {
            status: SeekStatus::Idle,
            generation: 0,
            target_ms: 0,
            target_byte: None,
            range_start: 0,
        }
    }
}

impl RemoteSeekState {
    pub fn request_seek(
        &mut self,
        target_ms: u64,
        duration_ms: u64,
        total_bytes: Option<u64>,
    ) -> u64 {
        self.generation = self.generation.saturating_add(1);
        self.status = SeekStatus::Seeking;
        self.target_ms = target_ms;

        self.target_byte = if let (Some(total), true) = (total_bytes, duration_ms > 0) {
            let clamped_ms = target_ms.min(duration_ms);
            Some((clamped_ms.saturating_mul(total)) / duration_ms)
        } else {
            None
        };

        self.range_start = self.target_byte.unwrap_or(0);
        self.generation
    }

    pub fn acknowledge_range(&mut self, generation: u64, start_offset: u64) {
        if generation == self.generation && self.status == SeekStatus::Seeking {
            self.range_start = start_offset;
            self.status = SeekStatus::Buffering;
        }
    }

    pub fn on_buffer_ready(&mut self, generation: u64) {
        if generation == self.generation
            && (self.status == SeekStatus::Seeking || self.status == SeekStatus::Buffering)
        {
            self.status = SeekStatus::Complete;
        }
    }

    pub fn cancel_seek(&mut self, generation: u64) {
        if generation == self.generation {
            self.status = SeekStatus::Idle;
        }
    }

    pub fn reset(&mut self) {
        self.status = SeekStatus::Idle;
        self.target_ms = 0;
        self.target_byte = None;
        self.range_start = 0;
    }
}

/// A half-open byte interval `[start, end)` representing a contiguous cached slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ByteRange {
    pub start: u64,
    pub end: u64,
}

impl ByteRange {
    pub fn new(start: u64, end: u64) -> Self {
        let (s, e) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        Self { start: s, end: e }
    }

    pub fn length(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    pub fn contains(&self, offset: u64) -> bool {
        offset >= self.start && offset < self.end
    }

    pub fn overlaps_or_adjacent(&self, other: &Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }

    pub fn merge(&self, other: &Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// Sorted, non-overlapping collection of contiguous byte ranges.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BufferedRanges {
    ranges: Vec<ByteRange>,
}

impl BufferedRanges {
    pub fn new() -> Self {
        Self { ranges: Vec::new() }
    }

    pub fn ranges(&self) -> &[ByteRange] {
        &self.ranges
    }

    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    pub fn count(&self) -> usize {
        self.ranges.len()
    }

    pub fn total_bytes(&self) -> u64 {
        self.ranges.iter().map(|r| r.length()).sum()
    }

    pub fn contains_offset(&self, offset: u64) -> bool {
        self.ranges.iter().any(|r| r.contains(offset))
    }

    /// Insert a received range [start, start + length) and coalesce overlapping or adjacent intervals.
    pub fn insert(&mut self, start: u64, length: u64) {
        if length == 0 {
            return;
        }
        let incoming = ByteRange::new(start, start.saturating_add(length));
        let mut new_ranges = Vec::with_capacity(self.ranges.len() + 1);
        let mut merged = incoming;
        let mut inserted = false;

        for r in &self.ranges {
            if merged.overlaps_or_adjacent(r) {
                merged = merged.merge(r);
            } else if r.start > merged.end {
                if !inserted {
                    new_ranges.push(merged);
                    inserted = true;
                }
                new_ranges.push(*r);
            } else {
                new_ranges.push(*r);
            }
        }

        if !inserted {
            new_ranges.push(merged);
        }

        self.ranges = new_ranges;
    }

    /// Return continuous contiguous bytes buffered starting from `offset`.
    pub fn contiguous_bytes_from(&self, offset: u64) -> u64 {
        for r in &self.ranges {
            if r.contains(offset) {
                return r.end.saturating_sub(offset);
            }
        }
        0
    }

    /// Find the next unbuffered gap starting from or after `from_offset`.
    pub fn find_next_gap(
        &self,
        from_offset: u64,
        max_gap_bytes: u64,
        total_bytes: Option<u64>,
    ) -> Option<ByteRange> {
        let mut probe = from_offset;
        for r in &self.ranges {
            if r.end <= probe {
                continue;
            }
            if r.start <= probe {
                // Currently inside a buffered range; jump probe to its end
                probe = r.end;
            } else {
                // Gap between probe and r.start
                let gap_start = probe;
                let desired_end = gap_start.saturating_add(max_gap_bytes);
                let gap_end = desired_end.min(r.start);
                let effective_end = if let Some(tot) = total_bytes {
                    gap_end.min(tot)
                } else {
                    gap_end
                };
                if effective_end > gap_start {
                    return Some(ByteRange::new(gap_start, effective_end));
                }
                return None;
            }
        }

        // Beyond all buffered ranges
        let gap_start = probe;
        let desired_end = gap_start.saturating_add(max_gap_bytes);
        let effective_end = if let Some(tot) = total_bytes {
            if gap_start >= tot {
                return None;
            }
            desired_end.min(tot)
        } else {
            desired_end
        };

        if effective_end > gap_start {
            Some(ByteRange::new(gap_start, effective_end))
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.ranges.clear();
    }
}

/// Preload urgency based on forward buffer depth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreloadPriority {
    /// Forward buffer is below critical threshold (< 64 KiB) or empty.
    Critical,
    /// Forward buffer is below low watermark; fetch promptly.
    High,
    /// Normal streaming window prefetching.
    Normal,
    /// Forward buffer is above high watermark; prefetch may pause.
    Idle,
}

/// A planned HTTP Range request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RangePlan {
    pub range_start: u64,
    pub range_end: u64,
    pub priority: PreloadPriority,
    pub is_cached: bool,
    pub chunk_size: u64,
    pub http_range_header: String,
}

/// Result of calculating a seek target byte range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeekRangeCalculation {
    pub target_ms: u64,
    pub estimated_byte: u64,
    pub aligned_range_start: u64,
    pub planned_range_end: u64,
    pub http_range_header: String,
}

/// Planner for stream chunk preloading and HTTP Range boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamRangePlanner {
    pub default_chunk_size: u64,
    pub min_chunk_size: u64,
    pub max_chunk_size: u64,
    pub alignment_bytes: u64,
}

impl Default for StreamRangePlanner {
    fn default() -> Self {
        Self {
            default_chunk_size: 512 * 1024,  // 512 KiB
            min_chunk_size: 64 * 1024,       // 64 KiB
            max_chunk_size: 2 * 1024 * 1024, // 2 MiB
            alignment_bytes: 64 * 1024,      // 64 KiB
        }
    }
}

impl StreamRangePlanner {
    pub fn new(default_chunk_size: u64, alignment_bytes: u64) -> Self {
        let align = alignment_bytes.max(1024);
        Self {
            default_chunk_size: default_chunk_size.max(align),
            alignment_bytes: align,
            ..Default::default()
        }
    }

    /// Plan the next HTTP Range request given the current playback offset and buffered ranges.
    pub fn plan_next_range(
        &self,
        current_offset: u64,
        buffered: &BufferedRanges,
        total_bytes: Option<u64>,
        low_watermark: u64,
        high_watermark: u64,
        requested_chunk_size: Option<u64>,
    ) -> Option<RangePlan> {
        let chunk_size = requested_chunk_size
            .unwrap_or(self.default_chunk_size)
            .clamp(self.min_chunk_size, self.max_chunk_size);

        let contiguous_forward = buffered.contiguous_bytes_from(current_offset);
        let priority = if contiguous_forward < self.min_chunk_size {
            PreloadPriority::Critical
        } else if contiguous_forward < low_watermark {
            PreloadPriority::High
        } else if contiguous_forward >= high_watermark {
            PreloadPriority::Idle
        } else {
            PreloadPriority::Normal
        };

        let gap = buffered.find_next_gap(current_offset, chunk_size, total_bytes)?;
        let is_cached = buffered.contains_offset(gap.start);

        let aligned_start = (gap.start / self.alignment_bytes) * self.alignment_bytes;
        let start = if aligned_start >= current_offset {
            aligned_start
        } else {
            gap.start
        };

        let raw_end = start.saturating_add(chunk_size);
        let end = if let Some(tot) = total_bytes {
            raw_end.min(tot)
        } else {
            raw_end
        };

        if end <= start {
            return None;
        }

        let http_range_header = format!("bytes={}-{}", start, end.saturating_sub(1));

        Some(RangePlan {
            range_start: start,
            range_end: end,
            priority,
            is_cached,
            chunk_size: end.saturating_sub(start),
            http_range_header,
        })
    }

    /// Calculate seek byte offset with alignment and duration scaling.
    pub fn calculate_seek_range(
        &self,
        target_ms: u64,
        duration_ms: u64,
        total_bytes: Option<u64>,
        requested_align: Option<u64>,
    ) -> SeekRangeCalculation {
        let align = requested_align.unwrap_or(self.alignment_bytes).max(1024);

        let estimated_byte = if let (Some(tot), true) = (total_bytes, duration_ms > 0) {
            let clamped_ms = target_ms.min(duration_ms);
            (clamped_ms.saturating_mul(tot)) / duration_ms
        } else {
            0
        };

        let aligned_start = (estimated_byte / align) * align;
        let raw_end = aligned_start.saturating_add(self.default_chunk_size);
        let planned_end = if let Some(tot) = total_bytes {
            raw_end.min(tot)
        } else {
            raw_end
        };

        let http_range_header =
            format!("bytes={}-{}", aligned_start, planned_end.saturating_sub(1));

        SeekRangeCalculation {
            target_ms,
            estimated_byte,
            aligned_range_start: aligned_start,
            planned_range_end: planned_end,
            http_range_header,
        }
    }
}

/// Overall playback state of the shadow session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackState {
    Idle,
    Buffering,
    Ready,
    Playing,
    Paused,
    Ended,
}

/// Serializable snapshot of the shadow playback session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePlaybackShadowSnapshot {
    pub session_id: String,
    pub format: NegotiatedFormat,
    pub status: PlaybackState,
    pub total_bytes: Option<u64>,
    pub current_offset: u64,
    pub buffered_bytes: u64,
    pub low_watermark_bytes: u64,
    pub high_watermark_bytes: u64,
    pub watermark_status: WatermarkStatus,
    pub seek_status: SeekStatus,
    pub seek_generation: u64,
    pub seek_target_ms: u64,
    pub seek_target_byte: Option<u64>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub underflow_count: u32,
    pub buffered_range_count: u32,
    pub total_buffered_bytes: u64,
    pub is_current_cached: bool,
}

/// Testable shadow playback session for remote stream migration.
///
/// This maintains state alongside the existing Electron `kunyin://` handler.
/// It verifies URL/MIME negotiation, buffer watermarking, discontinuous range tracking,
/// chunk preloading, and seek transitions without interrupting the proven Chromium HTMLAudio playback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemotePlaybackShadowSession {
    pub session_id: String,
    pub format: NegotiatedFormat,
    pub state: PlaybackState,
    pub buffer: BufferWatermark,
    pub seek: RemoteSeekState,
    pub buffered_ranges: BufferedRanges,
    pub range_planner: StreamRangePlanner,
    pub position_ms: u64,
    pub duration_ms: u64,
}

impl RemotePlaybackShadowSession {
    pub fn new(
        session_id: impl Into<String>,
        url: &str,
        content_type_hint: Option<&str>,
        ekey: Option<&str>,
        duration_ms: u64,
    ) -> Self {
        let format = negotiate_remote_format(url, content_type_hint, None, ekey);
        Self {
            session_id: session_id.into(),
            format,
            state: PlaybackState::Buffering,
            buffer: BufferWatermark::default(),
            seek: RemoteSeekState::default(),
            buffered_ranges: BufferedRanges::new(),
            range_planner: StreamRangePlanner::default(),
            position_ms: 0,
            duration_ms,
        }
    }

    pub fn snapshot(&self) -> RemotePlaybackShadowSnapshot {
        RemotePlaybackShadowSnapshot {
            session_id: self.session_id.clone(),
            format: self.format.clone(),
            status: self.state,
            total_bytes: self.buffer.total_bytes,
            current_offset: self.buffer.current_offset,
            buffered_bytes: self.buffer.buffered_bytes,
            low_watermark_bytes: self.buffer.low_watermark_bytes,
            high_watermark_bytes: self.buffer.high_watermark_bytes,
            watermark_status: self.buffer.status,
            seek_status: self.seek.status,
            seek_generation: self.seek.generation,
            seek_target_ms: self.seek.target_ms,
            seek_target_byte: self.seek.target_byte,
            position_ms: self.position_ms,
            duration_ms: self.duration_ms,
            underflow_count: self.buffer.underflow_count,
            buffered_range_count: self.buffered_ranges.count() as u32,
            total_buffered_bytes: self.buffered_ranges.total_bytes(),
            is_current_cached: self
                .buffered_ranges
                .contains_offset(self.buffer.current_offset),
        }
    }

    /// Update with incoming HTTP upstream response status and headers.
    pub fn update_response(
        &mut self,
        _status_code: u16,
        content_length: Option<u64>,
        content_range: Option<&str>,
        upstream_content_type: Option<&str>,
    ) -> RemotePlaybackShadowSnapshot {
        if let Some(range_header) = content_range {
            // e.g. "bytes 0-1000/50000" or "bytes 1000-2000/*"
            if let Some(slash_idx) = range_header.rfind('/') {
                let total_part = &range_header[slash_idx + 1..].trim();
                if let Ok(total) = total_part.parse::<u64>() {
                    self.buffer.set_total_bytes(total);
                }
            }

            if let Some(bytes_idx) = range_header.find("bytes ") {
                let range_part = &range_header[bytes_idx + 6..];
                if let Some(dash_idx) = range_part.find('-') {
                    if let Ok(start) = range_part[..dash_idx].trim().parse::<u64>() {
                        self.seek.acknowledge_range(self.seek.generation, start);
                        if self.buffer.current_offset != start {
                            self.buffer.reset_at_offset(start);
                        }
                    }
                }
            }
        } else if let Some(cl) = content_length {
            self.buffer.set_total_bytes(cl);
        }

        if self.format.source_hint == "fallback" {
            if let Some(upstream_ct) = upstream_content_type {
                let updated = negotiate_remote_format(
                    &self.format.normalized_url,
                    None,
                    Some(upstream_ct),
                    match self.format.encryption {
                        EncryptionScheme::Qmc2 => Some("qmc2"),
                        EncryptionScheme::None => None,
                    },
                );
                if updated.source_hint != "fallback" {
                    self.format = updated;
                }
            }
        }

        self.snapshot()
    }

    /// Record a received stream chunk and update watermark, range tracker, and seek progress.
    pub fn receive_chunk(&mut self, offset: u64, length: u64) -> RemotePlaybackShadowSnapshot {
        self.buffer.receive_chunk(offset, length);
        self.buffered_ranges.insert(offset, length);

        if self.seek.status == SeekStatus::Seeking || self.seek.status == SeekStatus::Buffering {
            if self.buffer.status == WatermarkStatus::Ready
                || self.buffer.status == WatermarkStatus::HighWatermark
                || self.buffer.status == WatermarkStatus::Ended
            {
                self.seek.on_buffer_ready(self.seek.generation);
                if self.state == PlaybackState::Buffering {
                    self.state = PlaybackState::Ready;
                }
            }
        } else if self.state == PlaybackState::Buffering
            && (self.buffer.status == WatermarkStatus::Ready
                || self.buffer.status == WatermarkStatus::HighWatermark
                || self.buffer.status == WatermarkStatus::Ended)
        {
            self.state = PlaybackState::Ready;
        }

        self.snapshot()
    }

    /// Request a seek to `target_ms`.
    pub fn seek(&mut self, target_ms: u64, duration_ms: u64) -> RemotePlaybackShadowSnapshot {
        self.duration_ms = duration_ms.max(self.duration_ms);
        self.position_ms = target_ms.min(self.duration_ms);

        self.seek
            .request_seek(target_ms, self.duration_ms, self.buffer.total_bytes);

        if let Some(target_byte) = self.seek.target_byte {
            self.buffer.reset_at_offset(target_byte);
        } else {
            self.buffer.reset_at_offset(0);
        }

        self.state = PlaybackState::Buffering;
        self.snapshot()
    }

    /// Update user playback intent (play vs pause).
    pub fn set_play_state(&mut self, playing: bool) -> RemotePlaybackShadowSnapshot {
        if playing {
            if self.buffer.status == WatermarkStatus::Buffering {
                self.state = PlaybackState::Buffering;
            } else {
                self.state = PlaybackState::Playing;
            }
        } else {
            self.state = PlaybackState::Paused;
        }
        self.snapshot()
    }

    /// Advance playback timeline and optionally consume buffered bytes.
    pub fn advance_playback(
        &mut self,
        elapsed_ms: u64,
        consumed_bytes: Option<u64>,
    ) -> RemotePlaybackShadowSnapshot {
        if self.state == PlaybackState::Playing {
            self.position_ms = self
                .position_ms
                .saturating_add(elapsed_ms)
                .min(self.duration_ms);

            if let Some(bytes) = consumed_bytes {
                self.buffer.consume_bytes(bytes);
            }

            if self.buffer.status == WatermarkStatus::Buffering {
                self.state = PlaybackState::Buffering;
            } else if self.position_ms >= self.duration_ms && self.duration_ms > 0 {
                self.state = PlaybackState::Ended;
            }
        }
        self.snapshot()
    }

    /// Plan the next HTTP Range request based on current offset and buffered chunks.
    pub fn plan_next_chunk(&self, chunk_size: Option<u64>) -> Option<RangePlan> {
        self.range_planner.plan_next_range(
            self.buffer.current_offset,
            &self.buffered_ranges,
            self.buffer.total_bytes,
            self.buffer.low_watermark_bytes,
            self.buffer.high_watermark_bytes,
            chunk_size,
        )
    }

    /// Calculate aligned seek byte range for given time target.
    pub fn calculate_seek_range(
        &self,
        target_ms: u64,
        align_bytes: Option<u64>,
    ) -> SeekRangeCalculation {
        self.range_planner.calculate_seek_range(
            target_ms,
            self.duration_ms,
            self.buffer.total_bytes,
            align_bytes,
        )
    }

    /// Query currently buffered intervals.
    pub fn get_buffered_ranges(&self) -> Vec<ByteRange> {
        self.buffered_ranges.ranges().to_vec()
    }

    /// Check if a specific byte offset is cached.
    pub fn is_offset_cached(&self, offset: u64) -> bool {
        self.buffered_ranges.contains_offset(offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_negotiation_normalizes_qq_url_and_detects_qmc2_flac() {
        let raw = "http://music.tc.qq.com/C40000test.mflac?vkey=secret";
        let format = negotiate_remote_format(raw, None, Some("audio/mpeg"), None);
        assert_eq!(
            format.normalized_url,
            "https://music.tc.qq.com/C40000test.mflac?vkey=secret"
        );
        assert_eq!(format.mime_type, "audio/flac");
        assert_eq!(format.container, "flac");
        assert_eq!(format.encryption, EncryptionScheme::Qmc2);
        assert_eq!(format.source_hint, "url_extension");
    }

    #[test]
    fn format_negotiation_respects_explicit_hint() {
        let raw = "https://cdn.example.com/audio/track?id=123";
        let format = negotiate_remote_format(raw, Some("audio/wav"), None, None);
        assert_eq!(format.mime_type, "audio/wav");
        assert_eq!(format.container, "wav");
        assert_eq!(format.encryption, EncryptionScheme::None);
        assert_eq!(format.source_hint, "explicit");
    }

    #[test]
    fn format_negotiation_falls_back_to_upstream_header_and_default() {
        let raw = "https://cdn.example.com/audio/stream";
        let format1 = negotiate_remote_format(raw, None, Some("audio/ogg"), None);
        assert_eq!(format1.mime_type, "audio/ogg");
        assert_eq!(format1.container, "ogg");
        assert_eq!(format1.source_hint, "upstream_header");

        let format2 = negotiate_remote_format(raw, None, None, None);
        assert_eq!(format2.mime_type, "audio/mpeg");
        assert_eq!(format2.container, "mp3");
        assert_eq!(format2.source_hint, "fallback");
    }

    #[test]
    fn buffer_watermark_transitions_and_detects_underflow() {
        let mut watermark = BufferWatermark::new(64 * 1024, 512 * 1024);
        watermark.set_total_bytes(1024 * 1024);
        assert_eq!(watermark.status, WatermarkStatus::Buffering);

        watermark.receive_chunk(0, 32 * 1024);
        assert_eq!(watermark.status, WatermarkStatus::Buffering);

        watermark.receive_chunk(32 * 1024, 32 * 1024);
        assert_eq!(watermark.status, WatermarkStatus::Ready);

        watermark.receive_chunk(64 * 1024, 512 * 1024);
        assert_eq!(watermark.status, WatermarkStatus::HighWatermark);

        // Consume almost all bytes down to underflow
        watermark.consume_bytes(550 * 1024);
        assert_eq!(watermark.status, WatermarkStatus::Buffering);
        assert_eq!(watermark.underflow_count, 1);
    }

    #[test]
    fn seek_state_calculates_target_byte_and_advances_generation() {
        let mut seek = RemoteSeekState::default();
        let gen1 = seek.request_seek(30_000, 60_000, Some(10_000_000));
        assert_eq!(gen1, 1);
        assert_eq!(seek.status, SeekStatus::Seeking);
        assert_eq!(seek.target_byte, Some(5_000_000));
        assert_eq!(seek.range_start, 5_000_000);

        seek.acknowledge_range(gen1, 5_000_000);
        assert_eq!(seek.status, SeekStatus::Buffering);

        seek.on_buffer_ready(gen1);
        assert_eq!(seek.status, SeekStatus::Complete);

        // Stale generation acknowledgments are ignored
        seek.acknowledge_range(999, 100);
        assert_eq!(seek.status, SeekStatus::Complete);
    }

    #[test]
    fn shadow_session_full_lifecycle_and_seek_buffering() {
        let mut session = RemotePlaybackShadowSession::new(
            "stream-123",
            "http://music.tc.qq.com/song.mflac",
            None,
            Some("dummy_ekey"),
            60_000,
        );

        assert_eq!(session.state, PlaybackState::Buffering);
        assert_eq!(session.format.encryption, EncryptionScheme::Qmc2);

        // Receive response headers
        let snap1 = session.update_response(
            206,
            Some(10_000_000),
            Some("bytes 0-9999999/10000000"),
            Some("audio/flac"),
        );
        assert_eq!(snap1.total_bytes, Some(10_000_000));

        // Receive chunk reaching low watermark
        let snap2 = session.receive_chunk(0, 300 * 1024);
        assert_eq!(snap2.watermark_status, WatermarkStatus::Ready);
        assert_eq!(snap2.status, PlaybackState::Ready);

        // Play and advance
        session.set_play_state(true);
        assert_eq!(session.state, PlaybackState::Playing);
        session.advance_playback(1_000, Some(50 * 1024));
        assert_eq!(session.position_ms, 1_000);

        // Seek
        let snap3 = session.seek(30_000, 60_000);
        assert_eq!(snap3.status, PlaybackState::Buffering);
        assert_eq!(snap3.seek_status, SeekStatus::Seeking);
        assert_eq!(snap3.seek_target_byte, Some(5_000_000));

        // Range response arrives for seek
        session.update_response(
            206,
            Some(5_000_000),
            Some("bytes 5000000-9999999/10000000"),
            None,
        );
        assert_eq!(session.seek.status, SeekStatus::Buffering);

        // First chunk at new offset reaches watermark
        session.receive_chunk(5_000_000, 300 * 1024);
        assert_eq!(session.seek.status, SeekStatus::Complete);
        assert_eq!(session.state, PlaybackState::Ready);
        assert!(session.is_offset_cached(5_000_000));
        assert_eq!(session.snapshot().buffered_range_count, 2);
    }

    #[test]
    fn buffered_ranges_coalesce_and_find_gaps() {
        let mut ranges = BufferedRanges::new();
        assert!(ranges.is_empty());
        assert_eq!(ranges.total_bytes(), 0);

        // Insert first chunk [0, 100_000)
        ranges.insert(0, 100_000);
        assert_eq!(ranges.count(), 1);
        assert_eq!(ranges.total_bytes(), 100_000);
        assert!(ranges.contains_offset(50_000));
        assert!(!ranges.contains_offset(100_000));
        assert_eq!(ranges.contiguous_bytes_from(0), 100_000);

        // Insert adjacent chunk [100_000, 200_000) -> merges into [0, 200_000)
        ranges.insert(100_000, 100_000);
        assert_eq!(ranges.count(), 1);
        assert_eq!(ranges.ranges()[0], ByteRange::new(0, 200_000));

        // Insert disjoint chunk [500_000, 600_000) -> 2 ranges
        ranges.insert(500_000, 100_000);
        assert_eq!(ranges.count(), 2);
        assert_eq!(ranges.total_bytes(), 300_000);

        // Find gap starting from 0: inside [0, 200_000), gap should start at 200_000
        let gap1 = ranges.find_next_gap(0, 500_000, Some(1_000_000));
        assert_eq!(gap1, Some(ByteRange::new(200_000, 500_000)));

        // Find gap beyond all buffered ranges
        let gap2 = ranges.find_next_gap(550_000, 200_000, Some(1_000_000));
        assert_eq!(gap2, Some(ByteRange::new(600_000, 800_000)));

        // Gap capped by total_bytes
        let gap3 = ranges.find_next_gap(900_000, 300_000, Some(1_000_000));
        assert_eq!(gap3, Some(ByteRange::new(900_000, 1_000_000)));

        // Beyond total_bytes returns None
        assert_eq!(
            ranges.find_next_gap(1_000_000, 100_000, Some(1_000_000)),
            None
        );
    }

    #[test]
    fn stream_range_planner_calculates_priorities_and_aligns_chunks() {
        let planner = StreamRangePlanner::new(512 * 1024, 64 * 1024);
        let mut ranges = BufferedRanges::new();

        // 1. Initial state (empty buffer): forward = 0 < 64KB -> Critical
        let plan1 = planner.plan_next_range(
            0,
            &ranges,
            Some(10_000_000),
            256 * 1024,
            2 * 1024 * 1024,
            None,
        );
        let p1 = plan1.expect("plan exists");
        assert_eq!(p1.range_start, 0);
        assert_eq!(p1.range_end, 512 * 1024);
        assert_eq!(p1.priority, PreloadPriority::Critical);
        assert_eq!(p1.http_range_header, format!("bytes=0-{}", 512 * 1024 - 1));

        // 2. Buffer 128KB (above critical but below low watermark 256KB) -> High
        ranges.insert(0, 128 * 1024);
        let plan2 = planner.plan_next_range(
            0,
            &ranges,
            Some(10_000_000),
            256 * 1024,
            2 * 1024 * 1024,
            None,
        );
        let p2 = plan2.expect("plan exists");
        assert_eq!(p2.range_start, 128 * 1024);
        assert_eq!(p2.priority, PreloadPriority::High);

        // 3. Buffer 1 MB (between low 256KB and high 2MB) -> Normal
        ranges.insert(128 * 1024, 896 * 1024);
        let plan3 = planner.plan_next_range(
            0,
            &ranges,
            Some(10_000_000),
            256 * 1024,
            2 * 1024 * 1024,
            None,
        );
        let p3 = plan3.expect("plan exists");
        assert_eq!(p3.range_start, 1024 * 1024);
        assert_eq!(p3.priority, PreloadPriority::Normal);

        // 4. Buffer > 2MB -> Idle
        ranges.insert(1024 * 1024, 2 * 1024 * 1024);
        let plan4 = planner.plan_next_range(
            0,
            &ranges,
            Some(10_000_000),
            256 * 1024,
            2 * 1024 * 1024,
            None,
        );
        let p4 = plan4.expect("plan exists");
        assert_eq!(p4.priority, PreloadPriority::Idle);
    }

    #[test]
    fn seek_range_calculation_aligns_to_blocks() {
        let planner = StreamRangePlanner::new(512 * 1024, 64 * 1024);
        let calc = planner.calculate_seek_range(30_000, 60_000, Some(10_000_000), Some(64 * 1024));
        assert_eq!(calc.target_ms, 30_000);
        assert_eq!(calc.estimated_byte, 5_000_000);
        // 5_000_000 / 65536 = 76.29 -> 76 * 65536 = 4_980_736
        assert_eq!(calc.aligned_range_start, 76 * 64 * 1024);
        assert_eq!(
            calc.planned_range_end,
            calc.aligned_range_start + 512 * 1024
        );
        assert_eq!(
            calc.http_range_header,
            format!(
                "bytes={}-{}",
                calc.aligned_range_start,
                calc.planned_range_end - 1
            )
        );
    }
}
