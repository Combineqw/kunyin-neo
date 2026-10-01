# aurora-audio

`aurora-audio` is the host-neutral boundary for the native playback migration.
It currently provides these deliberately small contracts:

- `probe_capabilities()` reports the implementation that is actually present.
  Local, unencrypted files can now be decoded by Symphonia. Windows builds
  expose a CPAL/WASAPI shared-mode output boundary; other hosts retain the
  Chromium `HTMLAudioElement` fallback. WASAPI exclusive mode is not claimed.
- `PcmRingBuffer` is a bounded interleaved `f32` transport for a decoder/output
  pair. It drops whole oldest frames when a device consumer is stalled, keeping
  memory bounded and exposing the dropped-frame count. `SharedPcmQueue` wraps
  it for a decoder thread and a non-blocking real-time callback.
- `NativeAudioDecoder` pulls one packet at a time and emits packet-sized
  interleaved `f32` chunks. `decode_frames()` is a bounded probe helper for
  tests and metadata inspection; it never loads a complete track.

`NativeOutputController` selects the Windows default output endpoint and its
default shared-mode format, then exposes `start`, `pause`, `stop`, `snapshot`,
and bounded queue methods. The callback uses `try_lock` and writes silence on a
lock miss or queue underrun, so it never waits behind decoder work. The native
host adapter still decides when to opt in; remote streams, provider encryption,
resampling, and actual Electron playback integration remain outside this slice.
