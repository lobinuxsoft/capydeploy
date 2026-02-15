//! Deploy orchestrator for multi-agent parallel deployment.
//!
//! Coordinates deployment to one or more agents, aggregates progress
//! events, and supports cancellation.

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::agent::{AgentConnection, AgentDeploy};
use crate::types::{DeployConfig, DeployEvent, DeployResult};

/// Orchestrates game deployment to multiple agents.
pub struct DeployOrchestrator {
    events_tx: mpsc::Sender<DeployEvent>,
    events_rx: Option<mpsc::Receiver<DeployEvent>>,
    cancel: CancellationToken,
}

impl Default for DeployOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl DeployOrchestrator {
    /// Creates a new orchestrator.
    pub fn new() -> Self {
        let (events_tx, events_rx) = mpsc::channel(256);
        Self {
            events_tx,
            events_rx: Some(events_rx),
            cancel: CancellationToken::new(),
        }
    }

    /// Takes the event receiver. Can only be called once.
    pub fn take_events(&mut self) -> Option<mpsc::Receiver<DeployEvent>> {
        self.events_rx.take()
    }

    /// Returns a cancellation token for this deployment.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    /// Deploys a game to one or more agents in parallel.
    ///
    /// Each agent runs independently. If one fails, the others continue.
    /// Returns a result per agent.
    pub async fn deploy(
        &self,
        config: DeployConfig,
        connections: Vec<&dyn AgentConnection>,
    ) -> Vec<DeployResult> {
        if connections.is_empty() {
            return Vec::new();
        }

        // Single agent — run directly without spawning.
        if connections.len() == 1 {
            let conn = connections[0];
            let result = self.deploy_single(conn, &config).await;
            return vec![result];
        }

        // Multiple agents — run in parallel.
        // Note: we can't easily spawn tasks holding &dyn references,
        // so we run them sequentially for now. The Hub UI will likely
        // deploy to one agent at a time anyway.
        let mut results = Vec::with_capacity(connections.len());
        for conn in connections {
            let result = self.deploy_single(conn, &config).await;
            results.push(result);
        }
        results
    }

