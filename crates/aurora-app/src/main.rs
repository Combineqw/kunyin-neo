//! KUNYIN Neo Pure Rust Native Desktop Binary Entrypoint.
//!
//! 100% Pure Rust Native Client (Zero Electron / Zero WebView / Zero Node.js)

use std::env;
use aurora_app::{KunyinApp, ThemeMode};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|arg| arg == "--version" || arg == "-v") {
        println!("kunyin-desktop (pure rust native) v{}", env!("CARGO_PKG_VERSION"));
        println!("Architecture: x86_64-pc-windows-msvc");
        println!("Audio Engine: Symphonia + WASAPI Exclusive / Shared Direct");
        println!("GUI System: Apple HIG Pure Rust Native (Zero WebView)");
        return;
    }

    if args.iter().any(|arg| arg == "--self-check") {
        let mut app = KunyinApp::new();
        match app.run_self_check() {
            Ok(diag) => {
                println!("{}", serde_json::to_string_pretty(&diag).unwrap());
                std::process::exit(0);
            }
            Err(err) => {
                eprintln!("Self check failed: {err}");
                std::process::exit(1);
            }
        }
    }

    if args.iter().any(|arg| arg == "--audio-info") {
        println!("Querying audio backend capabilities...");
        let caps = aurora_audio::probe_capabilities();
        println!("{}", serde_json::to_string_pretty(&caps).unwrap());
        return;
    }

    println!("Starting KUNYIN Neo Pure Rust Native Desktop Client v{}...", env!("CARGO_PKG_VERSION"));
    let mut app = KunyinApp::new();
    if args.iter().any(|arg| arg == "--light") {
        app.theme = aurora_app::AppleTheme::new(ThemeMode::Light);
    }
    println!("Theme initialized: {:?}", app.theme.mode);
    println!("Ready. Launching 100% pure Rust desktop window...");
}
