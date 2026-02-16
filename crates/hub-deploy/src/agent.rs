//! Agent connection trait and per-agent deploy session.
//!
//! `AgentConnection` is implemented by the Hub app to bridge
//! deploy logic to the actual WebSocket transport.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;

use capydeploy_protocol::envelope::Message;
use capydeploy_protocol::messages::{
    CompleteUploadRequestFull, CompleteUploadResponseFull, FileEntry, InitUploadRequestFull,
    InitUploadResponseFull,
};
use capydeploy_protocol::types::{ShortcutConfig, UploadConfig};
use capydeploy_transfer::ChunkReader;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use crate::artwork_selector::{build_shortcut_config, collect_local_artwork};
use crate::error::DeployError;
use crate::types::{
    CompleteUploadResult, DeployConfig, DeployEvent, GameSetup, InitUploadResult, LocalArtwork,
};

/// Abstract connection to an Agent.
///
/// The Hub app implements this trait on top of `WsClient`/`ConnectionManager`.
/// Using a trait keeps deploy logic decoupled from transport and testable with mocks.
pub trait AgentConnection: Send + Sync {
    /// Sends a JSON request and waits for the response.
    fn send_request(
        &self,
        msg_type: capydeploy_protocol::constants::MessageType,
        payload: &serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<Message, DeployError>> + Send + '_>>;

    /// Sends binary data with a JSON header to the agent.
    fn send_binary(
        &self,
        header: &serde_json::Value,
        data: &[u8],
    ) -> Pin<Box<dyn Future<Output = Result<Message, DeployError>> + Send + '_>>;

    /// Returns the agent's unique identifier.
    fn agent_id(&self) -> &str;
}

/// Manages a deploy session to a single agent.
pub struct AgentDeploy<'a> {
    conn: &'a dyn AgentConnection,
    cancel: CancellationToken,
}

impl<'a> AgentDeploy<'a> {
    /// Creates a new deploy session.
    pub fn new(conn: &'a dyn AgentConnection, cancel: CancellationToken) -> Self {
        Self { conn, cancel }
    }

    /// Runs the full deploy pipeline for one agent.
    ///
    /// Progress events are sent via `events_tx`. The pipeline:
    /// 1. Scan files (0.0–0.05)
    /// 2. Init upload (0.05–0.1)
    /// 3. Upload chunks (0.1–0.85)
    /// 4. Send local artwork (0.85–0.9)
    /// 5. Complete upload + create shortcut (0.9–1.0)
    pub async fn deploy(
        &self,
        config: &DeployConfig,
        events_tx: &tokio::sync::mpsc::Sender<DeployEvent>,
    ) -> Result<CompleteUploadResult, DeployError> {
        let agent_id = self.conn.agent_id().to_string();

        // 1. Scan files
        self.emit_progress(events_tx, 0.0, "Scanning files...")
            .await;
        self.check_cancelled()?;

        let root_path = Path::new(&config.setup.local_path);
        let (files, total_size) = crate::scanner::scan_files_for_upload(root_path)?;

        debug!(
            agent = %agent_id,
            files = files.len(),
            total_bytes = total_size,
            "scan complete"
        );

        // 2. Init upload
        self.emit_progress(events_tx, 0.05, "Initializing upload...")
            .await;
        self.check_cancelled()?;

        let init_result = self.init_upload(&config.setup, &files, total_size).await?;

        let chunk_size = if init_result.chunk_size > 0 {
            init_result.chunk_size as usize
        } else {
            capydeploy_transfer::DEFAULT_CHUNK_SIZE
        };

        // 3. Upload chunks
        self.emit_progress(events_tx, 0.1, "Uploading files...")
            .await;

        self.upload_files(
            &config.setup,
            &files,
            total_size,
            &init_result,
            chunk_size,
            events_tx,
        )
        .await?;

        // 4. Send local artwork
        self.emit_progress(events_tx, 0.85, "Sending artwork...")
            .await;
        self.check_cancelled()?;

        let local_artwork = collect_local_artwork(&config.artwork);
        self.send_artwork(&local_artwork, 0, events_tx).await;

        // 5. Complete upload
        self.emit_progress(events_tx, 0.9, "Creating shortcut...")
            .await;
        self.check_cancelled()?;

        let shortcut = build_shortcut_config(&config.setup, &config.artwork);
        let result = self
            .complete_upload(&init_result.upload_id, &shortcut)
            .await?;

        self.emit_progress(events_tx, 1.0, "Upload complete!").await;

        Ok(result)
    }

