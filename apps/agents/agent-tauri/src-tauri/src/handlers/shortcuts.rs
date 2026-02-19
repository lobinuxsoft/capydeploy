use capydeploy_agent_server::Sender;
use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::envelope::Message;
use capydeploy_protocol::messages;

use crate::handler::TauriAgentHandler;

impl TauriAgentHandler {
    pub(crate) async fn handle_list_shortcuts(&self, sender: Sender, msg: Message) {
        let req: messages::ListShortcutsRequest = match msg.parse_payload() {
            Ok(Some(r)) => r,
            _ => {
                let _ = sender.send_error(&msg, 400, "invalid payload");
                return;
            }
        };

        let user_id = req.user_id.to_string();
        let list = match capydeploy_steam::ShortcutManager::new() {
            Ok(sm) => {
                let vdf_path = sm.shortcuts_path(&user_id);
                capydeploy_steam::load_shortcuts_vdf(std::path::Path::new(&vdf_path))
                    .unwrap_or_default()
            }
            Err(e) => {
                tracing::warn!("failed to init ShortcutManager: {e}");
                vec![]
            }
        };

        let resp = messages::ShortcutsListResponse { shortcuts: list };
        if let Ok(reply) = msg.reply(MessageType::ShortcutsResponse, Some(&resp)) {
            let _ = sender.send_msg(reply);
        }
    }

    pub(crate) async fn handle_create_shortcut(&self, sender: Sender, msg: Message) {
        // TODO: implement VDF write for shortcut creation
        let _ = sender.send_error(&msg, 501, "shortcut creation not yet implemented");
    }

    pub(crate) async fn handle_delete_shortcut(&self, sender: Sender, msg: Message) {
        // TODO: implement VDF write for shortcut deletion
        let _ = sender.send_error(&msg, 501, "shortcut deletion not yet implemented");
    }

    pub(crate) async fn handle_apply_artwork(&self, sender: Sender, msg: Message) {
        // TODO: implement URL-based artwork download + apply
        let _ = sender.send_error(&msg, 501, "apply_artwork not yet implemented");
    }
}
