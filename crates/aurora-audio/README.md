# aurora-audio

`aurora-audio` is the host-neutral boundary for the native playback migration.
It currently provides two deliberately small contracts:

- `probe_capabilities()` reports the implementation that is actually present.
  The current desktop output is still Chromium `HTMLAudioElement` with an
  explicit fallback; the probe does not claim a decoder, device output, or
  WASAPI exclusive mode.
- `PcmRingBuffer` is a bounded interleaved `f32` transport for a future
  decoder/output pair. It drops whole oldest frames when a device consumer is
  stalled, keeping memory bounded and exposing the dropped-frame count.

The crate has no device or decoder dependency yet, so tests run without an
audio device. A future Windows output implementation can add an adapter over
this boundary without changing Electron's fallback contract.
