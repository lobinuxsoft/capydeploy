use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Instant;

use capydeploy_protocol::messages::FileEntry;
use capydeploy_protocol::types::{UploadConfig, UploadProgress, UploadStatus};

/// A chunk of file data for transfer.
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Byte offset within the file.
    pub offset: i64,
    /// Size of this chunk in bytes.
    pub size: usize,
    /// Raw chunk data.
    pub data: Vec<u8>,
    /// Relative file path within the upload.
    pub file_path: String,
    /// SHA-256 hex checksum of `data` (empty means no verification).
    pub checksum: String,
}

/// Tracks an active upload session (thread-safe).
pub struct UploadSession {
    inner: RwLock<SessionInner>,
}

struct SessionInner {
    id: String,
    config: UploadConfig,
    status: UploadStatus,
    total_bytes: i64,
    transferred_bytes: i64,
    files: Vec<FileEntry>,
    current_file_index: usize,
    started_at: Option<Instant>,
    updated_at: Instant,
    completed_at: Option<Instant>,
    error: String,
    /// Maps file_path -> last confirmed byte offset (for resume).
    chunk_offsets: HashMap<String, i64>,
}

impl UploadSession {
    /// Creates a new pending upload session.
    pub fn new(id: String, config: UploadConfig, total_bytes: i64, files: Vec<FileEntry>) -> Self {
        Self {
            inner: RwLock::new(SessionInner {
                id,
                config,
                status: UploadStatus::Pending,
                total_bytes,
                transferred_bytes: 0,
                files,
                current_file_index: 0,
                started_at: None,
                updated_at: Instant::now(),
                completed_at: None,
                error: String::new(),
                chunk_offsets: HashMap::new(),
            }),
        }
    }

    /// Marks the session as in-progress.
    pub fn start(&self) {
        let mut s = self.inner.write().unwrap();
        s.status = UploadStatus::InProgress;
        let now = Instant::now();
        s.started_at = Some(now);
        s.updated_at = now;
    }

    /// Records progress: adds transferred bytes and updates the chunk offset.
    pub fn add_progress(&self, bytes: i64, file_path: &str, offset: i64) {
        let mut s = self.inner.write().unwrap();
        s.transferred_bytes += bytes;
        s.chunk_offsets
            .insert(file_path.to_string(), offset + bytes);
        s.updated_at = Instant::now();
    }

    /// Marks the session as completed.
    pub fn complete(&self) {
        let mut s = self.inner.write().unwrap();
        s.status = UploadStatus::Completed;
        let now = Instant::now();
        s.completed_at = Some(now);
        s.updated_at = now;
    }

    /// Marks the session as failed with an error message.
    pub fn fail(&self, err: &str) {
        let mut s = self.inner.write().unwrap();
        s.status = UploadStatus::Failed;
        s.error = err.to_string();
        let now = Instant::now();
        s.completed_at = Some(now);
        s.updated_at = now;
    }

    /// Marks the session as cancelled.
    pub fn cancel(&self) {
        let mut s = self.inner.write().unwrap();
        s.status = UploadStatus::Cancelled;
        let now = Instant::now();
        s.completed_at = Some(now);
        s.updated_at = now;
    }

    /// Returns current progress as a protocol-compatible struct.
    pub fn progress(&self) -> UploadProgress {
        let s = self.inner.read().unwrap();
        let current_file = if s.current_file_index < s.files.len() {
            s.files[s.current_file_index].relative_path.clone()
        } else {
            String::new()
        };
        UploadProgress {
            upload_id: s.id.clone(),
            status: s.status.clone(),
            total_bytes: s.total_bytes,
            transferred_bytes: s.transferred_bytes,
            current_file,
            started_at: String::new(),
            updated_at: String::new(),
            error: s.error.clone(),
        }
    }

    /// Returns the last confirmed byte offset for a given file (0 if never written).
    pub fn get_resume_offset(&self, file_path: &str) -> i64 {
        let s = self.inner.read().unwrap();
        s.chunk_offsets.get(file_path).copied().unwrap_or(0)
    }

    /// Returns all resume offsets (file_path -> byte offset).
    pub fn get_all_resume_offsets(&self) -> HashMap<String, i64> {
        let s = self.inner.read().unwrap();
        s.chunk_offsets.clone()
    }

    /// Returns `true` if the session is pending or in-progress.
    pub fn is_active(&self) -> bool {
        let s = self.inner.read().unwrap();
        matches!(s.status, UploadStatus::Pending | UploadStatus::InProgress)
    }

    /// Returns the session ID.
    pub fn id(&self) -> String {
        let s = self.inner.read().unwrap();
        s.id.clone()
    }