    /// Initializes the upload session on the agent.
    async fn init_upload(
        &self,
        setup: &GameSetup,
        files: &[FileEntry],
        total_size: i64,
    ) -> Result<InitUploadResult, DeployError> {
        let upload_config = UploadConfig {
            game_name: setup.name.clone(),
            install_path: setup.install_path.clone(),
            executable: setup.executable.clone(),
            launch_options: setup.launch_options.clone(),
            tags: setup.tags.clone(),
        };

        let req = InitUploadRequestFull {
            config: upload_config,
            total_size,
            files: files.to_vec(),
        };

        let payload = serde_json::to_value(&req)?;
        let resp = self
            .conn
            .send_request(
                capydeploy_protocol::constants::MessageType::InitUpload,
                &payload,
            )
            .await?;

        let init_resp: InitUploadResponseFull = resp
            .parse_payload::<InitUploadResponseFull>()?
            .ok_or_else(|| DeployError::Upload("empty init response".into()))?;

        Ok(InitUploadResult {
            upload_id: init_resp.upload_id,
            chunk_size: init_resp.chunk_size,
            resume_from: init_resp.resume_from,
        })
    }

    /// Uploads all files in chunks with resume support.
    async fn upload_files(
        &self,
        setup: &GameSetup,
        files: &[FileEntry],
        total_size: i64,
        init_result: &InitUploadResult,
        chunk_size: usize,
        events_tx: &tokio::sync::mpsc::Sender<DeployEvent>,
    ) -> Result<(), DeployError> {
        let mut uploaded: i64 = 0;

        for file_entry in files {
            self.check_cancelled()?;

            let local_path = Path::new(&setup.local_path).join(&file_entry.relative_path);
            let mut reader = tokio::task::spawn_blocking({
                let path = local_path.clone();
                move || ChunkReader::new(&path, chunk_size)
            })
            .await
            .map_err(|e| DeployError::Upload(format!("task join error: {e}")))??;

            // Handle resume offset.
            if let Some(ref resume_map) = init_result.resume_from
                && let Some(&offset) = resume_map.get(&file_entry.relative_path)
                && offset > 0
            {
                reader.seek_to(offset)?;
                uploaded += offset;
            }

            // Read and send chunks.
            loop {
                self.check_cancelled()?;

                let chunk = tokio::task::spawn_blocking({
                    let mut r = unsafe_reader_send_wrapper(reader);
                    move || {
                        let chunk = r.0.next_chunk();
                        (r.0, chunk)
                    }
                })
                .await
                .map_err(|e| DeployError::Upload(format!("task join error: {e}")))?;

                reader = chunk.0;
                let chunk_result = chunk.1?;

                let Some(chunk_data) = chunk_result else {
                    break;
                };

                // Send chunk as a single binary message (matching Go Hub behavior).
                // The agent routes binary messages without "type" to the upload
                // chunk handler, which matches by uploadId + filePath.
                let header = serde_json::json!({
                    "uploadId": init_result.upload_id,
                    "filePath": file_entry.relative_path,
                    "offset": chunk_data.offset,
                    "checksum": chunk_data.checksum,
                });
                let _resp = self.conn.send_binary(&header, &chunk_data.data).await?;

                uploaded += chunk_data.size as i64;

                // Progress: 0.1 to 0.85.
                if total_size > 0 {
                    let progress = 0.1 + (uploaded as f64 / total_size as f64) * 0.75;
                    let status = format!("Uploading: {}", file_entry.relative_path);
                    self.emit_progress(events_tx, progress, &status).await;
                }
            }
        }

        Ok(())
    }

    /// Sends local artwork images to the agent.
    async fn send_artwork(
        &self,
        artwork: &[LocalArtwork],
        app_id: u32,
        events_tx: &tokio::sync::mpsc::Sender<DeployEvent>,
    ) {
        for art in artwork {
            let header = serde_json::json!({
                "type": "artwork_image",
                "appId": app_id,
                "artworkType": art.art_type,
                "contentType": art.content_type,
            });

            self.emit_progress(
                events_tx,
                0.87,
                &format!("Sending {} artwork...", art.art_type),
            )
            .await;

            match self.conn.send_binary(&header, &art.data).await {
                Ok(_) => {
                    debug!(art_type = %art.art_type, "sent local artwork");
                }
                Err(e) => {
                    warn!(art_type = %art.art_type, error = %e, "failed to send artwork");
                }
            }
        }
    }

