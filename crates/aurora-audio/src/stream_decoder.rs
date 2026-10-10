//! Streaming media decoder pipeline backed by StreamCache and Symphonia.
//!
//! Provides a pull-based, chunk-by-chunk stream decoder that consumes
//! non-contiguous or arriving HTTP Range chunks via `StreamCache`, probes
//! remote containers (MP3, FLAC, AAC, WAV, OGG), decodes into interleaved
//! f32 PCM frames, and pumps into `SharedPcmQueue` without full-file pre-buffering.

use std::fmt;
use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::Time;

use crate::cache::StreamCache;
use crate::{AudioStreamInfo, DecodedPcmChunk, PcmFormat, SharedPcmQueue};

/// Thread-safe media source adapter wrapping `Arc<Mutex<StreamCache>>`.
#[derive(Clone)]
pub struct SharedStreamSource {
    inner: Arc<Mutex<StreamCache>>,
}

impl SharedStreamSource {
    pub fn new(cache: Arc<Mutex<StreamCache>>) -> Self {
        Self { inner: cache }
    }

    fn with_cache_mut<T>(&mut self, f: impl FnOnce(&mut StreamCache) -> T) -> io::Result<T> {
        match self.inner.lock() {
            Ok(mut guard) => Ok(f(&mut guard)),
            Err(poisoned) => Ok(f(&mut poisoned.into_inner())),
        }
    }
}

impl Read for SharedStreamSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.with_cache_mut(|cache| cache.read(buf))?
    }
}

impl Seek for SharedStreamSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.with_cache_mut(|cache| cache.seek(pos))?
    }
}

impl MediaSource for SharedStreamSource {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        match self.inner.lock() {
            Ok(guard) => guard.total_length(),
            Err(poisoned) => poisoned.into_inner().total_length(),
        }
    }
}

/// Lifecycle state machine for the streaming decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StreamDecodeState {
    /// Awaiting enough initial stream bytes to probe container/codec
    AwaitingHeader,
    /// Probed and ready to decode packets
    Ready,
    /// Actively decoding packets
    Decoding,
    /// Network underflow: waiting for subsequent chunks to arrive
    Underflow,
    /// Stream reached EOF
    EndOfStream,
    /// Decoder or demuxer encountered unrecoverable error
    Failed,
}

/// Serializable snapshot of the stream decoder status and metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamDecoderSnapshot {
    pub stream_id: String,
    pub state: StreamDecodeState,
    pub format: Option<PcmFormat>,
    pub codec: Option<String>,
    pub duration_ms: Option<u64>,
    pub decoded_frames: u64,
    pub position_ms: u64,
    pub buffered_forward_bytes: u64,
    pub total_cached_bytes: u64,
}

/// Output of a single decode step.
#[derive(Debug, PartialEq)]
pub enum StreamDecodeOutput {
    /// A decoded PCM chunk ready for queue ingestion
    Chunk(DecodedPcmChunk),
    /// Underflow or header incomplete; caller should provide more network chunks
    NeedMoreData,
    /// Stream has completed
    EndOfStream,
}

/// Error returned by the streaming decoder.
#[derive(Debug)]
pub enum StreamDecodeError {
    Io(String),
    Unsupported(String),
    Invalid(String),
    Decoder(String),
    LockPoisoned,
    NoAudioTrack,
    SeekFailed(String),
}

impl fmt::Display for StreamDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "stream IO error: {msg}"),
            Self::Unsupported(msg) => write!(f, "unsupported stream: {msg}"),
            Self::Invalid(msg) => write!(f, "invalid stream: {msg}"),
            Self::Decoder(msg) => write!(f, "stream decoder error: {msg}"),
            Self::LockPoisoned => write!(f, "stream cache lock poisoned"),
            Self::NoAudioTrack => write!(f, "no audio track found in stream"),
            Self::SeekFailed(msg) => write!(f, "stream seek failed: {msg}"),
        }
    }
}

impl std::error::Error for StreamDecodeError {}

