# Tauri Core Shell

This independent shell validates a Tauri command calling the same host-neutral `aurora-core` crate as Electron's N-API adapter. It does not replace or package the Electron application.

Run on a Windows machine with the Rust MSVC target, Microsoft C++ Build Tools, Windows SDK, and WebView2 Runtime installed:

```powershell
cargo run --manifest-path src-tauri/Cargo.toml
```

The static frontend sends a sample LRC string to `parse_lyrics` and displays the serialized Rust result. Tauri and tauri-build are pinned to stable v2 releases in `Cargo.toml`.