    /// Completes the upload and creates a shortcut.
    async fn complete_upload(
        &self,
        upload_id: &str,
        shortcut: &ShortcutConfig,
    ) -> Result<CompleteUploadResult, DeployError> {
        let req = CompleteUploadRequestFull {
            upload_id: upload_id.to_string(),
            create_shortcut: true,
            shortcut: Some(shortcut.clone()),
        };

        let payload = serde_json::to_value(&req)?;
        let resp = self
            .conn
            .send_request(
                capydeploy_protocol::constants::MessageType::CompleteUpload,
                &payload,
            )
            .await?;

        let complete_resp: CompleteUploadResponseFull = resp
            .parse_payload::<CompleteUploadResponseFull>()?
            .ok_or_else(|| DeployError::Upload("empty complete response".into()))?;

        if !complete_resp.success {
            return Err(DeployError::Upload("upload completion failed".into()));
        }

        Ok(CompleteUploadResult {
            success: complete_resp.success,
            path: complete_resp.path,
            app_id: complete_resp.app_id,
        })
    }

    fn check_cancelled(&self) -> Result<(), DeployError> {
        if self.cancel.is_cancelled() {
            Err(DeployError::Cancelled)
        } else {
            Ok(())
        }
    }

    async fn emit_progress(
        &self,
        events_tx: &tokio::sync::mpsc::Sender<DeployEvent>,
        progress: f64,
        status: &str,
    ) {
        let _ = events_tx
            .send(DeployEvent::Progress {
                agent_id: self.conn.agent_id().to_string(),
                progress,
                status: status.to_string(),
            })
            .await;
    }
}

/// Wrapper to make ChunkReader Send-able across spawn_blocking boundaries.
///
/// ChunkReader is inherently single-threaded (holds a File handle)
/// but is used exclusively within spawn_blocking closures.
struct SendWrapper(ChunkReader);
unsafe impl Send for SendWrapper {}