impl From<io::Error> for StreamDecodeError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

impl From<SymphoniaError> for StreamDecodeError {
    fn from(error: SymphoniaError) -> Self {
        match error {
            SymphoniaError::Unsupported(msg) => Self::Unsupported(msg.to_string()),
            SymphoniaError::DecodeError(msg) => Self::Invalid(msg.to_string()),
            SymphoniaError::IoError(err) => Self::Io(err.to_string()),
            other => Self::Decoder(other.to_string()),
        }
    }
}

/// Symphonia-backed streaming audio decoder.
///
/// Reads directly from a shared `StreamCache`, probes metadata as soon as
/// minimal header bytes arrive, and yields interleaved f32 PCM chunks on demand.
pub struct StreamDecoder {
    stream_id: String,
    cache: Arc<Mutex<StreamCache>>,
    container_hint: Option<String>,
    mime_hint: Option<String>,
    format_reader: Option<Box<dyn FormatReader>>,
    decoder: Option<Box<dyn AudioDecoder>>,
    track_id: u32,
    info: Option<AudioStreamInfo>,
    state: StreamDecodeState,
    decoded_frames: u64,
    min_probe_bytes: usize,
}

impl StreamDecoder {
    /// Create a new stream decoder bound to an existing shared `StreamCache`.
    pub fn new(
        stream_id: impl Into<String>,
        cache: Arc<Mutex<StreamCache>>,
        container_hint: Option<String>,
        mime_hint: Option<String>,
    ) -> Self {
        Self {
            stream_id: stream_id.into(),
            cache,
            container_hint,
            mime_hint,
            format_reader: None,
            decoder: None,
            track_id: 0,
            info: None,
            state: StreamDecodeState::AwaitingHeader,
            decoded_frames: 0,
            min_probe_bytes: 44, // Minimal WAV header size, MP3/FLAC will expand naturally
        }
    }

    /// Helper to construct decoder from an owned `StreamCache`.
    pub fn from_cache(
        cache: StreamCache,
        container_hint: Option<String>,
        mime_hint: Option<String>,
    ) -> (Self, Arc<Mutex<StreamCache>>) {
        let stream_id = cache.id.clone();
        let shared = Arc::new(Mutex::new(cache));
        let decoder = Self::new(stream_id, Arc::clone(&shared), container_hint, mime_hint);
        (decoder, shared)
    }

    /// Configure minimum bytes required before attempting header probe.
    pub fn set_min_probe_bytes(&mut self, bytes: usize) {
        self.min_probe_bytes = bytes.max(16);
    }

    /// Check if enough initial contiguous bytes exist to attempt probe.
    pub fn can_probe(&self) -> bool {
        let Ok(guard) = self.cache.lock() else {
            return false;
        };
        let available = guard.buffered_ranges().contiguous_bytes_from(0);
        let total = guard.total_length();
        let is_complete = total.map(|t| available >= t).unwrap_or(false);
        available >= (self.min_probe_bytes as u64) || is_complete
    }

