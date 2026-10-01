//! Host-neutral boundary for the native audio migration.
//!
//! The current desktop product still renders through Chromium's
//! `HTMLAudioElement -> Web Audio -> AudioContext.destination` path.  This
//! crate deliberately reports that fact instead of pretending that a native
//! decoder or WASAPI endpoint already exists.  The bounded PCM queue is the
//! transport contract that a future decoder and output thread can share
//! without allowing a slow device to grow memory without limit.

use std::collections::VecDeque;
use std::fs::File;
use std::path::Path;

use serde::{Deserialize, Serialize};
use symphonia::core::audio::GenericAudioBufferRef;
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioOutputMode {
    HtmlAudioFallback,
    NativeShared,
    NativeExclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioBackendCapabilities {
    pub backend: String,
    pub output_mode: AudioOutputMode,
    pub can_decode_local_files: bool,
    pub can_output_to_device: bool,
    pub supports_exclusive_output: bool,
    pub bounded_pcm_queue: bool,
    pub production_ready: bool,
    pub fallback: String,
}

/// Return only capabilities backed by an implementation in this build.
///
/// Keeping this probe in Rust gives the Electron and Tauri adapters one
/// versioned contract while the actual decoder/device implementation lands in
/// later slices.  The explicit fallback is part of the contract so a missing
/// or stale native binary cannot silently disable playback.
pub fn probe_capabilities() -> AudioBackendCapabilities {
    AudioBackendCapabilities {
        backend: "symphonia-decoder+html-audio".to_string(),
        output_mode: AudioOutputMode::HtmlAudioFallback,
        can_decode_local_files: true,
        can_output_to_device: false,
        supports_exclusive_output: false,
        bounded_pcm_queue: true,
        production_ready: false,
        fallback: "chromium-html-audio".to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PcmFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

impl PcmFormat {
    pub fn frame_samples(self) -> usize {
        usize::from(self.channels)
    }
}

/// A bounded interleaved f32 PCM queue.
///
/// Frames are dropped from the oldest side when a producer outruns the device
/// consumer.  That policy keeps the queue bounded and avoids retaining an
/// entire track during device stalls; callers can inspect `dropped_frames` to
/// decide whether to surface an underrun/recovery event.
#[derive(Debug)]
pub struct PcmRingBuffer {
    format: PcmFormat,
    capacity_frames: usize,
    samples: VecDeque<f32>,
    dropped_frames: u64,
}

impl PcmRingBuffer {
    pub fn new(format: PcmFormat, capacity_frames: usize) -> Self {
        let capacity_frames = capacity_frames.max(1);
        Self {
            format,
            capacity_frames,
            samples: VecDeque::with_capacity(capacity_frames * format.frame_samples()),
            dropped_frames: 0,
        }
    }

    pub fn format(&self) -> PcmFormat {
        self.format
    }

    pub fn capacity_frames(&self) -> usize {
        self.capacity_frames
    }

    pub fn queued_frames(&self) -> usize {
        let channels = self.format.frame_samples();
        if channels == 0 {
            return 0;
        }
        self.samples.len() / channels
    }

    pub fn dropped_frames(&self) -> u64 {
        self.dropped_frames
    }

    pub fn push_interleaved(&mut self, input: &[f32]) -> usize {
        let channels = self.format.frame_samples();
        if channels == 0 {
            return 0;
        }
        let complete_samples = input.len() - input.len() % channels;
        let complete_frames = complete_samples / channels;
        for frame_index in 0..complete_frames {
            if self.queued_frames() >= self.capacity_frames {
                for _ in 0..channels {
                    self.samples.pop_front();
                }
                self.dropped_frames += 1;
            }
            let start = frame_index * channels;
            let end = start + channels;
            self.samples.extend(input[start..end].iter().copied());
        }
        complete_frames
    }

    pub fn pop_interleaved(&mut self, output: &mut [f32]) -> usize {
        let channels = self.format.frame_samples();
        if channels == 0 {
            return 0;
        }
        let requested_samples = output.len() - output.len() % channels;
        let count = requested_samples.min(self.samples.len());
        for slot in &mut output[..count] {
            *slot = self.samples.pop_front().unwrap_or_default();
        }
        count / channels
    }
}

/// A decoded PCM chunk produced by the native decoder.
///
/// The decoder always emits interleaved f32 samples. Keeping this DTO separate
/// from `PcmRingBuffer` lets a device adapter consume chunks without coupling
/// file IO or codec work to its real-time callback.
#[derive(Debug, Clone, PartialEq)]
pub struct DecodedPcmChunk {
    pub format: PcmFormat,
    pub frames: usize,
    pub samples: Vec<f32>,
    pub end_of_stream: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioStreamInfo {
    pub format: PcmFormat,
    pub codec: String,
    pub duration_frames: Option<u64>,
}

#[derive(Debug)]
pub enum AudioDecodeError {
    Io(String),
    Unsupported(String),
    Invalid(String),
    Decoder(String),
}

impl std::fmt::Display for AudioDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(message) => write!(f, "audio IO error: {message}"),
            Self::Unsupported(message) => write!(f, "unsupported audio: {message}"),
            Self::Invalid(message) => write!(f, "invalid audio: {message}"),
            Self::Decoder(message) => write!(f, "audio decoder error: {message}"),
        }
    }
}

impl std::error::Error for AudioDecodeError {}

impl From<std::io::Error> for AudioDecodeError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

/// A pull-based native decoder for local, unencrypted files.
///
/// All codec and container work stays outside the output callback. Callers
/// should pull a chunk on a worker thread and push it into the bounded PCM
/// queue. Remote streams and encrypted provider formats intentionally remain
/// outside this API until their authentication/decryption path is migrated.
pub struct NativeAudioDecoder {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
    info: AudioStreamInfo,
    finished: bool,
}

impl NativeAudioDecoder {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AudioDecodeError> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let mut hint = Hint::new();
        if let Some(extension) = path.extension().and_then(|value| value.to_str()) {
            hint.with_extension(extension);
        }
        let source = MediaSourceStream::new(Box::new(file), Default::default());
        let probed = symphonia::default::get_probe()
            .probe(
                &hint,
                source,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(|error| match error {
                SymphoniaError::Unsupported(message) => {
                    AudioDecodeError::Unsupported(message.to_string())
                }
                SymphoniaError::DecodeError(message) => {
                    AudioDecodeError::Invalid(message.to_string())
                }
                SymphoniaError::IoError(error) => AudioDecodeError::Io(error.to_string()),
                other => AudioDecodeError::Invalid(other.to_string()),
            })?;
        let format = probed;
        let track = format
            .default_track(TrackType::Audio)
            .ok_or_else(|| AudioDecodeError::Unsupported("no audio track".to_string()))?;
        let track_id = track.id;
        let codec_params = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
            .cloned()
            .ok_or_else(|| {
                AudioDecodeError::Unsupported("track has no audio parameters".to_string())
            })?;
        let sample_rate = codec_params
            .sample_rate
            .ok_or_else(|| AudioDecodeError::Unsupported("sample rate is unknown".to_string()))?;
        let channels = codec_params
            .channels
            .as_ref()
            .ok_or_else(|| AudioDecodeError::Unsupported("channel count is unknown".to_string()))?
            .count() as u16;
        if channels == 0 {
            return Err(AudioDecodeError::Invalid(
                "audio track has zero channels".to_string(),
            ));
        }
        let duration_frames = track.num_frames;
        let codec = format!("{:?}", codec_params.codec);
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(&codec_params, &AudioDecoderOptions::default())
            .map_err(|error| AudioDecodeError::Unsupported(error.to_string()))?;
        Ok(Self {
            track_id,
            format,
            decoder,
            info: AudioStreamInfo {
                format: PcmFormat {
                    sample_rate,
                    channels,
                },
                codec,
                duration_frames,
            },
            finished: false,
        })
    }

    pub fn info(&self) -> &AudioStreamInfo {
        &self.info
    }

    /// Decode at most one source packet. The caller controls queueing and
    /// therefore memory use; no full-track buffer is ever allocated here.
    pub fn next_chunk(&mut self) -> Result<Option<DecodedPcmChunk>, AudioDecodeError> {
        if self.finished {
            return Ok(None);
        }
        loop {
            let packet = match self.format.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => {
                    self.finished = true;
                    return Ok(Some(DecodedPcmChunk {
                        format: self.info.format,
                        frames: 0,
                        samples: Vec::new(),
                        end_of_stream: true,
                    }));
                }
                Err(SymphoniaError::ResetRequired) => {
                    self.finished = true;
                    return Ok(Some(DecodedPcmChunk {
                        format: self.info.format,
                        frames: 0,
                        samples: Vec::new(),
                        end_of_stream: true,
                    }));
                }
                Err(SymphoniaError::IoError(error)) => {
                    return Err(AudioDecodeError::Io(error.to_string()))
                }
                Err(error) => return Err(AudioDecodeError::Decoder(error.to_string())),
            };
            if packet.track_id != self.track_id {
                continue;
            }
            match self.decoder.decode(&packet) {
                Ok(audio) => return Ok(Some(Self::chunk_from_audio(self.info.format, audio))),
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(SymphoniaError::IoError(error)) => {
                    return Err(AudioDecodeError::Io(error.to_string()))
                }
                Err(error) => return Err(AudioDecodeError::Decoder(error.to_string())),
            }
        }
    }

    fn chunk_from_audio(format: PcmFormat, audio: GenericAudioBufferRef<'_>) -> DecodedPcmChunk {
        let mut samples = vec![0.0; audio.samples_interleaved()];
        audio.copy_to_slice_interleaved(&mut samples);
        DecodedPcmChunk {
            format,
            frames: samples.len() / usize::from(format.channels),
            samples,
            end_of_stream: false,
        }
    }
}

