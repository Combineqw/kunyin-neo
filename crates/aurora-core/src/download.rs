//! Bounded, streaming download commit protocol.
//!
//! The host owns HTTP (and therefore Electron proxy/auth semantics). This
//! module owns the byte protocol: bounded writes, SHA-256, cancellation
//! cleanup, and the final temporary-file commit.

use std::fs::{remove_file, rename, File, OpenOptions};
use std::io::Write;
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

#[cfg(test)]
mod tests {
    use super::DownloadSession;
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
}