fn unsafe_reader_send_wrapper(reader: ChunkReader) -> SendWrapper {
    SendWrapper(reader)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ArtworkAssignment;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tokio::sync::mpsc;

    /// Mock agent connection that records requests.
    struct MockAgent {
        id: String,
        responses: Mutex<Vec<Message>>,
        requests: Mutex<Vec<(String, serde_json::Value)>>,
        binary_sends: Mutex<Vec<(serde_json::Value, Vec<u8>)>>,
    }

    impl MockAgent {
        fn new(id: &str) -> Self {
            Self {
                id: id.to_string(),
                responses: Mutex::new(Vec::new()),
                requests: Mutex::new(Vec::new()),
                binary_sends: Mutex::new(Vec::new()),
            }
        }

        fn push_response(&self, msg: Message) {
            self.responses.lock().unwrap().push(msg);
        }

        fn request_count(&self) -> usize {
            self.requests.lock().unwrap().len()
        }

        fn binary_count(&self) -> usize {
            self.binary_sends.lock().unwrap().len()
        }
    }

    impl AgentConnection for MockAgent {
        fn send_request(
            &self,
            msg_type: capydeploy_protocol::constants::MessageType,
            payload: &serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = Result<Message, DeployError>> + Send + '_>> {
            let msg_type_str = format!("{msg_type:?}");
            self.requests
                .lock()
                .unwrap()
                .push((msg_type_str, payload.clone()));

            Box::pin(async move {
                let mut responses = self.responses.lock().unwrap();
                if responses.is_empty() {
                    Err(DeployError::Agent("no mock response available".into()))
                } else {
                    Ok(responses.remove(0))
                }
            })
        }

        fn send_binary(
            &self,
            header: &serde_json::Value,
            data: &[u8],
        ) -> Pin<Box<dyn Future<Output = Result<Message, DeployError>> + Send + '_>> {
            self.binary_sends
                .lock()
                .unwrap()
                .push((header.clone(), data.to_vec()));

            Box::pin(async move {
                let msg = Message::new::<()>(
                    "bin-resp",
                    capydeploy_protocol::constants::MessageType::Pong,
                    None,
                )
                .unwrap();
                Ok(msg)
            })
        }

        fn agent_id(&self) -> &str {
            &self.id
        }
    }

    fn make_init_response(upload_id: &str) -> Message {
        let resp = InitUploadResponseFull {
            upload_id: upload_id.into(),
            chunk_size: 1024 * 1024,
            resume_from: None,
        };
        Message::new(
            "init-resp",
            capydeploy_protocol::constants::MessageType::UploadInitResponse,
            Some(&resp),
        )
        .unwrap()
    }

    fn make_complete_response(success: bool) -> Message {
        let resp = CompleteUploadResponseFull {
            success,
            path: "/home/deck/Games/test".into(),
            app_id: 12345,
        };
        Message::new(
            "complete-resp",
            capydeploy_protocol::constants::MessageType::OperationResult,
            Some(&resp),
        )
        .unwrap()
    }

    fn test_setup(dir: &Path) -> GameSetup {
        GameSetup {
            id: "g1".into(),
            name: "Test Game".into(),
            local_path: dir.to_string_lossy().into_owned(),
            executable: "game.exe".into(),
            launch_options: String::new(),
            tags: String::new(),
            install_path: "/home/deck/Games/test".into(),
            griddb_game_id: 0,
            grid_portrait: String::new(),
            grid_landscape: String::new(),
            hero_image: String::new(),
            logo_image: String::new(),
            icon_image: String::new(),
        }
    }

    #[tokio::test]
    async fn deploy_full_pipeline() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("game.exe"), b"EXE").unwrap();

        let mock = MockAgent::new("agent-1");
        // Init response.
        mock.push_response(make_init_response("upload-1"));
        // Complete response.
        mock.push_response(make_complete_response(true));

        let cancel = CancellationToken::new();
        let deployer = AgentDeploy::new(&mock, cancel);
        let config = DeployConfig {
            setup: test_setup(dir.path()),
            artwork: ArtworkAssignment::default(),
        };

        let (events_tx, mut events_rx) = mpsc::channel(64);
        let result = deployer.deploy(&config, &events_tx).await.unwrap();

        assert!(result.success);
        assert_eq!(result.app_id, 12345);
        assert_eq!(mock.request_count(), 2); // init + complete
        assert_eq!(mock.binary_count(), 1); // chunk binary data

        // Verify we got progress events.
        drop(events_tx);
        let mut events = Vec::new();
        while let Some(e) = events_rx.recv().await {
            events.push(e);
        }
        assert!(!events.is_empty());
    }

    #[tokio::test]
    async fn deploy_cancelled_early() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("game.exe"), b"EXE").unwrap();

        let mock = MockAgent::new("agent-1");
        let cancel = CancellationToken::new();
        cancel.cancel(); // Cancel before starting.

        let deployer = AgentDeploy::new(&mock, cancel);
        let config = DeployConfig {
            setup: test_setup(dir.path()),
            artwork: ArtworkAssignment::default(),
        };

        let (events_tx, _events_rx) = mpsc::channel(64);
        let result = deployer.deploy(&config, &events_tx).await;
        assert!(matches!(result, Err(DeployError::Cancelled)));
    }

    #[tokio::test]
    async fn deploy_upload_failure() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("game.exe"), b"EXE").unwrap();

        let mock = MockAgent::new("agent-1");
        // Init response.
        mock.push_response(make_init_response("upload-1"));
        // Complete response with failure.
        mock.push_response(make_complete_response(false));

        let cancel = CancellationToken::new();
        let deployer = AgentDeploy::new(&mock, cancel);
        let config = DeployConfig {
            setup: test_setup(dir.path()),
            artwork: ArtworkAssignment::default(),
        };

        let (events_tx, _) = mpsc::channel(64);
        let result = deployer.deploy(&config, &events_tx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn deploy_with_resume() {
        let dir = tempfile::tempdir().unwrap();
        // 10 bytes file.
        std::fs::write(dir.path().join("game.exe"), b"0123456789").unwrap();

        let mock = MockAgent::new("agent-1");

        // Init response with resume point at 5 bytes.
        let mut resume = HashMap::new();
        resume.insert("game.exe".to_string(), 5i64);
        let resp = InitUploadResponseFull {
            upload_id: "upload-resume".into(),
            chunk_size: 1024 * 1024,
            resume_from: Some(resume),
        };
        let init_msg = Message::new(
            "init-resp",
            capydeploy_protocol::constants::MessageType::UploadInitResponse,
            Some(&resp),
        )
        .unwrap();
        mock.push_response(init_msg);

        // Complete.
        mock.push_response(make_complete_response(true));

        let cancel = CancellationToken::new();
        let deployer = AgentDeploy::new(&mock, cancel);
        let config = DeployConfig {
            setup: test_setup(dir.path()),
            artwork: ArtworkAssignment::default(),
        };

        let (events_tx, _) = mpsc::channel(64);
        let result = deployer.deploy(&config, &events_tx).await.unwrap();
        assert!(result.success);

        // Should have sent 1 chunk binary with only 5 bytes.
        let binaries = mock.binary_sends.lock().unwrap();
        assert_eq!(binaries.len(), 1);
        assert_eq!(binaries[0].1.len(), 5);
    }
}