    /// Attempt to probe the format and initialize the demuxer and codec.
    ///
    /// Returns `Ok(Some(info))` on success, `Ok(None)` if more header chunks
    /// are needed from the network, or `Err` if the format is invalid/unsupported.
    pub fn try_probe(&mut self) -> Result<Option<AudioStreamInfo>, StreamDecodeError> {
        if let Some(info) = &self.info {
            return Ok(Some(info.clone()));
        }

        let (available_bytes, total_bytes) = {
            let guard = self
                .cache
                .lock()
                .map_err(|_| StreamDecodeError::LockPoisoned)?;
            (
                guard.buffered_ranges().contiguous_bytes_from(0),
                guard.total_length(),
            )
        };

        let is_complete = total_bytes.map(|t| available_bytes >= t).unwrap_or(false);
        if available_bytes == 0 || (available_bytes < self.min_probe_bytes as u64 && !is_complete) {
            self.state = StreamDecodeState::AwaitingHeader;
            return Ok(None);
        }

        // Reset cursor to 0 before probe
        {
            let mut guard = self
                .cache
                .lock()
                .map_err(|_| StreamDecodeError::LockPoisoned)?;
            guard.seek(SeekFrom::Start(0))?;
        }

        let mut hint = Hint::new();
        if let Some(container) = &self.container_hint {
            hint.with_extension(container);
        }
        if let Some(mime) = &self.mime_hint {
            hint.mime_type(mime);
        }

        let source = SharedStreamSource::new(Arc::clone(&self.cache));
        let mss = MediaSourceStream::new(Box::new(source), Default::default());

        let probed = match symphonia::default::get_probe().probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        ) {
            Ok(probed) => probed,
            Err(SymphoniaError::IoError(_)) if !is_complete => {
                // Unexpected EOF on partial stream - need more chunks
                let mut guard = self
                    .cache
                    .lock()
                    .map_err(|_| StreamDecodeError::LockPoisoned)?;
                let _ = guard.seek(SeekFrom::Start(0));
                self.state = StreamDecodeState::AwaitingHeader;
                return Ok(None);
            }
            Err(SymphoniaError::DecodeError(_)) if !is_complete => {
                // Incomplete header - need more chunks
                let mut guard = self
                    .cache
                    .lock()
                    .map_err(|_| StreamDecodeError::LockPoisoned)?;
                let _ = guard.seek(SeekFrom::Start(0));
                self.state = StreamDecodeState::AwaitingHeader;
                return Ok(None);
            }
            Err(other) => {
                self.state = StreamDecodeState::Failed;
                return Err(other.into());
            }
        };

        let track = match probed.default_track(TrackType::Audio) {
            Some(t) => t,
            None => {
                self.state = StreamDecodeState::Failed;
                return Err(StreamDecodeError::NoAudioTrack);
            }
        };

        let track_id = track.id;
        let codec_params = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
            .cloned()
            .ok_or_else(|| {
                self.state = StreamDecodeState::Failed;
                StreamDecodeError::Unsupported("track has no audio parameters".to_string())
            })?;

        let sample_rate = codec_params.sample_rate.ok_or_else(|| {
            self.state = StreamDecodeState::Failed;
            StreamDecodeError::Unsupported("sample rate is unknown".to_string())
        })?;

        let channels = codec_params
            .channels
            .as_ref()
            .ok_or_else(|| {
                self.state = StreamDecodeState::Failed;
                StreamDecodeError::Unsupported("channel count is unknown".to_string())
            })?
            .count() as u16;

        if channels == 0 {
            self.state = StreamDecodeState::Failed;
            return Err(StreamDecodeError::Invalid(
                "track has zero channels".to_string(),
            ));
        }

        let duration_frames = track.num_frames;
        let codec = format!("{:?}", codec_params.codec);

        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(&codec_params, &AudioDecoderOptions::default())
            .map_err(|e| {
                self.state = StreamDecodeState::Failed;
                StreamDecodeError::Unsupported(e.to_string())
            })?;

        let info = AudioStreamInfo {
            format: PcmFormat {
                sample_rate,
                channels,
            },
            codec,
            duration_frames,
        };

        self.format_reader = Some(probed);
        self.decoder = Some(decoder);
        self.track_id = track_id;
        self.info = Some(info.clone());
        self.state = StreamDecodeState::Ready;

