# aurora-audio

`aurora-audio` is the host-neutral boundary for the native playback migration.
It currently provides three deliberately small contracts:

- `probe_capabilities()` reports the implementation that is actually present.
  Local, unencrypted files can now be decoded by Symphonia, while desktop
  output remains Chromium `HTMLAudioElement` with an explicit fallback. The
  probe does not claim device output or WASAPI exclusive mode.
- `PcmRingBuffer` is a bounded interleaved `f32` transport for a future
  decoder/output pair. It drops whole oldest frames when a device consumer is
  stalled, keeping memory bounded and exposing the dropped-frame count.
- `NativeAudioDecoder` pulls one packet at a time and emits packet-sized
  interleaved `f32` chunks. `decode_frames()` is a bounded probe helper for
  tests and metadata inspection; it never loads a complete track.

The crate has no device output implementation yet, so tests run without an
audio device. Remote streams, provider encryption, and actual Electron audio
output remain outside this slice. A future Windows output implementation can
add an adapter over this boundary without changing Electron's fallback
contract.
