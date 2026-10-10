//! Bounded stream cache and virtual seekable reader for remote audio pipeline.
//!
//! Provides a two-tier cache (in-memory chunk window with optional temp-file spill)
//! that implements `std::io::Read` and `std::io::Seek`, allowing Symphonia or any
//! native decoder to consume non-contiguous HTTP Range chunks as a standard media stream.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::remote::BufferedRanges;

/// Serializable snapshot of the stream cache status and resource usage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStats {
    pub cache_id: String,
    pub total_cached_bytes: u64,
    pub memory_bytes: usize,
    pub file_bytes: u64,
    pub read_cursor: u64,
    pub contiguous_forward_bytes: u64,
    pub buffered_range_count: usize,
    pub is_spilled_to_file: bool,
}

/// A chunk stored in memory.
#[derive(Debug, Clone)]
struct MemChunk {
    offset: u64,
    data: Vec<u8>,
}

/// Two-tier seekable stream cache for streaming media.
///
/// Holds incoming non-contiguous HTTP Range chunks in a bounded in-memory structure.
/// If the data exceeds the memory threshold or an explicit spill is requested, it
/// automatically spills to an underlying temporary file without disrupting read/seek cursors.
pub struct StreamCache {
    pub id: String,
    max_memory_bytes: usize,
    memory_chunks: BTreeMap<u64, MemChunk>,
    current_memory_bytes: usize,
    spill_file: Option<File>,
    spill_file_path: Option<PathBuf>,
    temp_dir: Option<PathBuf>,
    buffered_ranges: BufferedRanges,
    read_cursor: u64,
    total_length: Option<u64>,
}

impl StreamCache {
    /// Create a new stream cache.
    ///
    /// `max_memory_bytes` controls how many bytes can be kept in RAM before spilling to disk.
    /// If `temp_dir` is provided, spilled files are stored there; otherwise the OS temp dir is used.
    pub fn new(id: impl Into<String>, max_memory_bytes: usize, temp_dir: Option<PathBuf>) -> Self {
        let max_mem = max_memory_bytes.max(256 * 1024); // minimum 256 KiB
        Self {
            id: id.into(),
            max_memory_bytes: max_mem,
            memory_chunks: BTreeMap::new(),
            current_memory_bytes: 0,
            spill_file: None,
            spill_file_path: None,
            temp_dir,
            buffered_ranges: BufferedRanges::new(),
            read_cursor: 0,
            total_length: None,
        }
    }

    /// Set known total stream length (from Content-Length or Content-Range).
    pub fn set_total_length(&mut self, total: u64) {
        if total > 0 {
            self.total_length = Some(total);
        }
    }

    /// Write an incoming stream chunk at `offset`.
    pub fn write_chunk(&mut self, offset: u64, data: &[u8]) -> io::Result<usize> {
        if data.is_empty() {
            return Ok(0);
        }

        // If already spilled to disk, write directly to file
        if let Some(file) = &mut self.spill_file {
            file.seek(SeekFrom::Start(offset))?;
            file.write_all(data)?;
            file.flush()?;
            self.buffered_ranges.insert(offset, data.len() as u64);
            return Ok(data.len());
        }

        // Check if adding this chunk exceeds memory limit
        if self.current_memory_bytes.saturating_add(data.len()) > self.max_memory_bytes {
            self.spill_to_disk()?;
            return self.write_chunk(offset, data);
        }

        // Keep in memory
        self.memory_chunks.insert(
            offset,
            MemChunk {
                offset,
                data: data.to_vec(),
            },
        );
        self.current_memory_bytes = self.current_memory_bytes.saturating_add(data.len());
        self.buffered_ranges.insert(offset, data.len() as u64);

        Ok(data.len())
    }

