//! Bridge between `ConnectionManager` and the `AgentConnection` traits.
//!
//! Both `hub-deploy` and `hub-games` define their own `AgentConnection`
//! trait (same shape, different error types). This module provides a
//! single struct that implements both.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use capydeploy_hub_connection::ConnectionManager;
use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::envelope::Message;

/// Bridges `ConnectionManager` to the `AgentConnection` traits
/// expected by the deploy and games crates.
#[allow(dead_code)] // Used in later steps (deploy/games views).
pub struct ConnectionBridge {
    mgr: Arc<ConnectionManager>,
    agent_id: String,
}

impl ConnectionBridge {
    /// Creates a new bridge for the given connected agent.
    #[allow(dead_code)]
    pub fn new(mgr: Arc<ConnectionManager>, agent_id: String) -> Self {
        Self { mgr, agent_id }
    }
}

impl capydeploy_hub_deploy::AgentConnection for ConnectionBridge {
    fn send_request(
        &self,
        msg_type: MessageType,
        payload: &serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<Message, capydeploy_hub_deploy::DeployError>> + Send + '_>>
    {
        let payload = payload.clone();
        Box::pin(async move {
            self.mgr
                .send_request(msg_type, Some(&payload))
                .await
                .map_err(|e| capydeploy_hub_deploy::DeployError::Agent(e.to_string()))
        })
    }

    fn send_binary(
        &self,
        header: &serde_json::Value,
        data: &[u8],
    ) -> Pin<Box<dyn Future<Output = Result<Message, capydeploy_hub_deploy::DeployError>> + Send + '_>>
    {
        let header = header.clone();
        let data = data.to_vec();
        Box::pin(async move {
            self.mgr
                .send_binary(&header, &data)
                .await
                .map_err(|e| capydeploy_hub_deploy::DeployError::Agent(e.to_string()))
        })
    }

    fn agent_id(&self) -> &str {
        &self.agent_id
    }
}

impl capydeploy_hub_games::AgentConnection for ConnectionBridge {
    fn send_request(
        &self,
        msg_type: MessageType,
        payload: &serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<Message, capydeploy_hub_games::GamesError>> + Send + '_>>
    {
        let payload = payload.clone();
        Box::pin(async move {
            self.mgr
                .send_request(msg_type, Some(&payload))
                .await
                .map_err(|e| capydeploy_hub_games::GamesError::Agent(e.to_string()))
        })
    }

    fn send_binary(
        &self,
        header: &serde_json::Value,
        data: &[u8],
    ) -> Pin<Box<dyn Future<Output = Result<Message, capydeploy_hub_games::GamesError>> + Send + '_>>
    {
        let header = header.clone();
        let data = data.to_vec();
        Box::pin(async move {
            self.mgr
                .send_binary(&header, &data)
                .await
                .map_err(|e| capydeploy_hub_games::GamesError::Agent(e.to_string()))
        })
    }

    fn agent_id(&self) -> &str {
        &self.agent_id
    }
}
