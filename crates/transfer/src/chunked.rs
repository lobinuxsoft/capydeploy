use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::types::Chunk;
use crate::{DEFAULT_CHUNK_SIZE, TransferError};

/// Error returned when a chunk checksum does not match.
#[derive(Debug, thiserror::Error)]
#[error("checksum mismatch")]
pub struct ChecksumError;

// ---------------------------------------------------------------------------
// Checksum helpers
// ---------------------------------------------------------------------------

/// Computes SHA-256 of `data` and returns the hex-encoded digest.
pub fn checksum_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Computes SHA-256 of an entire file and returns the hex-encoded digest.
pub fn calculate_file_checksum(path: &Path) -> Result<String, TransferError> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

// ---------------------------------------------------------------------------
// ChunkReader
// ---------------------------------------------------------------------------

/// Reads a file in fixed-size chunks with automatic SHA-256 checksums.
pub struct ChunkReader {
    file: std::fs::File,
    chunk_size: usize,
    offset: i64,
    file_path: String,
    file_size: i64,
}

impl ChunkReader {
    /// Opens `path` for chunked reading.
    ///
    /// If `chunk_size` is 0, [`DEFAULT_CHUNK_SIZE`] (1 MiB) is used.
    pub fn new(path: &Path, chunk_size: usize) -> Result<Self, TransferError> {
        let file = std::fs::File::open(path)?;
        let file_size = file.metadata()?.len() as i64;
        let chunk_size = if chunk_size == 0 {
            DEFAULT_CHUNK_SIZE
        } else {
            chunk_size
        };
        Ok(Self {
            file,
            chunk_size,
            offset: 0,
            file_path: path.to_string_lossy().into_owned(),
            file_size,
        })
    }

    /// Seeks to the given byte offset (for resume).
    pub fn seek_to(&mut self, offset: i64) -> Result<(), TransferError> {
        self.file.seek(SeekFrom::Start(offset as u64))?;
        self.offset = offset;
        Ok(())
    }

    /// Reads the next chunk. Returns `None` at EOF.
    pub fn next_chunk(&mut self) -> Result<Option<Chunk>, TransferError> {
        let remaining = self.file_size - self.offset;
        if remaining <= 0 {
            return Ok(None);
        }

        let read_size = std::cmp::min(remaining as usize, self.chunk_size);
        let mut buf = vec![0u8; read_size];
        let n = self.file.read(&mut buf)?;
        if n == 0 {
            return Ok(None);
        }
        buf.truncate(n);

        let checksum = checksum_bytes(&buf);
        let chunk = Chunk {
            offset: self.offset,
            size: n,
            data: buf,
            file_path: self.file_path.clone(),
            checksum,
        };
        self.offset += n as i64;
        Ok(Some(chunk))
    }

    /// Current byte offset.
    pub fn offset(&self) -> i64 {
        self.offset
    }

    /// Total file size in bytes.
    pub fn file_size(&self) -> i64 {
        self.file_size
    }

    /// Bytes remaining to read.
    pub fn remaining(&self) -> i64 {
        self.file_size - self.offset
    }
}

// ---------------------------------------------------------------------------
// ChunkWriter
// ---------------------------------------------------------------------------

/// Writes chunks to disk at exact offsets with optional checksum verification.
pub struct ChunkWriter {
    base_path: PathBuf,
    written: HashMap<String, i64>,
}

impl ChunkWriter {
    /// Creates a writer rooted at `base_path`.
    pub fn new(base_path: &Path) -> Self {
        Self {
            base_path: base_path.to_path_buf(),
            written: HashMap::new(),
        }
    }

    /// Writes a chunk to disk.
    ///
    /// - Creates intermediate directories as needed.
    /// - Verifies checksum if non-empty.
    /// - Updates internal written-offset tracking.
    pub fn write_chunk(&mut self, chunk: &Chunk) -> Result<(), TransferError> {
        // Validate path before joining to prevent directory traversal.
        crate::validate_upload_path(&chunk.file_path)?;

        // Verify checksum before writing.
        if !chunk.checksum.is_empty() {
            let actual = checksum_bytes(&chunk.data);
            if actual != chunk.checksum {
                return Err(TransferError::ChecksumMismatch);
            }
        }

        let full_path = self.base_path.join(&chunk.file_path);

        // Create parent directories.
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open/create file, seek to offset, write.
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&full_path)?;
        file.seek(SeekFrom::Start(chunk.offset as u64))?;
        let n = file.write(&chunk.data)?;