    /// Explicitly transition all cached chunks to disk storage.
    fn spill_to_disk(&mut self) -> io::Result<()> {
        if self.spill_file.is_some() {
            return Ok(());
        }

        let dir = self.temp_dir.clone().unwrap_or_else(std::env::temp_dir);
        let filename = format!(
            "kunyin-cache-{}-{}-{}.tmp",
            self.id,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let path = dir.join(filename);

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;

        // Write all chunks in memory into the file at their respective offsets
        for chunk in self.memory_chunks.values() {
            file.seek(SeekFrom::Start(chunk.offset))?;
            file.write_all(&chunk.data)?;
        }
        file.flush()?;

        // Free RAM
        self.memory_chunks.clear();
        self.current_memory_bytes = 0;

        self.spill_file_path = Some(path);
        self.spill_file = Some(file);

        Ok(())
    }

    /// Read data at a specific offset without modifying `read_cursor`.
    pub fn read_at(&mut self, offset: u64, max_len: usize) -> io::Result<Option<Vec<u8>>> {
        let contiguous = self.buffered_ranges.contiguous_bytes_from(offset);
        if contiguous == 0 {
            return Ok(None);
        }

        let to_read = (contiguous as usize).min(max_len);
        let mut buf = vec![0u8; to_read];

        if let Some(file) = &mut self.spill_file {
            let saved = file.stream_position()?;
            file.seek(SeekFrom::Start(offset))?;
            file.read_exact(&mut buf)?;
            file.seek(SeekFrom::Start(saved))?;
            return Ok(Some(buf));
        }

        // Read from memory chunks
        let mut bytes_copied = 0;
        let mut current_pos = offset;

        for chunk in self.memory_chunks.values() {
            let chunk_end = chunk.offset.saturating_add(chunk.data.len() as u64);
            if current_pos >= chunk.offset && current_pos < chunk_end {
                let start_in_chunk = (current_pos - chunk.offset) as usize;
                let available_in_chunk = chunk.data.len() - start_in_chunk;
                let needed = to_read - bytes_copied;
                let copy_len = needed.min(available_in_chunk);

                buf[bytes_copied..bytes_copied + copy_len]
                    .copy_from_slice(&chunk.data[start_in_chunk..start_in_chunk + copy_len]);

                bytes_copied += copy_len;
                current_pos += copy_len as u64;

                if bytes_copied >= to_read {
                    break;
                }
            }
        }

        if bytes_copied > 0 {
            buf.truncate(bytes_copied);
            Ok(Some(buf))
        } else {
            Ok(None)
        }
    }

    /// Check if a specific byte offset is available in the cache.
    pub fn is_cached(&self, offset: u64) -> bool {
        self.buffered_ranges.contains_offset(offset)
    }

    /// Return statistics snapshot of the cache.
    pub fn stats(&self) -> CacheStats {
        let file_bytes = self
            .spill_file_path
            .as_ref()
            .and_then(|p| fs::metadata(p).ok())
            .map(|m| m.len())
            .unwrap_or(0);

        CacheStats {
            cache_id: self.id.clone(),
            total_cached_bytes: self.buffered_ranges.total_bytes(),
            memory_bytes: self.current_memory_bytes,
            file_bytes,
            read_cursor: self.read_cursor,
            contiguous_forward_bytes: self.buffered_ranges.contiguous_bytes_from(self.read_cursor),
            buffered_range_count: self.buffered_ranges.count(),
            is_spilled_to_file: self.spill_file.is_some(),
        }
    }

    /// Return reference to buffered ranges tracker.
    pub fn buffered_ranges(&self) -> &BufferedRanges {
        &self.buffered_ranges
    }

    /// Return current read cursor position.
    pub fn current_cursor(&self) -> u64 {
        self.read_cursor
    }
}

impl Read for StreamCache {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let contiguous = self.buffered_ranges.contiguous_bytes_from(self.read_cursor);
        if contiguous == 0 {
            // Buffer underrun or unbuffered gap at current cursor
            return Ok(0);
        }

        let to_read = (contiguous as usize).min(buf.len());

        if let Some(file) = &mut self.spill_file {
            file.seek(SeekFrom::Start(self.read_cursor))?;
            let n = file.read(&mut buf[..to_read])?;
            self.read_cursor = self.read_cursor.saturating_add(n as u64);
            return Ok(n);
        }

        // Read from memory chunks
        let mut bytes_copied = 0;
        let mut current_pos = self.read_cursor;

        for chunk in self.memory_chunks.values() {
            let chunk_end = chunk.offset.saturating_add(chunk.data.len() as u64);
            if current_pos >= chunk.offset && current_pos < chunk_end {
                let start_in_chunk = (current_pos - chunk.offset) as usize;
                let available_in_chunk = chunk.data.len() - start_in_chunk;
                let needed = to_read - bytes_copied;
                let copy_len = needed.min(available_in_chunk);

                buf[bytes_copied..bytes_copied + copy_len]
                    .copy_from_slice(&chunk.data[start_in_chunk..start_in_chunk + copy_len]);

                bytes_copied += copy_len;
                current_pos += copy_len as u64;

                if bytes_copied >= to_read {
                    break;
                }
            }
        }

