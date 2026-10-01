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
use std::sync::{
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    Arc, Condvar, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use symphonia::core::audio::GenericAudioBufferRef;
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

mod qmc2;
pub use qmc2::{decrypt_qmc2_chunk, Qmc2Decryptor};

#[cfg(windows)]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

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
    #[cfg(windows)]
    {
        AudioBackendCapabilities {
            backend: "symphonia-decoder+cpal-wasapi".to_string(),
            output_mode: AudioOutputMode::NativeShared,
            can_decode_local_files: true,
            can_output_to_device: true,
            supports_exclusive_output: false,
            bounded_pcm_queue: true,
            // The output controller is available, but playback integration and
            // format conversion policy still belong to the host adapter.
            production_ready: false,
            fallback: "chromium-html-audio".to_string(),
        }
    }

    #[cfg(not(windows))]
    {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

    pub fn clear(&mut self) {
        self.samples.clear();
    }

    fn pop_sample(&mut self) -> Option<f32> {
        self.samples.pop_front()
    }
}

/// Thread-safe handle shared by a decoder producer and a real-time output
/// callback. The mutex is held only while copying a bounded callback-sized
/// slice; callers should keep decode work outside this object.
#[derive(Clone, Debug)]
pub struct SharedPcmQueue {
    inner: Arc<Mutex<PcmRingBuffer>>,
    consumed_frames: Arc<AtomicU64>,
    gain_bits: Arc<AtomicU32>,
    muted: Arc<AtomicBool>,
}

impl SharedPcmQueue {
    pub fn new(format: PcmFormat, capacity_frames: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PcmRingBuffer::new(format, capacity_frames))),
            consumed_frames: Arc::new(AtomicU64::new(0)),
            gain_bits: Arc::new(AtomicU32::new(1.0f32.to_bits())),
            muted: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn format(&self) -> PcmFormat {
        self.with_queue(|queue| queue.format())
    }

    pub fn capacity_frames(&self) -> usize {
        self.with_queue(|queue| queue.capacity_frames())
    }

    pub fn queued_frames(&self) -> usize {
        self.with_queue(|queue| queue.queued_frames())
    }

    pub fn dropped_frames(&self) -> u64 {
        self.with_queue(|queue| queue.dropped_frames())
    }

    pub fn consumed_frames(&self) -> u64 {
        self.consumed_frames.load(Ordering::Relaxed)
    }

    pub fn set_volume(&self, volume: f32, muted: bool) {
        let volume = if volume.is_finite() {
            volume.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.gain_bits.store(volume.to_bits(), Ordering::Relaxed);
        self.muted.store(muted, Ordering::Relaxed);
    }

    pub fn push_interleaved(&self, input: &[f32]) -> usize {
        self.with_queue_mut(|queue| queue.push_interleaved(input))
    }

    pub fn pop_interleaved(&self, output: &mut [f32]) -> usize {
        let frames = self.with_queue_mut(|queue| queue.pop_interleaved(output));
        self.consumed_frames
            .fetch_add(frames as u64, Ordering::Relaxed);
        frames
    }

    pub fn clear(&self) {
        self.with_queue_mut(PcmRingBuffer::clear);
        self.consumed_frames.store(0, Ordering::Relaxed);
    }

    fn with_queue<T>(&self, callback: impl FnOnce(&PcmRingBuffer) -> T) -> T {
        match self.inner.lock() {
            Ok(queue) => callback(&queue),
            // A panic in an unrelated producer must not permanently disable
            // playback; recover the still-valid bounded buffer instead.
            Err(poisoned) => callback(&poisoned.into_inner()),
        }
    }

    fn with_queue_mut<T>(&self, callback: impl FnOnce(&mut PcmRingBuffer) -> T) -> T {
        match self.inner.lock() {
            Ok(mut queue) => callback(&mut queue),
            Err(poisoned) => callback(&mut poisoned.into_inner()),
        }
    }

    #[cfg(windows)]
    fn fill_output<T>(&self, output: &mut [T])
    where
        T: cpal::Sample + cpal::FromSample<f32>,
    {
        // CPAL documents callback buffers as pre-filled with silence, but
        // explicitly setting it also covers custom hosts and partial queues.
        for sample in output.iter_mut() {
            *sample = T::from_sample(0.0);
        }
        let Ok(mut queue) = self.inner.try_lock() else {
            // Never block the device callback behind a decode burst. The
            // pre-filled silence is preferable to a real-time priority stall.
            return;
        };
        if queue.format().frame_samples() == 0 {
            return;
        }
        let gain = if self.muted.load(Ordering::Relaxed) {
            0.0
        } else {
            f32::from_bits(self.gain_bits.load(Ordering::Relaxed))
        };
        let mut consumed_samples = 0usize;
        for sample in output.iter_mut() {
            if let Some(value) = queue.pop_sample() {
                *sample = T::from_sample((value * gain).clamp(-1.0, 1.0));
                consumed_samples += 1;
            }
        }
        let channels = queue.format().frame_samples();
        if channels != 0 {
            self.consumed_frames
                .fetch_add((consumed_samples / channels) as u64, Ordering::Relaxed);
        }
    }
}

#[derive(Debug)]
pub enum NativeOutputError {
    UnsupportedPlatform,
    NoOutputDevice,
    Device(String),
    Stream(String),
    UnsupportedSampleFormat(String),
}

impl std::fmt::Display for NativeOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                write!(f, "native audio output is unavailable on this platform")
            }
            Self::NoOutputDevice => write!(f, "no default output device is available"),
            Self::Device(message) => write!(f, "audio device error: {message}"),
            Self::Stream(message) => write!(f, "audio output stream error: {message}"),
            Self::UnsupportedSampleFormat(format) => {
                write!(f, "unsupported output sample format: {format}")
            }
        }
    }
}

