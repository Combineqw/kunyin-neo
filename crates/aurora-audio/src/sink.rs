//! Native Audio Sink & Stream Pump: Symphonia Decoder -> Resampler -> Native DSP -> WASAPI/PCM Queue.
//!
//! Connects native stream caching, format decoding, multi-band DSP audio effects,
//! and the Windows audio endpoint into an end-to-end native playback pipeline.

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Condvar, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{
    dsp::{DspPipelineConfig, DspPipelineSnapshot, NativeDspPipeline},
    resample_interleaved, NativeOutputController, PcmFormat, SharedPcmQueue, StreamCache,
    StreamDecodeOutput, StreamDecoder,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeSinkStatus {
    Idle,
    Buffering,
    Playing,
    Paused,
    Ended,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSinkSnapshot {
    pub sink_id: String,
    pub status: NativeSinkStatus,
    pub position_ms: u64,
    pub duration_ms: Option<u64>,
    pub source_format: Option<PcmFormat>,
    pub output_format: PcmFormat,
    pub queued_frames: usize,
    pub dropped_frames: u64,
    pub device_name: Option<String>,
    pub dsp: DspPipelineSnapshot,
    pub underflow_count: u64,
    pub is_virtual: bool,
    pub is_exclusive: bool,
}

pub enum AudioOutputEndpoint {
    Hardware(NativeOutputController),
    Virtual {
        queue: SharedPcmQueue,
        format: PcmFormat,
        running: bool,
        device_name: String,
        is_exclusive: bool,
    },
}

impl AudioOutputEndpoint {
    pub fn hardware_or_virtual(
        format: PcmFormat,
        capacity_frames: usize,
        force_virtual: bool,
    ) -> Self {
        if !force_virtual {
            if let Ok(controller) = NativeOutputController::open(capacity_frames) {
                return Self::Hardware(controller);
            }
        }
        let queue = SharedPcmQueue::new(format, capacity_frames);
        Self::Virtual {
            queue,
            format,
            running: false,
            device_name: "virtual-aurora-sink".to_string(),
            is_exclusive: false,
        }
    }

    pub fn format(&self) -> PcmFormat {
        match self {
            Self::Hardware(c) => c.format(),
            Self::Virtual { format, .. } => *format,
        }
    }

    pub fn device_name(&self) -> Option<String> {
        match self {
            Self::Hardware(c) => c.device_name().map(str::to_string),
            Self::Virtual { device_name, .. } => Some(device_name.clone()),
        }
    }

    pub fn is_virtual(&self) -> bool {
        matches!(self, Self::Virtual { .. })
    }

    pub fn is_exclusive(&self) -> bool {
        match self {
            Self::Hardware(c) => c.is_exclusive(),
            Self::Virtual { is_exclusive, .. } => *is_exclusive,
        }
    }

    pub fn switch_device(
        &mut self,
        target_device: Option<&str>,
        exclusive: bool,
    ) -> Result<(), String> {
        match self {
            Self::Hardware(c) => c
                .switch_device(target_device, exclusive)
                .map_err(|e| e.to_string()),
            Self::Virtual {
                device_name,
                is_exclusive,
                ..
            } => {
                *device_name = target_device
                    .unwrap_or("virtual-aurora-sink")
                    .to_string();
                *is_exclusive = exclusive;
                Ok(())
            }
        }
    }

    pub fn start(&mut self) -> Result<(), String> {
        match self {
            Self::Hardware(c) => c.start().map_err(|e| e.to_string()),
            Self::Virtual { running, .. } => {
                *running = true;
                Ok(())
            }
        }
    }

    pub fn pause(&mut self) -> Result<(), String> {
        match self {
            Self::Hardware(c) => c.pause().map_err(|e| e.to_string()),
            Self::Virtual { running, .. } => {
                *running = false;
                Ok(())
            }
        }
    }

    pub fn stop(&mut self) -> Result<(), String> {
        match self {
            Self::Hardware(c) => c.stop().map_err(|e| e.to_string()),
            Self::Virtual { running, queue, .. } => {
                *running = false;
                queue.clear();
                Ok(())
            }
        }
    }

    pub fn push_interleaved(&self, samples: &[f32]) -> usize {
        match self {
            Self::Hardware(c) => c.push_interleaved(samples),
            Self::Virtual { queue, .. } => queue.push_interleaved(samples),
        }
    }

    pub fn queued_frames(&self) -> usize {
        match self {
            Self::Hardware(c) => c.queued_frames(),
            Self::Virtual { queue, .. } => queue.queued_frames(),
        }
    }

    pub fn capacity_frames(&self) -> usize {
        match self {
            Self::Hardware(c) => c.queue().capacity_frames(),
            Self::Virtual { queue, .. } => queue.capacity_frames(),
        }
    }

    pub fn dropped_frames(&self) -> u64 {
        match self {
            Self::Hardware(c) => c.dropped_frames(),
            Self::Virtual { queue, .. } => queue.dropped_frames(),
        }
    }

    pub fn consumed_frames(&self) -> u64 {
        match self {
            Self::Hardware(c) => c.queue().consumed_frames(),
            Self::Virtual { queue, .. } => queue.consumed_frames(),
        }
    }

    pub fn clear_queue(&self) {
        match self {
            Self::Hardware(c) => c.queue().clear(),
            Self::Virtual { queue, .. } => queue.clear(),
        }
    }

    pub fn set_volume(&self, volume: f32, muted: bool) {
        match self {
            Self::Hardware(c) => c.set_volume(volume, muted),
            Self::Virtual { queue, .. } => queue.set_volume(volume, muted),
        }
    }
}

#[derive(Debug)]
struct SinkInternalState {
    status: NativeSinkStatus,
    position_frames: u64,
    source_format: Option<PcmFormat>,
    duration_frames: Option<u64>,
}

/// End-to-end native streaming audio sink:
/// StreamCache -> StreamDecoder -> Resampler -> NativeDspPipeline -> Output Endpoint.
pub struct NativeAudioSink {
    sink_id: String,
    cache: Arc<Mutex<StreamCache>>,
    decoder: Arc<Mutex<StreamDecoder>>,
    dsp: Arc<Mutex<NativeDspPipeline>>,
    endpoint: Arc<Mutex<AudioOutputEndpoint>>,
    state: Arc<(Mutex<SinkInternalState>, Condvar)>,
    stop: Arc<AtomicBool>,
    underflow_count: Arc<AtomicU64>,
    worker: Option<JoinHandle<()>>,
}

impl NativeAudioSink {
    pub fn new(
        sink_id: impl Into<String>,
        cache: Arc<Mutex<StreamCache>>,
        container_hint: Option<String>,
        mime_hint: Option<String>,
        dsp_config: Option<DspPipelineConfig>,
        force_virtual: bool,
    ) -> Self {
        let sink_id = sink_id.into();
        let decoder = StreamDecoder::new(&sink_id, Arc::clone(&cache), container_hint, mime_hint);

        // Standard default output format: 48kHz stereo, 2 seconds buffer
        let default_output_format = PcmFormat {
            sample_rate: 48000,
            channels: 2,
        };
        let capacity_frames = default_output_format.sample_rate as usize * 2;
        let endpoint = AudioOutputEndpoint::hardware_or_virtual(
            default_output_format,
            capacity_frames,
            force_virtual,
        );

        let output_format = endpoint.format();
        let dsp_config = dsp_config.unwrap_or_default();
        let dsp = NativeDspPipeline::new(output_format, dsp_config);

        let state = Arc::new((
            Mutex::new(SinkInternalState {
                status: NativeSinkStatus::Idle,
                position_frames: 0,
                source_format: None,
                duration_frames: None,
            }),
            Condvar::new(),
        ));
        let stop = Arc::new(AtomicBool::new(false));
        let underflow_count = Arc::new(AtomicU64::new(0));

        let decoder_arc = Arc::new(Mutex::new(decoder));
        let dsp_arc = Arc::new(Mutex::new(dsp));
        let endpoint_arc = Arc::new(Mutex::new(endpoint));

        let mut sink = Self {
            sink_id,
            cache,
            decoder: decoder_arc,
            dsp: dsp_arc,
            endpoint: endpoint_arc,
            state,
            stop,
            underflow_count,
            worker: None,
        };

        sink.spawn_worker();
        sink
    }

    pub fn sink_id(&self) -> &str {
        &self.sink_id
    }

    pub fn cache(&self) -> Arc<Mutex<StreamCache>> {
        Arc::clone(&self.cache)
    }

    pub fn play(&self) -> Result<(), String> {
        {
            let mut endpoint = self
                .endpoint
                .lock()
                .map_err(|_| "endpoint lock poisoned".to_string())?;
            endpoint.start()?;
        }
        let (lock, wake) = &*self.state;
        {
            let mut state = lock.lock().map_err(|_| "state lock poisoned".to_string())?;
            if state.status == NativeSinkStatus::Ended {
                return Err("playback has ended, seek to replay".to_string());
            }
            state.status = NativeSinkStatus::Playing;
        }
        wake.notify_all();
        Ok(())
    }

    pub fn pause(&self) -> Result<(), String> {
        {
            let mut endpoint = self
                .endpoint
                .lock()
                .map_err(|_| "endpoint lock poisoned".to_string())?;
            endpoint.pause()?;
        }
        let (lock, wake) = &*self.state;
        {
            let mut state = lock.lock().map_err(|_| "state lock poisoned".to_string())?;
            if state.status == NativeSinkStatus::Playing || state.status == NativeSinkStatus::Buffering {
                state.status = NativeSinkStatus::Paused;
            }
        }
        wake.notify_all();
        Ok(())
    }

    pub fn stop(&self) -> Result<(), String> {
        {
            let mut endpoint = self
                .endpoint
                .lock()
                .map_err(|_| "endpoint lock poisoned".to_string())?;
            endpoint.stop()?;
        }
        {
            let mut dsp = self.dsp.lock().map_err(|_| "dsp lock poisoned".to_string())?;
            dsp.reset();
        }
        let (lock, wake) = &*self.state;
        {
            let mut state = lock.lock().map_err(|_| "state lock poisoned".to_string())?;
            state.status = NativeSinkStatus::Idle;
            state.position_frames = 0;
        }
        wake.notify_all();
        Ok(())
    }

    pub fn seek_ms(&self, target_ms: u64) -> Result<(), String> {
        // Pause output queue & flush
        {
            let endpoint = self
                .endpoint
                .lock()
                .map_err(|_| "endpoint lock poisoned".to_string())?;
            endpoint.clear_queue();
        }
        // Seek stream decoder
        {
            let mut decoder = self
                .decoder
                .lock()
                .map_err(|_| "decoder lock poisoned".to_string())?;
            decoder
                .seek_ms(target_ms)
                .map_err(|err| format!("decoder seek error: {err}"))?;
        }
        // Reset DSP delays
        {
            let mut dsp = self.dsp.lock().map_err(|_| "dsp lock poisoned".to_string())?;
            dsp.reset();
        }
        // Update position
        let (lock, wake) = &*self.state;
        {
            let mut state = lock.lock().map_err(|_| "state lock poisoned".to_string())?;
            let sample_rate = state
                .source_format
                .map(|f| f.sample_rate)
                .unwrap_or(48000) as u64;
            state.position_frames = (target_ms * sample_rate) / 1000;
        }
        wake.notify_all();
        Ok(())
    }

    pub fn set_volume(&self, volume: f32, muted: bool) {
        if let Ok(mut dsp) = self.dsp.lock() {
            let mut cfg = dsp.snapshot().config;
            cfg.volume = volume;
            cfg.muted = muted;
            dsp.configure(cfg);
        }
        if let Ok(endpoint) = self.endpoint.lock() {
            endpoint.set_volume(volume, muted);
        }
    }

    pub fn configure_dsp(&self, config: DspPipelineConfig) {
        if let Ok(mut dsp) = self.dsp.lock() {
            dsp.configure(config);
        }
    }

    pub fn switch_device(&self, target_device: Option<&str>, exclusive: bool) -> Result<(), String> {
        let mut endpoint = self
            .endpoint
            .lock()
            .map_err(|_| "endpoint lock poisoned".to_string())?;
        endpoint.switch_device(target_device, exclusive)?;
        let output_format = endpoint.format();
        if let Ok(mut dsp) = self.dsp.lock() {
            dsp.set_format(output_format);
        }
        Ok(())
    }

    pub fn snapshot(&self) -> NativeSinkSnapshot {
        let (lock, _) = &*self.state;
        let state = lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let endpoint = self
            .endpoint
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let output_format = endpoint.format();
        let queued_frames = endpoint.queued_frames();
        let dropped_frames = endpoint.dropped_frames();
        let device_name = endpoint.device_name();
        let is_virtual = endpoint.is_virtual();
        let is_exclusive = endpoint.is_exclusive();

        let source_rate = state
            .source_format
            .map(|f| f.sample_rate)
            .unwrap_or(output_format.sample_rate);
        let consumed_source = (endpoint.consumed_frames() as u128)
            .saturating_mul(u128::from(source_rate))
            .saturating_div(u128::from(output_format.sample_rate.max(1))) as u64;
        let total_frames = state.position_frames.saturating_add(consumed_source);
        let position_ms = if source_rate > 0 {
            (total_frames * 1000) / (source_rate as u64)
        } else {
            0
        };

        let duration_ms = state.duration_frames.and_then(|df| {
            if source_rate > 0 {
                Some((df * 1000) / (source_rate as u64))
            } else {
                None
            }
        });

        let dsp_snap = self
            .dsp
            .lock()
            .map(|d| d.snapshot())
            .unwrap_or_else(|_| DspPipelineSnapshot {
                format: output_format,
                config: DspPipelineConfig::default(),
                eq_active: false,
                current_gain_linear: 1.0,
                replay_gain_linear: 1.0,
                peak_level: 0.0,
            });

        NativeSinkSnapshot {
            sink_id: self.sink_id.clone(),
            status: state.status,
            position_ms,
            duration_ms,
            source_format: state.source_format,
            output_format,
            queued_frames,
            dropped_frames,
            device_name,
            dsp: dsp_snap,
            underflow_count: self.underflow_count.load(Ordering::Relaxed),
            is_virtual,
            is_exclusive,
        }
    }

    fn spawn_worker(&mut self) {
        let state = Arc::clone(&self.state);
        let stop = Arc::clone(&self.stop);
        let decoder_arc = Arc::clone(&self.decoder);
        let dsp_arc = Arc::clone(&self.dsp);
        let endpoint_arc = Arc::clone(&self.endpoint);
        let underflow_count = Arc::clone(&self.underflow_count);

        self.worker = Some(thread::spawn(move || {
            let (lock, wake) = &*state;

            while !stop.load(Ordering::Acquire) {
                // Check play status
                {
                    let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
                    while guard.status != NativeSinkStatus::Playing
                        && guard.status != NativeSinkStatus::Buffering
                        && !stop.load(Ordering::Acquire)
                    {
                        guard = wake.wait(guard).unwrap_or_else(|p| p.into_inner());
                    }
                }
                if stop.load(Ordering::Acquire) {
                    break;
                }

                // Check output queue capacity to prevent overrun / CPU spin
                let (queued, capacity, output_format) = {
                    let ep = endpoint_arc.lock().unwrap_or_else(|p| p.into_inner());
                    (ep.queued_frames(), ep.capacity_frames(), ep.format())
                };

                if queued >= (capacity * 3) / 4 {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                }

                // Step decode
                let decode_step = {
                    let mut dec = decoder_arc.lock().unwrap_or_else(|p| p.into_inner());
                    dec.decode_next_chunk()
                };

                match decode_step {
                    Ok(StreamDecodeOutput::Chunk(chunk)) => {
                        // Successfully received decoded chunk
                        {
                            let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
                            if guard.source_format.is_none() {
                                guard.source_format = Some(chunk.format);
                            }
                            if guard.status == NativeSinkStatus::Buffering {
                                guard.status = NativeSinkStatus::Playing;
                            }
                        }

                        // Resample if necessary
                        let source_format = chunk.format;
                        let mut samples_to_process = if source_format == output_format {
                            chunk.samples
                        } else {
                            resample_interleaved(
                                &chunk.samples,
                                chunk.frames,
                                source_format,
                                output_format,
                            )
                        };

                        // DSP Stage (10-Band EQ -> ReplayGain -> Smooth Ramp -> Soft Limiter)
                        {
                            let mut dsp = dsp_arc.lock().unwrap_or_else(|p| p.into_inner());
                            dsp.set_format(output_format);
                            dsp.process_interleaved(&mut samples_to_process);
                        }

                        // Push into endpoint queue
                        {
                            let ep = endpoint_arc.lock().unwrap_or_else(|p| p.into_inner());
                            ep.push_interleaved(&samples_to_process);
                        }
                    }
                    Ok(StreamDecodeOutput::NeedMoreData) => {
                        // Network underflow: wait for more data to arrive into StreamCache
                        underflow_count.fetch_add(1, Ordering::Relaxed);
                        {
                            let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
                            if guard.status == NativeSinkStatus::Playing {
                                guard.status = NativeSinkStatus::Buffering;
                            }
                        }
                        thread::sleep(Duration::from_millis(8));
                    }
                    Ok(StreamDecodeOutput::EndOfStream) => {
                        // Wait for endpoint queued frames to drain
                        while !stop.load(Ordering::Acquire) {
                            let remaining = {
                                let ep = endpoint_arc.lock().unwrap_or_else(|p| p.into_inner());
                                ep.queued_frames()
                            };
                            if remaining == 0 {
                                break;
                            }
                            thread::sleep(Duration::from_millis(10));
                        }
                        if stop.load(Ordering::Acquire) {
                            break;
                        }

                        let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
                        guard.status = NativeSinkStatus::Ended;
                        wake.notify_all();
                    }
                    Err(_) => {
                        let mut guard = lock.lock().unwrap_or_else(|p| p.into_inner());
                        guard.status = NativeSinkStatus::Error;
                        wake.notify_all();
                        thread::sleep(Duration::from_millis(15));
                    }
                }
            }
        }));
    }
}

impl Drop for NativeAudioSink {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let (_, wake) = &*self.state;
        wake.notify_all();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_wav_bytes(sample_count: usize) -> Vec<u8> {
        let data_len = sample_count * 2;
        let riff_len = 36 + data_len;
        let mut bytes = Vec::with_capacity(44 + data_len);

        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(riff_len as u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&2u16.to_le_bytes()); // stereo
        bytes.extend_from_slice(&44100u32.to_le_bytes()); // 44.1 kHz
        bytes.extend_from_slice(&176400u32.to_le_bytes()); // byte rate
        bytes.extend_from_slice(&4u16.to_le_bytes()); // block align
        bytes.extend_from_slice(&16u16.to_le_bytes()); // 16 bits
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(data_len as u32).to_le_bytes());

        for i in 0..sample_count {
            let sample = ((i as f64 / 20.0).sin() * 12000.0) as i16;
            bytes.extend_from_slice(&sample.to_le_bytes());
        }

        bytes
    }

    #[test]
    fn audio_sink_lifecycle_and_virtual_pump_playback() {
        let wav_data = build_wav_bytes(800);
        let cache = Arc::new(Mutex::new(StreamCache::new("test-sink-cache", 1024 * 1024, None)));
        {
            let mut c = cache.lock().unwrap();
            c.set_total_length(wav_data.len() as u64);
            c.write_chunk(0, &wav_data).unwrap();
        }

        let sink = NativeAudioSink::new(
            "test-sink-1",
            cache,
            Some("wav".to_string()),
            Some("audio/wav".to_string()),
            None,
            true, // force_virtual for isolated test
        );

        let snap1 = sink.snapshot();
        assert_eq!(snap1.status, NativeSinkStatus::Idle);
        assert!(snap1.is_virtual);

        sink.play().unwrap();

        // Let the background pump thread process the chunk
        thread::sleep(Duration::from_millis(50));

        let snap2 = sink.snapshot();
        assert!(
            snap2.status == NativeSinkStatus::Playing || snap2.status == NativeSinkStatus::Ended
        );
        assert!(snap2.queued_frames > 0 || snap2.status == NativeSinkStatus::Ended);

        sink.pause().unwrap();
        let snap3 = sink.snapshot();
        assert_eq!(snap3.status, NativeSinkStatus::Paused);

        sink.seek_ms(0).unwrap();
        let snap4 = sink.snapshot();
        assert_eq!(snap4.position_ms, 0);

        sink.stop().unwrap();
        let snap5 = sink.snapshot();
        assert_eq!(snap5.status, NativeSinkStatus::Idle);

        // Verify device switching & exclusive flag
        sink.switch_device(Some("dac-exclusive-device"), true)
            .unwrap();
        let snap6 = sink.snapshot();
        assert_eq!(snap6.device_name.as_deref(), Some("dac-exclusive-device"));
        assert!(snap6.is_exclusive);
    }
}
