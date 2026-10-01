# aurora-native

Electron's Node-API adapter for the shared `aurora-core` crate. The exported JavaScript API and JSON-string response shapes remain compatible with the Electron bridge and shadow comparison scripts.

## 目录

| 文件 | 作用 |
| --- | --- |
| `src/lib.rs` | napi 导出层，对外只暴露返回 JSON 字符串的纯函数 |
| `../aurora-core/src/scan.rs` | Library scanning and audio metadata parsing |
| `../aurora-core/src/lyrics.rs` | LRC line parsing and directory scan |
| `../aurora-core/src/settings_io.rs` | Settings JSON read, merge, and sandbox roundtrip |
| `../aurora-core/src/error.rs` | Host-neutral error type |

## 命令

```bash
npm run native:build
npm run native:compare
npm run native:test
```

`native:compare` verifies the existing Node/N-API contract against the fixture set. `aurora-core` is also linked by the separate Tauri shell in `src-tauri/`.

## QMC2 stream decryption

`audioDecryptQmc2Chunk(ekey, fileOffset, chunk)` decrypts one QQ
`mflac`/`mgg` range chunk through Rust. The adapter caches only the current
ekey's decryptor, keeping repeated HTTP chunks cheap while bounding native
memory. Invalid keys and unavailable native exports are converted to the
existing Electron TypeScript fallback.

## 本地播放导出

Windows 构建还导出本地文件播放的可选 N-API：

| 导出 | 作用 |
| --- | --- |
| `nativeAudioStartFile` | 打开本地文件并创建 Rust 播放会话 |
| `nativeAudioPlay` / `nativeAudioPause` | 启动或暂停 native 输出 |
| `nativeAudioSeek` | 按毫秒定位并按原状态继续播放 |
| `nativeAudioStop` | 停止并重置当前会话 |
| `nativeAudioSnapshot` | 读取状态、位置、时长、设备、排队帧和丢帧数 |

Electron 对这些导出采用可选能力探测：本地文件优先尝试 Rust
`NativePlaybackEngine`，模块缺失或启动失败时回退现有 HTMLAudio 播放；
Rust 会在送入 WASAPI shared 队列前完成采样率和声道转换。远程和加密流不
经过这组导出。
