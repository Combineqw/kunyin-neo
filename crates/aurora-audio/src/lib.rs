//! Host-neutral boundary for the native audio migration.
//!
//! The current desktop product still renders through Chromium's
//! `HTMLAudioElement -> Web Audio -> AudioContext.destination` path.  This
//! crate deliberately reports that fact instead of pretending that a native
//! decoder or WASAPI endpoint already exists.  The bounded PCM queue is the
//! transport contract that a future decoder and output thread can share
//! without allowing a slow device to grow memory without limit.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

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
        backend: "html-audio".to_string(),
        output_mode: AudioOutputMode::HtmlAudioFallback,
        can_decode_local_files: false,
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

#[cfg(test)]
mod tests {
    use super::{probe_capabilities, AudioOutputMode, PcmFormat, PcmRingBuffer};

    #[test]
    fn probe_is_explicit_about_html_audio_fallback() {
        let capabilities = probe_capabilities();
        assert_eq!(capabilities.backend, "html-audio");
        assert_eq!(capabilities.output_mode, AudioOutputMode::HtmlAudioFallback);
        assert!(!capabilities.can_decode_local_files);
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
}