        // Track written offset.
        let new_offset = chunk.offset + n as i64;
        self.written.insert(chunk.file_path.clone(), new_offset);

        Ok(())
    }

    /// Returns the last written byte offset for `file_path` (0 if never written).
    pub fn get_written_offset(&self, file_path: &str) -> i64 {
        self.written.get(file_path).copied().unwrap_or(0)
    }

    /// Returns the base output path.
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, data: &[u8]) -> PathBuf {
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(data).unwrap();
        path
    }

    #[test]
    fn checksum_bytes_deterministic() {
        let c1 = checksum_bytes(b"hello world");
        let c2 = checksum_bytes(b"hello world");
        assert_eq!(c1, c2);
        assert_eq!(c1.len(), 64); // SHA-256 = 64 hex chars.
    }

    #[test]
    fn checksum_bytes_different_data() {
        let c1 = checksum_bytes(b"hello");
        let c2 = checksum_bytes(b"world");
        assert_ne!(c1, c2);
    }

    #[test]
    fn calculate_file_checksum_matches_bytes() {
        let dir = TempDir::new().unwrap();
        let data = b"test content for checksum";
        let path = create_test_file(dir.path(), "test.bin", data);

        let file_cs = calculate_file_checksum(&path).unwrap();
        let mem_cs = checksum_bytes(data);
        assert_eq!(file_cs, mem_cs);
    }

    #[test]
    fn chunk_reader_reads_all() {
        let dir = TempDir::new().unwrap();
        let data = b"AABBCCDDEE"; // 10 bytes.
        let path = create_test_file(dir.path(), "test.bin", data);

        let mut reader = ChunkReader::new(&path, 4).unwrap();
        assert_eq!(reader.file_size(), 10);
        assert_eq!(reader.remaining(), 10);

        let c1 = reader.next_chunk().unwrap().unwrap();
        assert_eq!(c1.offset, 0);
        assert_eq!(c1.size, 4);
        assert_eq!(&c1.data, b"AABB");
        assert!(!c1.checksum.is_empty());
        assert_eq!(reader.remaining(), 6);

        let c2 = reader.next_chunk().unwrap().unwrap();
        assert_eq!(c2.offset, 4);
        assert_eq!(c2.size, 4);
        assert_eq!(&c2.data, b"CCDD");

        let c3 = reader.next_chunk().unwrap().unwrap();
        assert_eq!(c3.offset, 8);
        assert_eq!(c3.size, 2);
        assert_eq!(&c3.data, b"EE");

        assert!(reader.next_chunk().unwrap().is_none());
    }

    #[test]
    fn chunk_reader_seek_and_resume() {
        let dir = TempDir::new().unwrap();
        let data = b"0123456789";
        let path = create_test_file(dir.path(), "test.bin", data);

        let mut reader = ChunkReader::new(&path, 4).unwrap();
        reader.seek_to(6).unwrap();
        assert_eq!(reader.offset(), 6);
        assert_eq!(reader.remaining(), 4);

        let c = reader.next_chunk().unwrap().unwrap();
        assert_eq!(c.offset, 6);
        assert_eq!(&c.data, b"6789");

        assert!(reader.next_chunk().unwrap().is_none());
    }

    #[test]
    fn chunk_reader_default_chunk_size() {
        let dir = TempDir::new().unwrap();
        let path = create_test_file(dir.path(), "test.bin", b"x");
        let reader = ChunkReader::new(&path, 0).unwrap();
        // Internal chunk_size should be DEFAULT_CHUNK_SIZE, but since file is 1 byte
        // it doesn't matter for this test — just verify it doesn't panic.
        assert_eq!(reader.file_size(), 1);
    }

    #[test]
    fn chunk_writer_writes_at_offset() {
        let dir = TempDir::new().unwrap();
        let mut writer = ChunkWriter::new(dir.path());

        let chunk1 = Chunk {
            offset: 0,
            size: 5,
            data: b"Hello".to_vec(),
            file_path: "out.bin".into(),
            checksum: String::new(),
        };
        writer.write_chunk(&chunk1).unwrap();
        assert_eq!(writer.get_written_offset("out.bin"), 5);

        let chunk2 = Chunk {
            offset: 5,
            size: 6,
            data: b" World".to_vec(),
            file_path: "out.bin".into(),
            checksum: String::new(),
        };
        writer.write_chunk(&chunk2).unwrap();
        assert_eq!(writer.get_written_offset("out.bin"), 11);

        let content = std::fs::read(dir.path().join("out.bin")).unwrap();
        assert_eq!(&content, b"Hello World");
    }

    #[test]
    fn chunk_writer_creates_subdirectories() {
        let dir = TempDir::new().unwrap();
        let mut writer = ChunkWriter::new(dir.path());

        let chunk = Chunk {
            offset: 0,
            size: 4,
            data: b"data".to_vec(),
            file_path: "sub/dir/file.txt".into(),
            checksum: String::new(),
        };
        writer.write_chunk(&chunk).unwrap();

        let content = std::fs::read(dir.path().join("sub/dir/file.txt")).unwrap();
        assert_eq!(&content, b"data");
    }

    #[test]
    fn chunk_writer_verifies_checksum() {
        let dir = TempDir::new().unwrap();
        let mut writer = ChunkWriter::new(dir.path());

        let data = b"test data";
        let good_checksum = checksum_bytes(data);

        // Good checksum: should succeed.
        let chunk_ok = Chunk {
            offset: 0,
            size: data.len(),
            data: data.to_vec(),
            file_path: "ok.bin".into(),
            checksum: good_checksum,
        };
        writer.write_chunk(&chunk_ok).unwrap();

        // Bad checksum: should fail.
        let chunk_bad = Chunk {
            offset: 0,
            size: data.len(),
            data: data.to_vec(),
            file_path: "bad.bin".into(),
            checksum: "0000000000000000000000000000000000000000000000000000000000000000".into(),
        };
        let result = writer.write_chunk(&chunk_bad);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TransferError::ChecksumMismatch
        ));
    }

    #[test]
    fn chunk_writer_untracked_file_returns_zero() {
        let dir = TempDir::new().unwrap();
        let writer = ChunkWriter::new(dir.path());
        assert_eq!(writer.get_written_offset("nonexistent"), 0);
    }

    #[test]
    fn path_traversal_parent_dir_rejected() {
        let dir = TempDir::new().unwrap();
        let mut writer = ChunkWriter::new(dir.path());
        let chunk = Chunk {
            offset: 0,
            size: 4,
            data: b"evil".to_vec(),
            file_path: "../../../etc/passwd".into(),
            checksum: String::new(),
        };
        let result = writer.write_chunk(&chunk);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TransferError::InvalidPath(_)));
    }

    #[test]
    fn path_traversal_nested_rejected() {
        let dir = TempDir::new().unwrap();
        let mut writer = ChunkWriter::new(dir.path());
        let chunk = Chunk {
            offset: 0,
            size: 4,
            data: b"evil".to_vec(),
            file_path: "sub/../../../escape".into(),
            checksum: String::new(),
        };
        let result = writer.write_chunk(&chunk);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TransferError::InvalidPath(_)));
    }

    #[test]
    fn path_traversal_absolute_rejected() {
        let dir = TempDir::new().unwrap();
        let mut writer = ChunkWriter::new(dir.path());
        let chunk = Chunk {
            offset: 0,
            size: 4,
            data: b"evil".to_vec(),
            file_path: "/tmp/malicious".into(),
            checksum: String::new(),
        };
        let result = writer.write_chunk(&chunk);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TransferError::InvalidPath(_)));
    }

    #[test]
    fn reader_writer_roundtrip() {
        let dir = TempDir::new().unwrap();
        let src_dir = dir.path().join("src");
        let dst_dir = dir.path().join("dst");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&dst_dir).unwrap();

        // Create source file.
        let original = b"The quick brown fox jumps over the lazy dog";
        let src_path = create_test_file(&src_dir, "test.txt", original);

        // Read all chunks and write them.
        let mut reader = ChunkReader::new(&src_path, 10).unwrap();
        let mut writer = ChunkWriter::new(&dst_dir);

        while let Some(chunk) = reader.next_chunk().unwrap() {
            let write_chunk = Chunk {
                file_path: "test.txt".into(),
                ..chunk
            };
            writer.write_chunk(&write_chunk).unwrap();
        }

        let result = std::fs::read(dst_dir.join("test.txt")).unwrap();
        assert_eq!(&result, original);
    }
}
