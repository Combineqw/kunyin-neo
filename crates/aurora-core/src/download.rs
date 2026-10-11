//! Bounded, streaming download commit protocol.
//!
//! The host owns HTTP (and therefore Electron proxy/auth semantics). This
//! module owns the byte protocol: bounded writes, SHA-256, cancellation
//! cleanup, and the final temporary-file commit.

use std::fs::{remove_file, rename, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::{AuroraError, Result};

const DEFAULT_MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub written_bytes: u64,
    pub max_bytes: u64,
    pub sha256: Option<String>,
}

/// A single bounded temporary-file transaction.
pub struct DownloadSession {
    temp_path: PathBuf,
    final_path: PathBuf,
    max_bytes: u64,
    written_bytes: u64,
    hasher: Sha256,
    file: File,
}

impl DownloadSession {
    pub fn create(temp_path: &Path, final_path: &Path, max_bytes: u64) -> Result<Self> {
        if temp_path.as_os_str().is_empty() || final_path.as_os_str().is_empty() {
            return Err(AuroraError::InvalidInput("下载路径不能为空".into()));
        }
        let max_bytes = if max_bytes == 0 {
            DEFAULT_MAX_BYTES
        } else {
            max_bytes
        };
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(temp_path)
            .map_err(|error| AuroraError::tag(temp_path, error))?;
        Ok(Self {
            temp_path: temp_path.to_path_buf(),
            final_path: final_path.to_path_buf(),
            max_bytes,
            written_bytes: 0,
            hasher: Sha256::new(),
            file,
        })
    }

    pub fn write_chunk(&mut self, chunk: &[u8]) -> Result<DownloadProgress> {
        let chunk_len = u64::try_from(chunk.len())
            .map_err(|_| AuroraError::InvalidInput("下载块过大".into()))?;
        let next = self
            .written_bytes
            .checked_add(chunk_len)
            .ok_or_else(|| AuroraError::InvalidInput("下载大小溢出".into()))?;
        if next > self.max_bytes {
            return Err(AuroraError::InvalidInput(format!(
                "响应体超过上限 {} bytes",
                self.max_bytes
            )));
        }
        self.file
            .write_all(chunk)
            .map_err(|error| AuroraError::tag(&self.temp_path, error))?;
        self.hasher.update(chunk);
        self.written_bytes = next;
        Ok(self.progress(false))
    }

    pub fn commit(mut self) -> Result<DownloadProgress> {
        if let Err(error) = self.file.flush().and_then(|_| self.file.sync_all()) {
            drop(self.file);
            let _ = remove_file(&self.temp_path);
            return Err(AuroraError::tag(&self.temp_path, error));
        }
        let progress = self.progress(true);
        drop(self.file);
        if self.final_path.exists() {
            if let Err(error) = remove_file(&self.final_path) {
                let _ = remove_file(&self.temp_path);
                return Err(AuroraError::tag(&self.final_path, error));
            }
        }
        if let Err(error) = rename(&self.temp_path, &self.final_path) {
            let _ = remove_file(&self.temp_path);
            return Err(AuroraError::tag(&self.final_path, error));
        }
        Ok(progress)
    }

    pub fn abort(self) -> Result<()> {
        drop(self.file);
        if self.temp_path.exists() {
            remove_file(&self.temp_path)
                .map_err(|error| AuroraError::tag(&self.temp_path, error))?;
        }
        Ok(())
    }

    fn progress(&self, complete: bool) -> DownloadProgress {
        DownloadProgress {
            written_bytes: self.written_bytes,
            max_bytes: self.max_bytes,
            sha256: complete.then(|| format!("{:x}", self.hasher.clone().finalize())),
        }
    }
}

/// Sniffs real container audio extension from raw header bytes (.ogg, .flac, .mp3).
pub fn sniff_audio_extension_bytes(header: &[u8]) -> Option<&'static str> {
    if header.len() < 4 {
        return None;
    }
    if &header[0..4] == b"OggS" {
        return Some(".ogg");
    }
    if &header[0..4] == b"fLaC" {
        return Some(".flac");
    }
    if (header.len() >= 3 && &header[0..3] == b"ID3")
        || (header[0] == 0xff && (header[1] & 0xe0) == 0xe0)
    {
        return Some(".mp3");
    }
    None
}

/// Sniffs real container audio extension by reading the first 12 bytes of a file.
pub fn sniff_audio_extension_file(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut header = [0u8; 12];
    let n = file.read(&mut header).ok()?;
    sniff_audio_extension_bytes(&header[..n]).map(|ext| ext.to_string())
}

/// Parses FLAC sample rate and bit depth from STREAMINFO block (42 bytes).
/// Formats as `[bitsPerSample]Bit-[kHz]kHz`, e.g. `[16Bit-44.1kHz]`, `[24Bit-96kHz]`.
pub fn parse_flac_streaminfo_tag_bytes(buf: &[u8]) -> Option<String> {
    if buf.len() < 42 || &buf[0..4] != b"fLaC" {
        return None;
    }
    let sample_rate = (((buf[18] as u32) << 12)
        | ((buf[19] as u32) << 4)
        | ((buf[20] as u32) >> 4))
        & 0xfffff;
    let bits_per_sample = (((buf[20] & 0x01) << 4) | (buf[21] >> 4)) + 1;
    if sample_rate == 0 {
        return None;
    }
    let khz = (sample_rate as f64) / 1000.0;
    let khz_formatted = format!("{:.1}", khz);
    let khz_str = khz_formatted.strip_suffix(".0").unwrap_or(&khz_formatted);
    Some(format!("[{}Bit-{}kHz]", bits_per_sample, khz_str))
}

