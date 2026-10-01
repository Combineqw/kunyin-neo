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
- `NativePlaybackEngine` owns a local-file session around the decoder and
  output controller. It provides play, pause, stop, seek, and snapshot
  operations. Decoder work runs on a joinable worker and feeds the bounded
  queue; the device callback remains non-blocking and reports silence on an
  underrun or lock miss. Source packets are linearly resampled and mapped to
  the device channel count before they enter the output queue.

`NativeOutputController` selects the Windows default output endpoint and its
default shared-mode format, then exposes `start`, `pause`, `stop`, `snapshot`,
and bounded queue methods. The callback uses `try_lock` and writes silence on a
lock miss or queue underrun, so it never waits behind decoder work. The native
host adapter opts into this engine for local files and keeps the existing
HTMLAudio fallback when native startup fails. Remote streams, provider
encryption, DSP, and WASAPI exclusive mode remain outside this slice.

The crate also provides `Qmc2Decryptor` and `decrypt_qmc2_chunk` for QQ
`mflac`/`mgg` streams. It supports ekey V1/V2 and map/RC4 modes, and takes the
absolute encrypted-file offset so HTTP Range chunks remain independently
decryptable. The Electron adapter uses the native path first and keeps its
TypeScript fallback for compatibility.

The N-API adapter also exposes a bounded 256 KiB in-place file operation for
completed downloads. It reuses the same offset-based decryptor and never loads
the complete encrypted file into one native buffer.