    async fn deploy_single(
        &self,
        conn: &dyn AgentConnection,
        config: &DeployConfig,
    ) -> DeployResult {
        let agent_id = conn.agent_id().to_string();
        let deployer = AgentDeploy::new(conn, self.cancel.clone());

        match deployer.deploy(config, &self.events_tx).await {
            Ok(result) => {
                let _ = self
                    .events_tx
                    .send(DeployEvent::Completed {
                        agent_id: agent_id.clone(),
                    })
                    .await;
                info!(agent = %agent_id, app_id = result.app_id, "deploy completed");

                DeployResult {
                    agent_id,
                    success: true,
                    error: None,
                    app_id: Some(result.app_id),
                }
            }
            Err(e) => {
                let err_msg = e.to_string();
                let _ = self
                    .events_tx
                    .send(DeployEvent::Failed {
                        agent_id: agent_id.clone(),
                        error: err_msg.clone(),
                    })
                    .await;
                error!(agent = %agent_id, error = %err_msg, "deploy failed");

                DeployResult {
                    agent_id,
                    success: false,
                    error: Some(err_msg),
                    app_id: None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentConnection;
    use crate::types::{ArtworkAssignment, GameSetup};
    use capydeploy_protocol::constants::MessageType;
    use capydeploy_protocol::envelope::Message;
    use capydeploy_protocol::messages::{
        CompleteUploadResponseFull, InitUploadResponseFull,
    };
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Mutex;

    struct MockConn {
        id: String,
        responses: Mutex<Vec<Message>>,
    }

    impl MockConn {
        fn new(id: &str, responses: Vec<Message>) -> Self {
            Self {
                id: id.into(),
                responses: Mutex::new(responses),
            }
        }
    }

    impl AgentConnection for MockConn {
        fn send_request(
            &self,
            _msg_type: MessageType,
            _payload: &serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = Result<Message, crate::error::DeployError>> + Send + '_>>
        {
            Box::pin(async move {
                let mut resps = self.responses.lock().unwrap();
                if resps.is_empty() {
                    Err(crate::error::DeployError::Agent("no response".into()))
                } else {
                    Ok(resps.remove(0))
                }
            })
        }

        fn send_binary(
            &self,
            _header: &serde_json::Value,
            _data: &[u8],
        ) -> Pin<Box<dyn Future<Output = Result<Message, crate::error::DeployError>> + Send + '_>>
        {
            Box::pin(async move {
                Ok(Message::new::<()>("b", MessageType::Pong, None).unwrap())
            })
        }

        fn agent_id(&self) -> &str {
            &self.id
        }
    }

    fn mock_responses() -> Vec<Message> {
        let init = InitUploadResponseFull {
            upload_id: "u1".into(),
            chunk_size: 1024 * 1024,
            resume_from: None,
        };
        let complete = CompleteUploadResponseFull {
            success: true,
            path: "/games/test".into(),
            app_id: 42,
        };

        vec![
            Message::new("1", MessageType::UploadInitResponse, Some(&init)).unwrap(),
            // complete
            Message::new("2", MessageType::OperationResult, Some(&complete)).unwrap(),
        ]
    }

    fn test_config(dir: &std::path::Path) -> DeployConfig {
        DeployConfig {
            setup: GameSetup {
                id: "g1".into(),
                name: "Test".into(),
                local_path: dir.to_string_lossy().into_owned(),
                executable: "game.exe".into(),
                launch_options: String::new(),
                tags: String::new(),
                install_path: "/games/test".into(),
                griddb_game_id: 0,
                grid_portrait: String::new(),
                grid_landscape: String::new(),
                hero_image: String::new(),
                logo_image: String::new(),
                icon_image: String::new(),
            },
            artwork: ArtworkAssignment::default(),
        }
    }

    #[tokio::test]
    async fn orchestrator_single_agent() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("game.exe"), b"X").unwrap();

        let conn = MockConn::new("a1", mock_responses());
        let mut orch = DeployOrchestrator::new();
        let mut events_rx = orch.take_events().unwrap();

        let config = test_config(dir.path());
        let results = orch.deploy(config, vec![&conn]).await;

        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert_eq!(results[0].agent_id, "a1");
        assert_eq!(results[0].app_id, Some(42));

        // Collect events.
        drop(orch);
        let mut events = Vec::new();
        while let Some(e) = events_rx.recv().await {
            events.push(e);
        }
        // Should have progress + completed.
        assert!(events
            .iter()
            .any(|e| matches!(e, DeployEvent::Completed { .. })));
    }

    #[tokio::test]
    async fn orchestrator_empty_connections() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("game.exe"), b"X").unwrap();

        let orch = DeployOrchestrator::new();
        let config = test_config(dir.path());
        let results = orch.deploy(config, vec![]).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn orchestrator_agent_failure() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("game.exe"), b"X").unwrap();

        // Empty responses → will fail immediately.
        let conn = MockConn::new("a1", vec![]);
        let orch = DeployOrchestrator::new();
        let config = test_config(dir.path());
        let results = orch.deploy(config, vec![&conn]).await;

        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
        assert!(results[0].error.is_some());
    }

    #[tokio::test]
    async fn orchestrator_multi_agent() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("game.exe"), b"X").unwrap();

        let conn1 = MockConn::new("a1", mock_responses());
        let conn2 = MockConn::new("a2", mock_responses());

        let orch = DeployOrchestrator::new();
        let config = test_config(dir.path());
        let results = orch.deploy(config, vec![&conn1, &conn2]).await;

        assert_eq!(results.len(), 2);
        assert!(results[0].success);
        assert!(results[1].success);
        assert_eq!(results[0].agent_id, "a1");
        assert_eq!(results[1].agent_id, "a2");
    }

    #[tokio::test]
    async fn orchestrator_cancellation() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("game.exe"), b"X").unwrap();

        let conn = MockConn::new("a1", mock_responses());
        let orch = DeployOrchestrator::new();
        let cancel = orch.cancel_token();
        cancel.cancel();

        let config = test_config(dir.path());
        let results = orch.deploy(config, vec![&conn]).await;

        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
        assert!(results[0].error.as_deref().unwrap().contains("cancelled"));
    }

    #[tokio::test]
    async fn orchestrator_progress_monotonic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("game.exe"), b"X").unwrap();

        let conn = MockConn::new("a1", mock_responses());
        let mut orch = DeployOrchestrator::new();
        let mut events_rx = orch.take_events().unwrap();

        let config = test_config(dir.path());
        let _results = orch.deploy(config, vec![&conn]).await;
        drop(orch);

        let mut last_progress = -1.0f64;
        while let Some(e) = events_rx.recv().await {
            if let DeployEvent::Progress { progress, .. } = e {
                assert!(
                    progress >= last_progress,
                    "progress should be monotonic: {last_progress} -> {progress}"
                );
                last_progress = progress;
            }
        }
    }

    #[tokio::test]
    async fn take_events_once() {
        let mut orch = DeployOrchestrator::new();
        assert!(orch.take_events().is_some());
        assert!(orch.take_events().is_none());
    }
}