/// Decode a bounded number of frames for probes and tests without retaining a
/// complete track. A zero limit returns metadata only.
pub fn decode_frames(
    path: impl AsRef<Path>,
    max_frames: usize,
) -> Result<(AudioStreamInfo, Vec<f32>), AudioDecodeError> {
    let mut decoder = NativeAudioDecoder::open(path)?;
    let info = decoder.info().clone();
    if max_frames == 0 {
        return Ok((info, Vec::new()));
    }
    let max_samples = max_frames.saturating_mul(usize::from(info.format.channels));
    let mut samples = Vec::with_capacity(max_samples.min(16_384));
    while samples.len() < max_samples {
        let Some(chunk) = decoder.next_chunk()? else {
            break;
        };
        if chunk.end_of_stream {
            break;
        }
        let remaining = max_samples - samples.len();
        samples.extend(chunk.samples.into_iter().take(remaining));
    }
    Ok((info, samples))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;

    use super::{decode_frames, probe_capabilities, AudioOutputMode, PcmFormat, PcmRingBuffer};

    #[test]
    fn probe_is_explicit_about_html_audio_fallback() {
        let capabilities = probe_capabilities();
        assert_eq!(capabilities.backend, "symphonia-decoder+html-audio");
        assert_eq!(capabilities.output_mode, AudioOutputMode::HtmlAudioFallback);
        assert!(capabilities.can_decode_local_files);
        assert!(!capabilities.can_output_to_device);
        assert!(capabilities.bounded_pcm_queue);
        assert!(!capabilities.production_ready);
    }

    #[test]
    fn pcm_queue_is_bounded_and_drops_oldest_frames() {
        let format = PcmFormat {
            sample_rate: 48_000,
            channels: 2,
        };
        let mut queue = PcmRingBuffer::new(format, 2);
        assert_eq!(queue.push_interleaved(&[1.0, 2.0, 3.0, 4.0]), 2);
        assert_eq!(queue.push_interleaved(&[5.0, 6.0]), 1);
        assert_eq!(queue.queued_frames(), 2);
        assert_eq!(queue.dropped_frames(), 1);
        let mut output = [0.0; 4];
        assert_eq!(queue.pop_interleaved(&mut output), 2);
        assert_eq!(output, [3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn incomplete_interleaved_tail_is_ignored() {
        let format = PcmFormat {
            sample_rate: 44_100,
            channels: 2,
        };
        let mut queue = PcmRingBuffer::new(format, 4);
        assert_eq!(queue.push_interleaved(&[1.0, 2.0, 3.0]), 1);
        assert_eq!(queue.queued_frames(), 1);
    }

    #[test]
    fn zero_channel_format_stays_empty_without_panicking() {
        let format = PcmFormat {
            sample_rate: 48_000,
            channels: 0,
        };
        let mut queue = PcmRingBuffer::new(format, 4);
        assert_eq!(queue.push_interleaved(&[1.0, 2.0]), 0);
        assert_eq!(queue.queued_frames(), 0);
        let mut output = [0.0; 2];
        assert_eq!(queue.pop_interleaved(&mut output), 0);
    }

    #[test]
    fn decodes_small_pcm_wav_without_loading_the_track() {
        let path = std::env::temp_dir().join(format!(
            "aurora-audio-{}-{}.wav",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock before epoch")
                .as_nanos()
        ));
        let samples: [i16; 8] = [0, 8_192, -8_192, 16_384, -16_384, 24_576, -24_576, 32_767];
        let data_len = (samples.len() * std::mem::size_of::<i16>()) as u32;
        let riff_len = 36 + data_len;
        let mut file = fs::File::create(&path).expect("create test wav");
        file.write_all(b"RIFF").unwrap();
        file.write_all(&riff_len.to_le_bytes()).unwrap();
        file.write_all(b"WAVEfmt ").unwrap();
        file.write_all(&16u32.to_le_bytes()).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap();
        file.write_all(&2u16.to_le_bytes()).unwrap();
        file.write_all(&44_100u32.to_le_bytes()).unwrap();
        file.write_all(&176_400u32.to_le_bytes()).unwrap();
        file.write_all(&4u16.to_le_bytes()).unwrap();
        file.write_all(&16u16.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&data_len.to_le_bytes()).unwrap();
        for sample in samples {
            file.write_all(&sample.to_le_bytes()).unwrap();
        }
        file.flush().unwrap();

        let (info, decoded) = decode_frames(&path, 4).expect("decode test wav");
        assert_eq!(
            info.format,
            PcmFormat {
                sample_rate: 44_100,
                channels: 2
            }
        );
        assert_eq!(decoded.len(), 8);
        assert!((decoded[0] - 0.0).abs() < 0.0001);
        assert!((decoded[1] - 0.25).abs() < 0.001);
        assert!((decoded[2] + 0.25).abs() < 0.001);
        assert!(decoded.iter().all(|sample| (-1.0..=1.0).contains(sample)));

        fs::remove_file(path).expect("remove test wav");
    }
}