        self.read_cursor = self.read_cursor.saturating_add(bytes_copied as u64);
        Ok(bytes_copied)
    }
}

impl Seek for StreamCache {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos: i64 = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::Current(offset) => (self.read_cursor as i64).saturating_add(offset),
            SeekFrom::End(offset) => {
                let total = self
                    .total_length
                    .or_else(|| self.buffered_ranges.ranges().last().map(|r| r.end))
                    .unwrap_or(0);
                (total as i64).saturating_add(offset)
            }
        };

        if new_pos < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot seek to negative position",
            ));
        }

        self.read_cursor = new_pos as u64;
        if let Some(file) = &mut self.spill_file {
            file.seek(SeekFrom::Start(self.read_cursor))?;
        }

        Ok(self.read_cursor)
    }
}

impl Drop for StreamCache {
    fn drop(&mut self) {
        // Close file handle and clean up temp spill file
        self.spill_file.take();
        if let Some(path) = self.spill_file_path.take() {
            let _ = fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_cache_read_and_seek_operations() {
        let mut cache = StreamCache::new("test-mem", 1024 * 1024, None);
        assert_eq!(cache.read_cursor, 0);

        // Write 3 contiguous chunks
        cache.write_chunk(0, b"Hello, ").unwrap();
        cache.write_chunk(7, b"world! ").unwrap();
        cache.write_chunk(14, b"Stream cache works.").unwrap();

        assert_eq!(cache.stats().total_cached_bytes, 33);
        assert_eq!(cache.stats().is_spilled_to_file, false);

        // Sequential read
        let mut buf = [0u8; 13];
        let n = cache.read(&mut buf).unwrap();
        assert_eq!(n, 13);
        assert_eq!(&buf[..n], b"Hello, world!");
        assert_eq!(cache.current_cursor(), 13);

        // Seek forward
        cache.seek(SeekFrom::Start(14)).unwrap();
        let mut buf2 = [0u8; 19];
        let n2 = cache.read(&mut buf2).unwrap();
        assert_eq!(n2, 19);
        assert_eq!(&buf2[..n2], b"Stream cache works.");

        // Read at gap (seek to 1000)
        cache.seek(SeekFrom::Start(1000)).unwrap();
        let mut empty = [0u8; 10];
        assert_eq!(cache.read(&mut empty).unwrap(), 0);
    }

    #[test]
    fn cache_spills_to_disk_when_memory_limit_exceeded() {
        // Set low memory limit: 256 KiB
        let mut cache = StreamCache::new("test-spill", 256 * 1024, None);

        // Write 200 KiB chunk (fits in RAM)
        let chunk1 = vec![0xAAu8; 200 * 1024];
        cache.write_chunk(0, &chunk1).unwrap();
        assert_eq!(cache.stats().is_spilled_to_file, false);
        assert_eq!(cache.stats().memory_bytes, 200 * 1024);

        // Write another 100 KiB chunk (total 300 KiB > 256 KiB limit -> triggers spill)
        let chunk2 = vec![0xBBu8; 100 * 1024];
        cache.write_chunk(200 * 1024, &chunk2).unwrap();

        assert_eq!(cache.stats().is_spilled_to_file, true);
        assert_eq!(cache.stats().memory_bytes, 0); // RAM freed
        assert_eq!(cache.stats().file_bytes, 300 * 1024);

        // Read across boundary from file
        cache.seek(SeekFrom::Start((200 * 1024) - 2)).unwrap();
        let mut boundary = [0u8; 4];
        cache.read_exact(&mut boundary).unwrap();
        assert_eq!(boundary, [0xAA, 0xAA, 0xBB, 0xBB]);

        // Temp file is automatically deleted on drop
        let path = cache.spill_file_path.clone().unwrap();
        assert!(path.exists());
        drop(cache);
        assert!(!path.exists());
    }

    #[test]
    fn discontinuous_chunks_and_read_at() {
        let mut cache = StreamCache::new("test-discontinuous", 512 * 1024, None);
        cache.write_chunk(0, b"AAAA").unwrap();
        cache.write_chunk(100, b"BBBB").unwrap();

        assert_eq!(cache.stats().buffered_range_count, 2);
        assert!(cache.is_cached(0));
        assert!(cache.is_cached(100));
        assert!(!cache.is_cached(50));

        let peek = cache.read_at(100, 4).unwrap();
        assert_eq!(peek, Some(b"BBBB".to_vec()));
        assert_eq!(cache.current_cursor(), 0); // read_cursor untouched
    }
}