impl std::error::Error for NativeOutputError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeOutputSnapshot {
    pub format: PcmFormat,
    pub device_name: Option<String>,
    pub running: bool,
    pub queued_frames: usize,
    pub dropped_frames: u64,
}

/// A Windows WASAPI shared-mode output stream backed by a bounded PCM queue.
///
/// `open` selects the system default output device and its default shared-mode
/// format. The caller should decode/resample into [`format`](Self::format),
/// enqueue with [`push_interleaved`](Self::push_interleaved), then call
/// [`start`](Self::start). The non-Windows implementation is an explicit
/// unsupported-platform result so host adapters can retain their fallback.
pub struct NativeOutputController {
    queue: SharedPcmQueue,
    format: PcmFormat,
    device_name: Option<String>,
    running: bool,
    #[cfg(windows)]
    stream: cpal::Stream,
}

impl std::fmt::Debug for NativeOutputController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeOutputController")
            .field("format", &self.format)
            .field("device_name", &self.device_name)
            .field("running", &self.running)
            .finish_non_exhaustive()
    }
}

impl NativeOutputController {
    pub fn open(capacity_frames: usize) -> Result<Self, NativeOutputError> {
        #[cfg(windows)]
        {
            let host = cpal::default_host();
            let device = host
                .default_output_device()
                .ok_or(NativeOutputError::NoOutputDevice)?;
            let device_name = Some(device.to_string());
            let supported = device
                .default_output_config()
                .map_err(|error| NativeOutputError::Device(error.to_string()))?;
            let format = PcmFormat {
                sample_rate: supported.sample_rate(),
                channels: supported.channels(),
            };
            if format.channels == 0 || format.sample_rate == 0 {
                return Err(NativeOutputError::Device(
                    "default output config has an invalid format".to_string(),
                ));
            }
            let queue = SharedPcmQueue::new(format, capacity_frames);
            let sample_format = supported.sample_format();
            let stream_config: cpal::StreamConfig = supported.into();
            let stream = match sample_format {
                cpal::SampleFormat::F32 => {
                    build_cpal_stream::<f32>(&device, &stream_config, queue.clone())?
                }
                cpal::SampleFormat::I16 => {
                    build_cpal_stream::<i16>(&device, &stream_config, queue.clone())?
                }
                cpal::SampleFormat::U16 => {
                    build_cpal_stream::<u16>(&device, &stream_config, queue.clone())?
                }
                other => {
                    return Err(NativeOutputError::UnsupportedSampleFormat(
                        other.to_string(),
                    ))
                }
            };
            return Ok(Self {
                queue,
                format,
                device_name,
                running: false,
                stream,
            });
        }

        #[cfg(not(windows))]
        {
            let _ = capacity_frames;
            Err(NativeOutputError::UnsupportedPlatform)
        }
    }

