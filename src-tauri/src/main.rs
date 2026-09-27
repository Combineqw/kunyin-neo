use aurora_core::lyrics::{parse_lrc, LyricLine};

#[tauri::command]
fn parse_lyrics(input: String) -> Vec<LyricLine> {
    parse_lrc(&input)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![parse_lyrics])
        .run(tauri::generate_context!())
        .expect("failed to run the Tauri shell");
}
