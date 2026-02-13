//! Application orchestrator — wires all agent components together.

use std::sync::Arc;

use capydeploy_agent_server::{AgentServer, Handler, HandlerFuture, Sender, ServerConfig};
use capydeploy_console_log::Collector as ConsoleLogCollector;
use capydeploy_discovery::{Server as DiscoveryServer, ServiceInfo, detect_platform};
use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::envelope::Message;
use capydeploy_protocol::messages;
use capydeploy_telemetry::Collector as TelemetryCollector;
use capydeploy_tray::{TrayConfig, TrayEvent, TrayHandle};
use tokio_util::sync::CancellationToken;

use crate::config::Config;

/// Runs the agent until shutdown is requested.
pub async fn run(config: Config) -> anyhow::Result<()> {
    let cancel = CancellationToken::new();

    // -- Telemetry --
    let telemetry = Arc::new(TelemetryCollector::new(Box::new(|_data| {
        // Wired to WS send in handler.
    })));

    // -- Console log --
    let console_log = Arc::new(ConsoleLogCollector::new(Box::new(|_batch| {
        // Wired to WS send in handler.
    })));

    // -- WS server --
    let handler = AgentHandler {
        config: config.clone(),
        telemetry: Arc::clone(&telemetry),
        console_log: Arc::clone(&console_log),
    };

    let server_config = ServerConfig {
        port: config.port,
        accept_connections: true,
    };

    let server = AgentServer::new(server_config, handler);
    let server_run = Arc::clone(&server);
    tokio::spawn(async move {
        if let Err(e) = server_run.run().await {
            tracing::error!("server error: {e}");
        }
    });

    // Wait for the server to bind.
    let port = loop {
        let p = server.port().await;
        if p > 0 {
            break p;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    };

    tracing::info!(port, "WebSocket server listening");

    // -- Discovery server --
    let agent_id = uuid::Uuid::new_v4().to_string();
    let service_info = ServiceInfo {
        id: agent_id,
        name: config.name.clone(),
        platform: detect_platform().to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        port,
        ips: vec![],
    };

    let mut discovery = DiscoveryServer::new(service_info);
    discovery.start()?;
    tracing::info!("mDNS discovery registered");

    // -- Tray --
    let tray_config = TrayConfig {
        agent_name: config.name.clone(),
        ..TrayConfig::default()
    };
    let (tray_handle, _event_tx, _update_rx) = TrayHandle::new(tray_config);

    // -- Start services --
    if config.telemetry_enabled {
        telemetry.start(config.telemetry_interval).await;
    }

    if config.console_log_enabled {
        console_log.start().await;
    }

    tracing::info!("agent ready");

    // -- Main loop: wait for shutdown --
    tokio::select! {
        _ = cancel.cancelled() => {
            tracing::info!("shutdown signal received");
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("SIGINT received, shutting down");
        }
        _ = async {
            loop {
                if let Some(TrayEvent::QuitRequested) = tray_handle.try_recv_event() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        } => {
            tracing::info!("quit requested via tray");
        }
    }

    // -- Graceful shutdown --
    tracing::info!("stopping services...");
    telemetry.stop().await;
    console_log.stop().await;
    discovery.stop()?;
    server.shutdown();
    tray_handle.shutdown();

    Ok(())
}

/// Agent-specific message handler.
struct AgentHandler {
    config: Config,
    telemetry: Arc<TelemetryCollector>,
    console_log: Arc<ConsoleLogCollector>,
}

impl Handler for AgentHandler {
    fn on_hub_connected(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            tracing::info!("Hub connected");

            let status = messages::AgentStatusResponse {
                name: self.config.name.clone(),
                platform: detect_platform().to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                accept_connections: true,
                telemetry_enabled: self.telemetry.is_running().await,
                telemetry_interval: self.config.telemetry_interval as i32,
                console_log_enabled: self.console_log.is_running().await,
            };

            match msg.reply(MessageType::AgentStatus, Some(&status)) {
                Ok(reply) => {
                    let _ = sender.send_msg(reply);
                }
                Err(e) => tracing::error!("failed to build reply: {e}"),
            }
        })
    }

    fn on_get_info(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let info = capydeploy_protocol::AgentInfo {
                id: String::new(),
                name: self.config.name.clone(),
                platform: detect_platform().to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                accept_connections: true,
                supported_image_formats: vec![],
            };

            let resp = messages::InfoResponse { agent: info };
            match msg.reply(MessageType::InfoResponse, Some(&resp)) {
                Ok(reply) => {
                    let _ = sender.send_msg(reply);
                }
                Err(e) => tracing::error!("failed to build reply: {e}"),
            }
        })
    }

    fn on_set_console_log_enabled(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let req: messages::SetConsoleLogEnabledRequest = match msg.parse_payload() {
                Ok(Some(r)) => r,
                Ok(None) => {
                    let _ = sender.send_error(&msg, 400, "missing payload");
                    return;
                }
                Err(e) => {
                    let _ = sender.send_error(&msg, 400, &format!("invalid payload: {e}"));
                    return;
                }
            };

            if req.enabled {
                self.console_log.start().await;
            } else {
                self.console_log.stop().await;
            }

            let resp = messages::SetConsoleLogEnabledResponse {
                enabled: req.enabled,
            };
            match msg.reply(MessageType::SetConsoleLogEnabled, Some(&resp)) {
                Ok(reply) => {
                    let _ = sender.send_msg(reply);
                }
                Err(e) => tracing::error!("failed to build reply: {e}"),
            }
        })
    }

    fn on_set_console_log_filter(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let req: messages::SetConsoleLogFilterRequest = match msg.parse_payload() {
                Ok(Some(r)) => r,
                Ok(None) => {
                    let _ = sender.send_error(&msg, 400, "missing payload");
                    return;
                }
                Err(e) => {
                    let _ = sender.send_error(&msg, 400, &format!("invalid payload: {e}"));
                    return;
                }
            };

            self.console_log.set_level_mask(req.level_mask);

            let resp = messages::SetConsoleLogFilterResponse {
                level_mask: req.level_mask,
            };
            match msg.reply(MessageType::SetConsoleLogFilter, Some(&resp)) {
                Ok(reply) => {
                    let _ = sender.send_msg(reply);
                }
                Err(e) => tracing::error!("failed to build reply: {e}"),
            }
        })
    }
}