    pub fn format(&self) -> PcmFormat {
        self.format
    }

    pub fn device_name(&self) -> Option<&str> {
        self.device_name.as_deref()
    }

    pub fn queue(&self) -> SharedPcmQueue {
        self.queue.clone()
    }

    pub fn push_interleaved(&self, input: &[f32]) -> usize {
        self.queue.push_interleaved(input)
    }

    pub fn queued_frames(&self) -> usize {
        self.queue.queued_frames()
    }

    pub fn dropped_frames(&self) -> u64 {
        self.queue.dropped_frames()
    }

    pub fn set_volume(&self, volume: f32, muted: bool) {
        self.queue.set_volume(volume, muted);
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn snapshot(&self) -> NativeOutputSnapshot {
        NativeOutputSnapshot {
            format: self.format,
            device_name: self.device_name.clone(),
            running: self.running,
            queued_frames: self.queued_frames(),
            dropped_frames: self.dropped_frames(),
        }
    }

    pub fn start(&mut self) -> Result<(), NativeOutputError> {
        #[cfg(windows)]
        {
            self.stream
                .play()
                .map_err(|error| NativeOutputError::Stream(error.to_string()))?;
            self.running = true;
            return Ok(());
        }

        #[cfg(not(windows))]
        {
            Err(NativeOutputError::UnsupportedPlatform)
        }
    }

    pub fn pause(&mut self) -> Result<(), NativeOutputError> {
        #[cfg(windows)]
        {
            self.stream
                .pause()
                .map_err(|error| NativeOutputError::Stream(error.to_string()))?;
            self.running = false;
            return Ok(());
        }

        #[cfg(not(windows))]
        {
            Err(NativeOutputError::UnsupportedPlatform)
        }
    }

    pub fn stop(&mut self) -> Result<(), NativeOutputError> {
        let result = self.pause();
        self.queue.clear();
        result
    }
}

#[cfg(windows)]
fn build_cpal_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    queue: SharedPcmQueue,
) -> Result<cpal::Stream, NativeOutputError>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let error_callback = |error| {
        // CPAL invokes this on its own error path. The controller exposes
        // stream errors through the normal callback lifecycle; avoid locking
        // or allocating here so recovery remains host-owned.
        eprintln!("native audio output stream error: {error}");
    };
    device
        .build_output_stream(
            config.clone(),
            move |data: &mut [T], _| queue.fill_output(data),
            error_callback,
            None,
        )
        .map_err(|error| NativeOutputError::Stream(error.to_string()))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativePlaybackStatus {
    Idle,
    Paused,
    Playing,
    Ended,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativePlaybackSnapshot {
    pub status: NativePlaybackStatus,
    pub position_ms: u64,
    pub duration_ms: Option<u64>,
    pub format: PcmFormat,
    pub device_name: Option<String>,
    pub queued_frames: usize,
    pub dropped_frames: u64,
}

#[derive(Debug)]
pub enum NativePlaybackError {
    Decode(AudioDecodeError),
    Output(NativeOutputError),
    FormatMismatch {
        source: PcmFormat,
        output: PcmFormat,
    },
    UnsupportedSeek,
    NotLoaded,
    Ended,
    Worker(String),
}

impl std::fmt::Display for NativePlaybackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Decode(error) => write!(f, "{error}"),
            Self::Output(error) => write!(f, "{error}"),
            Self::FormatMismatch { source, output } => write!(
                f,
                "audio format mismatch: source {} Hz/{} ch, output {} Hz/{} ch",
                source.sample_rate, source.channels, output.sample_rate, output.channels
            ),
            Self::UnsupportedSeek => write!(f, "native audio seek is unavailable"),
            Self::NotLoaded => write!(f, "native audio file is not loaded"),
            Self::Ended => write!(f, "native audio track has ended"),
            Self::Worker(message) => write!(f, "native audio worker error: {message}"),
        }
    }
}

impl std::error::Error for NativePlaybackError {}

impl From<AudioDecodeError> for NativePlaybackError {
    fn from(error: AudioDecodeError) -> Self {
        Self::Decode(error)
    }
}

impl From<NativeOutputError> for NativePlaybackError {
    fn from(error: NativeOutputError) -> Self {
        Self::Output(error)
    }
}

