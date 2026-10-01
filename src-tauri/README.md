# Kunyin Native Rust

This native Rust slice runs through Tauri and calls the same host-neutral
`aurora-core` crate used by Electron's N-API adapter. It provides settings
read/write, local directory scanning, LRC parsing, a schema-v9 SQLite library
repository (`aurora-library`), and read-only audio metadata extraction. The
Electron application remains the default product while the remaining domains
migrate.

Run on a Windows machine with the Rust MSVC target, Microsoft C++ Build Tools, Windows SDK, and WebView2 Runtime installed:

```powershell
cargo run --manifest-path src-tauri/Cargo.toml
```

The frontend exercises `parse_lyrics`, `scan_library`, `read_settings`,
`update_settings`, `read_audio_metadata`, the `library_*` commands (including
the paginated read-only `library_search_songs(query, limit, offset)` command),
and `native_capabilities`. Tauri and tauri-build are pinned to stable v2
releases in `Cargo.toml`. The bundle target is enabled in `tauri.conf.json`;
use the Tauri CLI to produce the platform installer.
