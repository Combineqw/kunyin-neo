//! Host-neutral Rust core shared by the Electron and Tauri adapters.

pub mod download;
pub mod error;
pub mod lyrics;
pub mod metadata;
pub mod network;
pub mod provider;
pub mod scan;
pub mod settings_io;
pub mod sync;
pub mod theme;

pub use error::AuroraError;

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{lyrics, network, settings_io};

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
    fn timeline_find_active_line_works() {
        let lines = lyrics::parse_lrc("[00:01.00]Line 1\n[00:03.00]Line 2\n[00:06.00]Line 3");
        let active = lyrics::find_active_line(&lines, 2000).expect("find active");
        assert_eq!(active.index, 0);
        assert_eq!(active.text, "Line 1");
        assert_eq!(active.next_text.as_deref(), Some("Line 2"));
        assert!((active.progress - 0.5).abs() < 0.01);

        let active2 = lyrics::find_active_line(&lines, 4500).expect("find active line 2");
        assert_eq!(active2.index, 1);
        assert_eq!(active2.text, "Line 2");
        assert_eq!(active2.prev_text.as_deref(), Some("Line 1"));
        assert_eq!(active2.next_text.as_deref(), Some("Line 3"));
    }

    #[test]
    fn desktop_and_mini_player_appearance_calculators_work() {
        let dl = lyrics::compute_desktop_lyric_appearance(0.45, 80.0, 600, 0.7, 2);
        assert_eq!(dl.background_rgba, "rgba(9, 12, 11, 0.45)");
        assert_eq!(dl.text_opacity_str, "0.80");

        let mp = lyrics::compute_mini_player_appearance(75.0);
        assert_eq!(mp.bg_rgba, "rgba(31, 33, 31, 0.75)");
        assert_eq!(mp.hover_rgba, "rgba(35, 37, 34, 0.93)");
    }

    #[test]
    fn network_headers_and_proxy_validation_work() {
        let h = network::build_network_headers("qq", "https://y.qq.com/song", Some(0), Some(1024));
        assert_eq!(h.headers.get("Referer").map(|s| s.as_str()), Some("https://y.qq.com/"));
        assert_eq!(h.headers.get("Range").map(|s| s.as_str()), Some("bytes=0-1024"));

        let proxy = network::validate_and_normalize_proxy(true, "127.0.0.1", 7890).expect("proxy ok");
        assert_eq!(proxy, Some("http://127.0.0.1:7890".to_string()));
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