/// Convert decoder PCM to the system shared-mode format with bounded linear
/// interpolation and deterministic channel mapping. Decoder packets are
/// already bounded, so the temporary buffer remains proportional to one
/// packet rather than the full track.
fn resample_interleaved(
    input: &[f32],
    input_frames: usize,
    input_format: PcmFormat,
    output_format: PcmFormat,
) -> Vec<f32> {
    let input_channels = usize::from(input_format.channels);
    let output_channels = usize::from(output_format.channels);
    if input_frames == 0 || input_channels == 0 || output_channels == 0 {
        return Vec::new();
    }
    if input_format == output_format {
        return input.to_vec();
    }
    let output_frames = if input_format.sample_rate == output_format.sample_rate {
        input_frames
    } else {
        ((input_frames as u64)
            .saturating_mul(u64::from(output_format.sample_rate))
            .saturating_add(u64::from(input_format.sample_rate).saturating_sub(1))
            / u64::from(input_format.sample_rate)) as usize
    };
    let mut output = vec![0.0; output_frames.saturating_mul(output_channels)];
    let rate_ratio = f64::from(input_format.sample_rate) / f64::from(output_format.sample_rate);
    for output_frame in 0..output_frames {
        let source_position = (output_frame as f64) * rate_ratio;
        let source_index = (source_position.floor() as usize).min(input_frames - 1);
        let next_index = (source_index + 1).min(input_frames - 1);
        let fraction = (source_position - source_index as f64) as f32;
        for output_channel in 0..output_channels {
            let sample = |frame: usize, channel: usize| {
                let source_channel = if input_channels == 1 {
                    0
                } else if output_channels == 1 {
                    channel.min(input_channels - 1)
                } else {
                    channel.min(input_channels - 1)
                };
                input[frame * input_channels + source_channel]
            };
            let value = if output_channels == 1 && input_channels > 1 {
                let mut current = 0.0;
                let mut next = 0.0;
                for channel in 0..input_channels {
                    current += input[source_index * input_channels + channel];
                    next += input[next_index * input_channels + channel];
                }
                ((current / input_channels as f32) * (1.0 - fraction))
                    + ((next / input_channels as f32) * fraction)
            } else {
                sample(source_index, output_channel) * (1.0 - fraction)
                    + sample(next_index, output_channel) * fraction
            };
            output[output_frame * output_channels + output_channel] = value;
        }
    }
    output
}

#[derive(Debug)]
struct NativeWorkerState {
    status: NativePlaybackStatus,
    position_frames: u64,
    skip_frames: u64,
}

/// A complete local-file playback session for native adapters.
///
/// Decoder work runs on a normal worker thread and only ever feeds the bounded
/// queue. The device callback never waits for this worker; pause and stop wake
/// the worker and join it before a session is replaced.
pub struct NativePlaybackEngine {
    path: std::path::PathBuf,
    output: NativeOutputController,
    source_format: PcmFormat,
    duration_frames: Option<u64>,
    state: Arc<(Mutex<NativeWorkerState>, Condvar)>,
    stop: Arc<AtomicBool>,
    decoder: Option<NativeAudioDecoder>,
    worker: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for NativePlaybackEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativePlaybackEngine")
            .field("path", &self.path)
            .field("source_format", &self.source_format)
            .field("duration_frames", &self.duration_frames)
            .field("snapshot", &self.snapshot())
            .finish()
    }
}

impl NativePlaybackEngine {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, NativePlaybackError> {
        let path = path.as_ref().to_path_buf();
        let decoder = NativeAudioDecoder::open(&path)?;
        let source_format = decoder.info().format;
        let duration_frames = decoder.info().duration_frames;
        let capacity_frames = source_format.sample_rate.saturating_mul(2) as usize;
        let output = NativeOutputController::open(capacity_frames.max(1))?;
        Ok(Self {
            path,
            output,
            source_format,
            duration_frames,
            state: Arc::new((
                Mutex::new(NativeWorkerState {
                    status: NativePlaybackStatus::Paused,
                    position_frames: 0,
                    skip_frames: 0,
                }),
                Condvar::new(),
            )),
            stop: Arc::new(AtomicBool::new(false)),
            decoder: Some(decoder),
            worker: None,
        })
    }