    /// Returns the upload configuration.
    pub fn config(&self) -> UploadConfig {
        let s = self.inner.read().unwrap();
        s.config.clone()
    }

    /// Returns the current status.
    pub fn status(&self) -> UploadStatus {
        let s = self.inner.read().unwrap();
        s.status.clone()
    }

    /// Returns total bytes to transfer.
    pub fn total_bytes(&self) -> i64 {
        let s = self.inner.read().unwrap();
        s.total_bytes
    }

    /// Returns bytes transferred so far.
    pub fn transferred_bytes(&self) -> i64 {
        let s = self.inner.read().unwrap();
        s.transferred_bytes
    }

    /// Sets the current file index being transferred.
    pub fn set_current_file_index(&self, index: usize) {
        let mut s = self.inner.write().unwrap();
        s.current_file_index = index;
        s.updated_at = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> UploadConfig {
        UploadConfig {
            game_name: "TestGame".into(),
            install_path: "/games/test".into(),
            executable: "game.exe".into(),
            launch_options: String::new(),
            tags: String::new(),
        }
    }

    fn sample_files() -> Vec<FileEntry> {
        vec![
            FileEntry {
                relative_path: "game.exe".into(),
                size: 1024,
            },
            FileEntry {
                relative_path: "data/level1.dat".into(),
                size: 2048,
            },
        ]
    }

    #[test]
    fn new_session_is_pending() {
        let session = UploadSession::new("s1".into(), sample_config(), 3072, sample_files());
        assert_eq!(session.status(), UploadStatus::Pending);
        assert!(session.is_active());
        assert_eq!(session.transferred_bytes(), 0);
    }

    #[test]
    fn start_sets_in_progress() {
        let session = UploadSession::new("s1".into(), sample_config(), 3072, sample_files());
        session.start();
        assert_eq!(session.status(), UploadStatus::InProgress);
        assert!(session.is_active());
    }

    #[test]
    fn add_progress_updates_bytes_and_offsets() {
        let session = UploadSession::new("s1".into(), sample_config(), 3072, sample_files());
        session.start();
        session.add_progress(512, "game.exe", 0);
        assert_eq!(session.transferred_bytes(), 512);
        assert_eq!(session.get_resume_offset("game.exe"), 512);

        session.add_progress(512, "game.exe", 512);
        assert_eq!(session.transferred_bytes(), 1024);
        assert_eq!(session.get_resume_offset("game.exe"), 1024);
    }

    #[test]
    fn complete_marks_completed() {
        let session = UploadSession::new("s1".into(), sample_config(), 1024, sample_files());
        session.start();
        session.add_progress(1024, "game.exe", 0);
        session.complete();
        assert_eq!(session.status(), UploadStatus::Completed);
        assert!(!session.is_active());
    }

    #[test]
    fn fail_records_error() {
        let session = UploadSession::new("s1".into(), sample_config(), 1024, sample_files());
        session.start();
        session.fail("disk full");
        assert_eq!(session.status(), UploadStatus::Failed);
        assert!(!session.is_active());
        let p = session.progress();
        assert_eq!(p.error, "disk full");
    }

    #[test]
    fn cancel_marks_cancelled() {
        let session = UploadSession::new("s1".into(), sample_config(), 1024, sample_files());
        session.start();
        session.cancel();
        assert_eq!(session.status(), UploadStatus::Cancelled);
        assert!(!session.is_active());
    }

    #[test]
    fn progress_returns_current_file() {
        let session = UploadSession::new("s1".into(), sample_config(), 3072, sample_files());
        session.start();
        let p = session.progress();
        assert_eq!(p.current_file, "game.exe");

        session.set_current_file_index(1);
        let p = session.progress();
        assert_eq!(p.current_file, "data/level1.dat");
    }

    #[test]
    fn resume_offset_defaults_to_zero() {
        let session = UploadSession::new("s1".into(), sample_config(), 1024, sample_files());
        assert_eq!(session.get_resume_offset("nonexistent.bin"), 0);
    }

    #[test]
    fn concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let session = Arc::new(UploadSession::new(
            "s1".into(),
            sample_config(),
            100_000,
            sample_files(),
        ));
        session.start();

        let mut handles = vec![];

        // 10 writers adding progress.
        for i in 0..10 {
            let s = Arc::clone(&session);
            handles.push(thread::spawn(move || {
                for j in 0..100 {
                    s.add_progress(1, &format!("file_{i}"), j);
                }
            }));
        }

        // 10 readers checking progress.
        for _ in 0..10 {
            let s = Arc::clone(&session);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let _ = s.progress();
                    let _ = s.is_active();
                    let _ = s.transferred_bytes();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // 10 writers × 100 iterations × 1 byte each = 1000.
        assert_eq!(session.transferred_bytes(), 1000);
    }
}
