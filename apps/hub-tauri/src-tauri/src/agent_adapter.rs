//! Adapters bridging `ConnectionManager` to the `AgentConnection` traits
//! required by `hub-deploy` and `hub-games` crates.
//!
//! Each crate defines its own trait with a different error type, so we
//! provide two thin wrappers around the same `Arc<ConnectionManager>`.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use capydeploy_hub_connection::ConnectionManager;
use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::envelope::Message;

// ---------------------------------------------------------------------------
// Deploy adapter
// ---------------------------------------------------------------------------

/// Implements `hub_deploy::AgentConnection` by delegating to `ConnectionManager`.
pub struct DeployAdapter {
    mgr: Arc<ConnectionManager>,
    agent_id: String,
    agent_ip: Option<std::net::IpAddr>,
}

impl DeployAdapter {
    #[allow(dead_code)]
    pub fn new(mgr: Arc<ConnectionManager>, agent_id: String) -> Self {
        Self {
            mgr,
            agent_id,
            agent_ip: None,
        }
    }

    /// Creates a DeployAdapter with cached agent IP address.
    pub fn with_agent_info(
        mgr: Arc<ConnectionManager>,
        agent_id: String,
        connected: &capydeploy_hub_connection::ConnectedAgent,
    ) -> Self {
        Self {
            mgr,
            agent_id,
            agent_ip: connected.agent.ips.first().copied(),
        }
    }
}

impl capydeploy_hub_deploy::AgentConnection for DeployAdapter {
    fn send_request(
        &self,
        msg_type: MessageType,
        payload: &serde_json::Value,
    ) -> Pin<
        Box<dyn Future<Output = Result<Message, capydeploy_hub_deploy::DeployError>> + Send + '_>,
    > {
        let mgr = self.mgr.clone();
        let payload = payload.clone();
        Box::pin(async move {
            mgr.send_request(msg_type, Some(&payload))
                .await
                .map_err(|e| capydeploy_hub_deploy::DeployError::Agent(e.to_string()))
        })
    }

    fn send_binary(
        &self,
        header: &serde_json::Value,
        data: &[u8],
    ) -> Pin<
        Box<dyn Future<Output = Result<Message, capydeploy_hub_deploy::DeployError>> + Send + '_>,
    > {
        let mgr = self.mgr.clone();
        let header = header.clone();
        let data = data.to_vec();
        Box::pin(async move {
            mgr.send_binary(&header, &data)
                .await
                .map_err(|e| capydeploy_hub_deploy::DeployError::Agent(e.to_string()))
        })
    }

    fn agent_id(&self) -> &str {
        &self.agent_id
    }

    fn agent_addr(&self) -> Option<std::net::IpAddr> {
        self.agent_ip
    }
}

// ---------------------------------------------------------------------------
// Games adapter
// ---------------------------------------------------------------------------

/// Implements `hub_games::AgentConnection` by delegating to `ConnectionManager`.
pub struct GamesAdapter {
    mgr: Arc<ConnectionManager>,
    agent_id: String,
}

impl GamesAdapter {
    pub fn new(mgr: Arc<ConnectionManager>, agent_id: String) -> Self {
        Self { mgr, agent_id }
    }
}

impl capydeploy_hub_games::AgentConnection for GamesAdapter {
    fn send_request(
        &self,
        msg_type: MessageType,
        payload: &serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<Message, capydeploy_hub_games::GamesError>> + Send + '_>>
    {
        let mgr = self.mgr.clone();
        let payload = payload.clone();
        Box::pin(async move {
            mgr.send_request(msg_type, Some(&payload))
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
        let mgr = self.mgr.clone();
        let header = header.clone();
        let data = data.to_vec();
        Box::pin(async move {
            mgr.send_binary(&header, &data)
                .await
                .map_err(|e| capydeploy_hub_games::GamesError::Agent(e.to_string()))
        })
    }

    fn agent_id(&self) -> &str {
        &self.agent_id
    }
}