    pub fn snapshot(&self) -> NativePlaybackSnapshot {
        let (lock, _) = &*self.state;
        let state = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let output_format = self.output.format();
        let consumed_source_frames = (self.output.queue().consumed_frames() as u128)
            .saturating_mul(u128::from(self.source_format.sample_rate))
            .saturating_div(u128::from(output_format.sample_rate.max(1)))
            as u64;
        let position_frames = state.position_frames.saturating_add(consumed_source_frames);
        NativePlaybackSnapshot {
            status: state.status,
            position_ms: position_frames
                .saturating_mul(1_000)
                .saturating_div(u64::from(self.source_format.sample_rate)),
            duration_ms: self.duration_frames.map(|frames| {
                frames
                    .saturating_mul(1_000)
                    .saturating_div(u64::from(self.source_format.sample_rate))
            }),
            format: self.source_format,
            device_name: self.output.device_name().map(str::to_string),
            queued_frames: self.output.queued_frames(),
            dropped_frames: self.output.dropped_frames(),
        }
    }

    pub fn play(&mut self) -> Result<(), NativePlaybackError> {
        {
            let (lock, _) = &*self.state;
            let state = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            if state.status == NativePlaybackStatus::Ended {
                return Err(NativePlaybackError::Ended);
            }
        }
        self.output.start()?;
        let (lock, wake) = &*self.state;
        {
            let mut state = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            state.status = NativePlaybackStatus::Playing;
        }
        wake.notify_all();
        if self.worker.is_none() {
            let Some(decoder) = self.decoder.take() else {
                return Err(NativePlaybackError::NotLoaded);
            };
            self.spawn_worker(decoder);
        }
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), NativePlaybackError> {
        self.output.pause()?;
        let (lock, wake) = &*self.state;
        let mut state = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.status == NativePlaybackStatus::Playing {
            state.status = NativePlaybackStatus::Paused;
        }
        wake.notify_all();
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), NativePlaybackError> {
        self.stop_worker();
        self.output.stop()?;
        let (lock, _) = &*self.state;
        let mut state = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        state.status = NativePlaybackStatus::Idle;
        state.position_frames = 0;
        state.skip_frames = 0;
        self.decoder = Some(NativeAudioDecoder::open(&self.path)?);
        Ok(())
    }

    pub fn seek(&mut self, position_ms: u64) -> Result<(), NativePlaybackError> {
        let was_playing = {
            let (lock, _) = &*self.state;
            lock.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .status
                == NativePlaybackStatus::Playing
        };
        self.stop_worker();
        self.output.pause()?;
        self.output.queue().clear();
        let decoder = NativeAudioDecoder::open(&self.path)?;
        let target = position_ms
            .saturating_mul(u64::from(self.source_format.sample_rate))
            .saturating_div(1_000);
        let (lock, _) = &*self.state;
        let mut state = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        state.position_frames = self
            .duration_frames
            .map_or(target, |duration| target.min(duration));
        state.skip_frames = state.position_frames;
        state.status = NativePlaybackStatus::Paused;
        drop(state);
        self.decoder = Some(decoder);
        if was_playing {
            self.play()?;
        }
        Ok(())
    }

    pub fn set_volume(&self, volume: f32, muted: bool) {
        self.output.set_volume(volume, muted);
    }

