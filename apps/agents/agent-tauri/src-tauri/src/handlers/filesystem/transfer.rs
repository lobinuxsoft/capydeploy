//! TCP file transfer handlers: fs_download, fs_upload.

use capydeploy_agent_server::Sender;
use capydeploy_protocol::constants::{self, MessageType};
use capydeploy_protocol::envelope::Message;
use capydeploy_protocol::messages;

use crate::handler::TauriAgentHandler;

impl TauriAgentHandler {
    /// Handle fs_download — always uses TCP data channel.
    pub(crate) async fn handle_fs_download(&self, sender: Sender, msg: Message) {
        let req: messages::FsDownloadRequest = match msg.parse_payload() {
            Ok(Some(r)) => r,
            _ => {
                let _ =
                    sender.send_error(&msg, constants::WS_ERR_CODE_BAD_REQUEST, "invalid payload");
                return;
            }
        };

        let canonical = match self.validate_path(&req.path, &msg, &sender) {
            Some(p) => p,
            None => return,
        };

        let meta = match tokio::fs::metadata(&canonical).await {
            Ok(m) => m,
            Err(e) => {
                let _ = sender.send_error(
                    &msg,
                    constants::WS_ERR_CODE_BAD_REQUEST,
                    &format!("cannot stat file: {e}"),
                );
                return;
            }
        };

        if meta.is_dir() {
            let _ = sender.send_error(
                &msg,
                constants::WS_ERR_CODE_BAD_REQUEST,
                "cannot download a directory",
            );
            return;
        }

        let file_size = meta.len();
        let cancel = self.state.shutdown_token.clone();

        let listener = match tokio::net::TcpListener::bind("0.0.0.0:0").await {
            Ok(l) => l,
            Err(e) => {
                let _ = sender.send_error(
                    &msg,
                    constants::WS_ERR_CODE_INTERNAL,
                    &format!("failed to bind TCP: {e}"),
                );
                return;
            }
        };
        let tcp_port = listener.local_addr().unwrap().port();
        let tcp_token = capydeploy_data_channel::generate_token();

        let resp = messages::FsDownloadReadyResponse {
            path: canonical.to_string_lossy().into_owned(),
            size: file_size as i64,
            tcp_port,
            tcp_token: tcp_token.clone(),
        };
        if let Ok(reply) = msg.reply(MessageType::FsDownloadReady, Some(&resp)) {
            let _ = sender.send_msg(reply);
        }

        let file_name = canonical
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "download".to_string());
        let files = vec![(canonical, file_name)];
        let (progress_tx, _) = tokio::sync::mpsc::channel(64);

        tokio::spawn(async move {
            match capydeploy_data_channel::server::TcpDataServer::accept_and_send(
                listener,
                &tcp_token,
                &files,
                cancel,
                progress_tx,
            )
            .await
            {
                Ok(bytes) => tracing::info!(bytes, "fs_download TCP complete"),
                Err(e) => tracing::error!("fs_download TCP failed: {e}"),
            }
        });
    }

    /// Handle fs_upload — always uses TCP data channel.
    pub(crate) async fn handle_fs_upload(&self, sender: Sender, msg: Message) {
        let req: messages::FsUploadRequest = match msg.parse_payload() {
            Ok(Some(r)) => r,
            _ => {
                let _ =
                    sender.send_error(&msg, constants::WS_ERR_CODE_BAD_REQUEST, "invalid payload");
                return;
            }
        };

        let dir = match self.validate_path(&req.path, &msg, &sender) {
            Some(p) => p,
            None => return,
        };

        let cancel = self.state.shutdown_token.clone();
        let base_path = dir.clone();

        let tcp_server = capydeploy_data_channel::server::TcpDataServer::new(base_path, cancel);
        let (info, listener) = match tcp_server.listen().await {
            Ok(r) => r,
            Err(e) => {
                let _ = sender.send_error(
                    &msg,
                    constants::WS_ERR_CODE_INTERNAL,
                    &format!("failed to start TCP: {e}"),
                );
                return;
            }
        };

        let resp = messages::FsUploadReadyResponse {
            tcp_port: info.port,
            tcp_token: info.token.clone(),
        };
        if let Ok(reply) = msg.reply(MessageType::FsUploadReady, Some(&resp)) {
            let _ = sender.send_msg(reply);
        }

        let token = info.token;
        let (progress_tx, _) = tokio::sync::mpsc::channel(64);

        tokio::spawn(async move {
            match tcp_server
                .accept_and_receive(listener, &token, progress_tx)
                .await
            {
                Ok(bytes) => tracing::info!(bytes, "fs_upload TCP complete"),
                Err(e) => tracing::error!("fs_upload TCP failed: {e}"),
            }
        });
    }
}