/// Parses FLAC sample rate and bit depth tag directly from file header.
pub fn parse_flac_streaminfo_tag_file(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut buf = [0u8; 42];
    let n = file.read(&mut buf).ok()?;
    if n < 42 {
        return None;
    }
    parse_flac_streaminfo_tag_bytes(&buf)
}

/// Normalizes song title for alternative download source matching.
/// Removes spaces, punctuation, brackets, quotes and converts to lowercase.
pub fn normalize_download_title(s: &str) -> String {
    let chars_to_remove = " ()（）【】—-—_·,.，。'\"'\"\"?!:：;；";
    s.to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace() && !chars_to_remove.contains(*c))
        .collect()
}

/// Searches candidate list for first match with normalized title matching target_title.
pub fn select_alternative_candidate(target_title: &str, candidates_json: &str) -> Option<String> {
    let target = normalize_download_title(target_title);
    if target.is_empty() {
        return None;
    }
    let list: Vec<serde_json::Value> = serde_json::from_str(candidates_json).ok()?;
    for item in list {
        let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
        if normalize_download_title(title) == target {
            return serde_json::to_string(&item).ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn paths(label: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let root =
            std::env::temp_dir().join(format!("aurora-download-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp root");
        (root.join("part.tmp"), root.join("done.bin"))
    }

    #[test]
    fn bounded_commit_hashes_without_buffering() {
        let (temp, final_path) = paths("commit");
        let mut session = DownloadSession::create(&temp, &final_path, 16).expect("create");
        session.write_chunk(b"abc").expect("chunk");
        let progress = session.commit().expect("commit");
        assert_eq!(progress.written_bytes, 3);
        assert_eq!(
            progress.sha256.as_deref(),
            Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        );
        assert_eq!(fs::read(final_path).expect("read"), b"abc");
    }

    #[test]
    fn limit_failure_and_abort_remove_temp_file() {
        let (temp, final_path) = paths("abort");
        let mut session = DownloadSession::create(&temp, &final_path, 2).expect("create");
        assert!(session.write_chunk(b"123").is_err());
        session.abort().expect("abort");
        assert!(!temp.exists());
        assert!(!final_path.exists());
    }

    #[test]
    fn test_sniff_audio_extension_bytes() {
        assert_eq!(sniff_audio_extension_bytes(b"OggS\x00\x02"), Some(".ogg"));
        assert_eq!(sniff_audio_extension_bytes(b"fLaC\x00\x00"), Some(".flac"));
        assert_eq!(sniff_audio_extension_bytes(b"ID3\x03\x00"), Some(".mp3"));
        assert_eq!(sniff_audio_extension_bytes(&[0xff, 0xfb, 0x90, 0x64]), Some(".mp3"));
        assert_eq!(sniff_audio_extension_bytes(b"RIFF\x00\x00"), None);
    }

    #[test]
    fn test_parse_flac_streaminfo_tag_bytes() {
        // Construct a synthetic 42-byte FLAC header:
        // magic: "fLaC" (4 bytes)
        // block header: 4 bytes (8..18 is block body)
        // offset 18: sample rate 44100 (0x0AC44) and bits per sample 16 (15 + 1)
        // 44100 = 0b0000 1010 1100 0100 0100 (20 bits)
        // b[18] = 0x0a (upper 8 bits: 0000 1010)
        // b[19] = 0xc4 (mid 8 bits: 1100 0100)
        // b[20] = 0x40 | ((16 - 1) << 4)...
        let mut buf = [0u8; 42];
        buf[0..4].copy_from_slice(b"fLaC");
        // sample_rate = 44100 (0xAC44):
        // high 8 bits = (44100 >> 12) = 10 (0x0A)
        // mid 8 bits = (44100 >> 4) & 0xff = 196 (0xC4)
        // low 4 bits = 44100 & 0x0f = 4 (0x04)
        // bits_per_sample = 16 -> (16 - 1) = 15 = 0x0F (5 bits: 1 bit in b[20], 4 bits in b[21])
        buf[18] = 10;
        buf[19] = 196;
        buf[20] = (4 << 4) | 0x00; // low 4 bits of sample rate, high bit of bits_per_sample (0)
        buf[21] = 0xF0; // lower 4 bits of bits_per_sample = 0x0F (so bits_per_sample = 16)
        let tag = parse_flac_streaminfo_tag_bytes(&buf);
        assert_eq!(tag.as_deref(), Some("[16Bit-44.1kHz]"));
    }

    #[test]
    fn test_normalize_download_title_and_select_alternative() {
        assert_eq!(
            normalize_download_title("七里香 (Live) - 周杰伦"),
            "七里香live周杰伦"
        );
        let candidates = r#"[
            {"title": "七里香", "artist": "周杰伦", "id": 1},
            {"title": "七里香 (Live)", "artist": "周杰伦", "id": 2}
        ]"#;
        let selected = select_alternative_candidate("七里香", candidates);
        assert!(selected.is_some());
        assert!(selected.unwrap().contains("\"id\":1"));
    }
}
