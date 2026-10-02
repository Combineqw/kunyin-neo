//! Host-neutral Rust core shared by the Electron and Tauri adapters.

pub mod download;
pub mod error;
pub mod lyrics;
pub mod metadata;
pub mod provider;
pub mod scan;
pub mod settings_io;
pub mod sync;
pub mod theme;

pub use error::AuroraError;

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{lyrics, settings_io};

    #[test]
    fn lrc_parser_keeps_line_timing_contract() {
        let lines = lyrics::parse_lrc("[00:01.20]First\n[00:02.50]Second");

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].start, 1_200);
        assert_eq!(lines[0].end, 2_500);
        assert_eq!(lines[0].text, "First");
        assert_eq!(lines[1].end, 0);
    }

    #[test]
    fn settings_merge_replaces_arrays_and_merges_objects() {
        let base = json!({"player": {"volume": 0.5, "modes": ["a", "b"]}});
        let patch = json!({"player": {"volume": 0.8, "modes": ["c"]}});

        assert_eq!(
            settings_io::deep_merge(&base, &patch),
            json!({"player": {"volume": 0.8, "modes": ["c"]}})
        );
    }

    #[test]
    fn settings_write_json_roundtrips_atomically() {
        let suffix = format!(
            "kunyin-settings-test-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock before unix epoch")
                .as_nanos()
        );
        let path = std::env::temp_dir().join(suffix);
        let source = json!({"version": 4, "player": {"volume": 0.8}});

        settings_io::write_settings_json(&path, &source.to_string()).expect("write settings");
        let written = settings_io::read_json(&path).expect("read settings");

        assert_eq!(written, source);
        let _ = std::fs::remove_file(path);
    }
}