    fn spawn_worker(&mut self, mut decoder: NativeAudioDecoder) {
        let state = Arc::clone(&self.state);
        let stop = Arc::clone(&self.stop);
        let queue = self.output.queue();
        let format = self.source_format;
        let output_format = self.output.format();
        self.worker = Some(thread::spawn(move || loop {
            if stop.load(Ordering::Acquire) {
                break;
            }
            let (lock, wake) = &*state;
            let mut worker_state = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            while worker_state.status != NativePlaybackStatus::Playing
                && !stop.load(Ordering::Acquire)
            {
                worker_state = wake
                    .wait(worker_state)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
            }
            if stop.load(Ordering::Acquire) {
                break;
            }
            drop(worker_state);
            if queue.queued_frames() >= queue.capacity_frames().saturating_mul(3) / 4 {
                thread::sleep(Duration::from_millis(5));
                continue;
            }
            match decoder.next_chunk() {
                Ok(Some(chunk)) if chunk.end_of_stream => {
                    // The decoder can reach EOF while the device still has a
                    // bounded tail queued. Wait for that tail to drain before
                    // exposing `Ended`, otherwise the renderer advances early
                    // and truncates the last packet.
                    while queue.queued_frames() > 0 && !stop.load(Ordering::Acquire) {
                        thread::sleep(Duration::from_millis(10));
                    }
                    if stop.load(Ordering::Acquire) {
                        break;
                    }
                    let mut state = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    state.status = NativePlaybackStatus::Ended;
                    wake.notify_all();
                    break;
                }
                Ok(Some(chunk)) => {
                    if chunk.format != format {
                        let mut state =
                            lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                        state.status = NativePlaybackStatus::Ended;
                        wake.notify_all();
                        break;
                    }
                    let mut state = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    let skip = state.skip_frames.min(chunk.frames as u64) as usize;
                    state.skip_frames -= skip as u64;
                    let start = skip.saturating_mul(usize::from(format.channels));
                    let converted = resample_interleaved(
                        &chunk.samples[start..],
                        chunk.frames.saturating_sub(skip),
                        format,
                        output_format,
                    );
                    queue.push_interleaved(&converted);
                }
                Ok(None) => {
                    let mut state = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    state.status = NativePlaybackStatus::Ended;
                    wake.notify_all();
                    break;
                }
                Err(_) => {
                    let mut state = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    state.status = NativePlaybackStatus::Ended;
                    wake.notify_all();
                    break;
                }
            }
        }));
    }

    fn stop_worker(&mut self) {
        self.stop.store(true, Ordering::Release);
        let (_, wake) = &*self.state;
        wake.notify_all();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.stop.store(false, Ordering::Release);
    }
}

impl Drop for NativePlaybackEngine {
    fn drop(&mut self) {
        self.stop_worker();
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

    #[cfg(not(windows))]
    use super::NativeOutputController;
    use super::{
        decode_frames, probe_capabilities, resample_interleaved, AudioOutputMode, PcmFormat,
        PcmRingBuffer, SharedPcmQueue,
    };

    #[cfg(not(windows))]
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

    #[cfg(windows)]
    #[test]
    fn probe_exposes_wasapi_shared_output_boundary() {
        let capabilities = probe_capabilities();
        assert_eq!(capabilities.backend, "symphonia-decoder+cpal-wasapi");
        assert_eq!(capabilities.output_mode, AudioOutputMode::NativeShared);
        assert!(capabilities.can_decode_local_files);
        assert!(capabilities.can_output_to_device);
        assert!(!capabilities.supports_exclusive_output);
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
    fn shared_queue_preserves_bounded_transport_across_handles() {
        let format = PcmFormat {
            sample_rate: 48_000,
            channels: 2,
        };
        let producer = SharedPcmQueue::new(format, 2);
        let consumer = producer.clone();
        assert_eq!(producer.push_interleaved(&[0.1, 0.2, 0.3, 0.4]), 2);
        assert_eq!(consumer.queued_frames(), 2);
        let mut output = [0.0; 2];
        assert_eq!(consumer.pop_interleaved(&mut output), 1);
        assert_eq!(output, [0.1, 0.2]);
        assert_eq!(producer.dropped_frames(), 0);
        assert_eq!(producer.capacity_frames(), 2);
        assert_eq!(producer.consumed_frames(), 1);
    }

    #[test]
    fn resamples_rate_and_maps_channels_for_shared_output() {
        let input = [0.0, 1.0, 1.0, 0.0];
        let converted = resample_interleaved(
            &input,
            2,
            PcmFormat {
                sample_rate: 44_100,
                channels: 2,
            },
            PcmFormat {
                sample_rate: 48_000,
                channels: 1,
            },
        );
        assert_eq!(converted.len(), 3);
        assert!((converted[0] - 0.5).abs() < 0.0001);
        assert!((converted[1] - 0.5).abs() < 0.0001);
        assert!((converted[2] - 0.5).abs() < 0.0001);
    }

    #[cfg(not(windows))]
    #[test]
    fn native_output_is_explicitly_unavailable_outside_windows() {
        assert!(matches!(
            NativeOutputController::open(48_000),
            Err(super::NativeOutputError::UnsupportedPlatform)
        ));
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
