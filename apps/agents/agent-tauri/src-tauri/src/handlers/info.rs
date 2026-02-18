use std::sync::atomic::Ordering;

use capydeploy_agent_server::Sender;
use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::envelope::Message;
use capydeploy_protocol::messages;

use crate::handler::TauriAgentHandler;
use crate::helpers::generate_agent_id;

impl TauriAgentHandler {
    pub(crate) async fn handle_get_info(&self, sender: Sender, msg: Message) {
        let config = self.state.config.lock().await;
        let info = capydeploy_protocol::types::AgentInfo {
            id: generate_agent_id(&config.name),
            name: config.name.clone(),
            platform: std::env::consts::OS.into(),
            version: env!("CAPYDEPLOY_VERSION").into(),
            accept_connections: self.state.accept_connections.load(Ordering::Relaxed),
            supported_image_formats: vec!["png".into(), "jpg".into(), "jpeg".into(), "webp".into()],
        };
        let resp = messages::InfoResponse { agent: info };
        if let Ok(reply) = msg.reply(MessageType::InfoResponse, Some(&resp)) {
            let _ = sender.send_msg(reply);
        }
    }

    pub(crate) async fn handle_get_config(&self, sender: Sender, msg: Message) {
        let config = self.state.config.lock().await;
        let resp = messages::ConfigResponse {
            install_path: config.install_path.clone(),
        };
        if let Ok(reply) = msg.reply(MessageType::ConfigResponse, Some(&resp)) {
            let _ = sender.send_msg(reply);
        }
    }
}
