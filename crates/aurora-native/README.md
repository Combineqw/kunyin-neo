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