        Ok(Some(info))
    }

    /// Decode the next audio packet from the stream into PCM chunk.
    pub fn decode_next_chunk(&mut self) -> Result<StreamDecodeOutput, StreamDecodeError> {
        if self.state == StreamDecodeState::EndOfStream {
            return Ok(StreamDecodeOutput::EndOfStream);
        }

        if self.format_reader.is_none() && self.try_probe()?.is_none() {
            return Ok(StreamDecodeOutput::NeedMoreData);
        }

        // Inspect cache buffer status
        let (contiguous, at_eof) = {
            let guard = self
                .cache
                .lock()
                .map_err(|_| StreamDecodeError::LockPoisoned)?;
            let cursor = guard.current_cursor();
            let cont = guard.buffered_ranges().contiguous_bytes_from(cursor);
            let byte_eof = if let Some(total) = guard.total_length() {
                cursor >= total
            } else {
                false
            };
            let frame_eof = self
                .info
                .as_ref()
                .and_then(|i| i.duration_frames)
                .map(|df| df > 0 && self.decoded_frames >= df)
                .unwrap_or(false);
            (cont, byte_eof || frame_eof)
        };

        if contiguous == 0 && !at_eof {
            self.state = StreamDecodeState::Underflow;
            return Ok(StreamDecodeOutput::NeedMoreData);
        }

        let format_reader = match &mut self.format_reader {
            Some(reader) => reader,
            None => return Ok(StreamDecodeOutput::NeedMoreData),
        };

        loop {
            let packet = match format_reader.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => {
                    self.state = StreamDecodeState::EndOfStream;
                    return Ok(StreamDecodeOutput::EndOfStream);
                }
                Err(SymphoniaError::ResetRequired) => {
                    self.state = StreamDecodeState::EndOfStream;
                    return Ok(StreamDecodeOutput::EndOfStream);
                }
                Err(SymphoniaError::IoError(_)) => {
                    if at_eof {
                        self.state = StreamDecodeState::EndOfStream;
                        return Ok(StreamDecodeOutput::EndOfStream);
                    } else {
                        self.state = StreamDecodeState::Underflow;
                        return Ok(StreamDecodeOutput::NeedMoreData);
                    }
                }
                Err(err) => {
                    self.state = StreamDecodeState::Failed;
                    return Err(err.into());
                }
            };

            if packet.track_id != self.track_id {
                continue;
            }

            let decoder = match &mut self.decoder {
                Some(dec) => dec,
                None => return Ok(StreamDecodeOutput::NeedMoreData),
            };

            match decoder.decode(&packet) {
                Ok(audio) => {
                    let pcm_format = self.info.as_ref().map(|i| i.format).unwrap_or(PcmFormat {
                        sample_rate: 44100,
                        channels: 2,
                    });
                    let mut samples = vec![0.0f32; audio.samples_interleaved()];
                    audio.copy_to_slice_interleaved(&mut samples);
                    let frames = if pcm_format.channels > 0 {
                        samples.len() / usize::from(pcm_format.channels)
                    } else {
                        0
                    };
                    self.decoded_frames = self.decoded_frames.saturating_add(frames as u64);
                    self.state = StreamDecodeState::Decoding;
                    return Ok(StreamDecodeOutput::Chunk(DecodedPcmChunk {
                        format: pcm_format,
                        frames,
                        samples,
                        end_of_stream: false,
                    }));
                }
                Err(SymphoniaError::DecodeError(_)) => {
                    // Frame error: skip and proceed to next packet
                    continue;
                }
                Err(err) => {
                    self.state = StreamDecodeState::Failed;
                    return Err(err.into());
                }
            }
        }
    }

    /// Decode chunks and push directly into a `SharedPcmQueue` up to `max_frames`.
    pub fn pump_to_queue(
        &mut self,
        queue: &SharedPcmQueue,
        max_frames: usize,
    ) -> Result<usize, StreamDecodeError> {
        let mut total_pushed = 0;
        while total_pushed < max_frames {
            match self.decode_next_chunk()? {
                StreamDecodeOutput::Chunk(chunk) => {
                    if chunk.frames == 0 {
                        break;
                    }
                    let pushed = queue.push_interleaved(&chunk.samples);
                    total_pushed += pushed;
                }
                StreamDecodeOutput::NeedMoreData | StreamDecodeOutput::EndOfStream => {
                    break;
                }
            }
        }
        Ok(total_pushed)
    }

    /// Seek stream to target timestamp in milliseconds.
    pub fn seek_ms(&mut self, target_ms: u64) -> Result<(), StreamDecodeError> {
        let format_reader = self.format_reader.as_mut().ok_or_else(|| {
            StreamDecodeError::Invalid("cannot seek before stream is probed".to_string())
        })?;

        let secs = (target_ms as f64) / 1000.0;
        let time =
            Time::try_from_secs_f64(secs).unwrap_or_else(|| Time::from((target_ms / 1000) as u32));
        let seek_to = SeekTo::Time {
            time,
            track_id: Some(self.track_id),
        };

        format_reader
            .seek(SeekMode::Accurate, seek_to)
            .or_else(|_| {
                format_reader.seek(
                    SeekMode::Coarse,
                    SeekTo::Time {
                        time,
                        track_id: Some(self.track_id),
                    },
                )
            })
            .map_err(|e| StreamDecodeError::SeekFailed(e.to_string()))?;

        if let Some(decoder) = &mut self.decoder {
            decoder.reset();
        }

        if let Some(info) = &self.info {
            self.decoded_frames = (target_ms * u64::from(info.format.sample_rate)) / 1000;
        }

        self.state = StreamDecodeState::Ready;
        Ok(())
    }

    /// Current lifecycle state.
    pub fn state(&self) -> StreamDecodeState {
        self.state
    }

    /// Probed stream format information, if available.
    pub fn info(&self) -> Option<&AudioStreamInfo> {
        self.info.as_ref()
    }

    /// Total audio frames decoded since start or seek.
    pub fn decoded_frames(&self) -> u64 {
        self.decoded_frames
    }

    /// Get current state snapshot and metrics.
    pub fn snapshot(&self) -> StreamDecoderSnapshot {
        let (buffered_forward_bytes, total_cached_bytes) = if let Ok(guard) = self.cache.lock() {
            (
                guard
                    .buffered_ranges()
                    .contiguous_bytes_from(guard.current_cursor()),
                guard.buffered_ranges().total_bytes(),
            )
        } else {
            (0, 0)
        };

        let sample_rate = self
            .info
            .as_ref()
            .map(|i| i.format.sample_rate)
            .unwrap_or(44100);
        let position_ms = if sample_rate > 0 {
            self.decoded_frames.saturating_mul(1000) / u64::from(sample_rate)
        } else {
            0
        };

        let duration_ms = self.info.as_ref().and_then(|i| {
            i.duration_frames.map(|f| {
                if i.format.sample_rate > 0 {
                    f.saturating_mul(1000) / u64::from(i.format.sample_rate)
                } else {
                    0
                }
            })
        });

        StreamDecoderSnapshot {
            stream_id: self.stream_id.clone(),
            state: self.state,
            format: self.info.as_ref().map(|i| i.format),
            codec: self.info.as_ref().map(|i| i.codec.clone()),
            duration_ms,
            decoded_frames: self.decoded_frames,
            position_ms,
            buffered_forward_bytes,
            total_cached_bytes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_wav_bytes(num_samples: usize) -> Vec<u8> {
        let mut data = Vec::new();
        let samples: Vec<i16> = (0..num_samples).map(|i| ((i % 100) as i16) * 300).collect();
        let data_len = (samples.len() * std::mem::size_of::<i16>()) as u32;
        let riff_len = 36 + data_len;

        data.extend_from_slice(b"RIFF");
        data.extend_from_slice(&riff_len.to_le_bytes());
        data.extend_from_slice(b"WAVEfmt ");
        data.extend_from_slice(&16u32.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes()); // PCM
        data.extend_from_slice(&2u16.to_le_bytes()); // 2 channels
        data.extend_from_slice(&44_100u32.to_le_bytes()); // 44.1 kHz
        data.extend_from_slice(&176_400u32.to_le_bytes()); // byte rate
        data.extend_from_slice(&4u16.to_le_bytes()); // block align
        data.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        data.extend_from_slice(b"data");
        data.extend_from_slice(&data_len.to_le_bytes());
        for sample in samples {
            data.extend_from_slice(&sample.to_le_bytes());
        }
        data
    }

    #[test]
    fn stream_decoder_probes_and_decodes_wav_stream_in_chunks() {
        let wav_data = build_test_wav_bytes(80);
        let mut cache = StreamCache::new("test-stream-wav", 1024 * 1024, None);
        cache.set_total_length(wav_data.len() as u64);
        cache.write_chunk(0, &wav_data).unwrap();

        let (mut decoder, _shared) = StreamDecoder::from_cache(
            cache,
            Some("wav".to_string()),
            Some("audio/wav".to_string()),
        );

        assert!(decoder.can_probe());
        let info = decoder.try_probe().unwrap().expect("probe should succeed");
        assert_eq!(info.format.sample_rate, 44_100);
        assert_eq!(info.format.channels, 2);
        assert_eq!(decoder.state(), StreamDecodeState::Ready);

        // Decode chunks
        let mut total_frames = 0;
        let queue = SharedPcmQueue::new(info.format, 2048);

        let pumped = decoder.pump_to_queue(&queue, 2048).unwrap();
        assert!(pumped > 0);
        total_frames += pumped;

        // Exhaust stream
        while let Ok(out) = decoder.decode_next_chunk() {
            match out {
                StreamDecodeOutput::Chunk(c) => total_frames += c.frames,
                StreamDecodeOutput::EndOfStream => break,
                StreamDecodeOutput::NeedMoreData => break,
            }
        }

        assert_eq!(total_frames, 40); // 80 samples / 2 channels = 40 frames
        let snap = decoder.snapshot();
        assert_eq!(snap.state, StreamDecodeState::EndOfStream);
        assert_eq!(snap.decoded_frames, 40);
        assert_eq!(snap.position_ms, (40 * 1000) / 44100);
    }

    #[test]
    fn stream_decoder_handles_underflow_and_resumes_when_chunk_arrives() {
        let wav_data = build_test_wav_bytes(80);
        let mut cache = StreamCache::new("test-underflow", 1024 * 1024, None);
        cache.set_total_length(wav_data.len() as u64);

        // Only write initial 44 bytes header
        cache.write_chunk(0, &wav_data[..44]).unwrap();

        let (mut decoder, shared) = StreamDecoder::from_cache(cache, Some("wav".to_string()), None);

        // Header probe succeeds
        let info = decoder.try_probe().unwrap().expect("probe should succeed");
        assert_eq!(info.format.channels, 2);

        // Attempting to decode next packet encounters underflow
        let output = decoder.decode_next_chunk().unwrap();
        assert_eq!(output, StreamDecodeOutput::NeedMoreData);
        assert_eq!(decoder.state(), StreamDecodeState::Underflow);

        // Subsequent chunk arrives from network
        {
            let mut guard = shared.lock().unwrap();
            guard.write_chunk(44, &wav_data[44..]).unwrap();
        }

        // Now decode resumes successfully!
        let output2 = decoder.decode_next_chunk().unwrap();
        assert!(matches!(output2, StreamDecodeOutput::Chunk(_)));
        assert_eq!(decoder.state(), StreamDecodeState::Decoding);
    }

    #[test]
    fn stream_decoder_pumps_to_queue_bounds_transport() {
        let wav_data = build_test_wav_bytes(200);
        let mut cache = StreamCache::new("test-pump", 1024 * 1024, None);
        cache.set_total_length(wav_data.len() as u64);
        cache.write_chunk(0, &wav_data).unwrap();

        let (mut decoder, _shared) =
            StreamDecoder::from_cache(cache, Some("wav".to_string()), None);

        let info = decoder.try_probe().unwrap().unwrap();
        let queue = SharedPcmQueue::new(info.format, 50); // Small capacity: 50 frames

        let pumped = decoder.pump_to_queue(&queue, 30).unwrap();
        assert_eq!(pumped, 100);
        assert_eq!(queue.queued_frames(), 50);
        assert_eq!(queue.dropped_frames(), 50);
    }
}
