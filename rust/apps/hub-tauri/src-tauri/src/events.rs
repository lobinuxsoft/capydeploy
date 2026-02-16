//! Background task that forwards `ConnectionEvent`s to the Tauri frontend.

use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tracing::{debug, warn};

use capydeploy_hub_connection::{ConnectionEvent, ConnectionManager, ConnectionState};
use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::console_log::ConsoleLogBatch;
use capydeploy_protocol::telemetry::{TelemetryData, TelemetryStatusEvent};

use crate::state::HubState;
use crate::types::{
    ConnectionStatusDto, DiscoveredAgentDto, PairingRequiredDto, ReconnectingDto,
    UploadProgressDto,
};

/// Main event loop that bridges Rust events to the Tauri frontend.
pub async fn event_loop(handle: AppHandle, mgr: Arc<ConnectionManager>) {
    let Some(mut rx) = mgr.take_events().await else {
        warn!("events already taken");
        return;
    };

    mgr.start_discovery(Duration::from_secs(5)).await;
    debug!("event loop started");

    while let Some(event) = rx.recv().await {
        match event {
            ConnectionEvent::AgentFound(agent) => {
                let dto = DiscoveredAgentDto::from(&agent);
                let _ = handle.emit("discovery:agent-found", &dto);
            }

            ConnectionEvent::AgentUpdated(agent) => {
                let dto = DiscoveredAgentDto::from(&agent);
                let _ = handle.emit("discovery:agent-found", &dto);
            }

            ConnectionEvent::AgentLost(id) => {
                let _ = handle.emit("discovery:agent-lost", &id);
            }

            ConnectionEvent::StateChanged { agent_id, state } => {
                let status = match state {
                    ConnectionState::Connected => {
                        if let Some(connected) = mgr.get_connected().await {
                            ConnectionStatusDto::from_connected(&connected)
                        } else {
                            ConnectionStatusDto::disconnected()
                        }
                    }
                    _ => {
                        let mut dto = ConnectionStatusDto::disconnected();
                        dto.agent_id = agent_id;
                        dto
                    }
                };
                let _ = handle.emit("connection:changed", &status);
            }

            ConnectionEvent::PairingNeeded {
                agent_id,
                code,
                expires_in,
            } => {
                let dto = PairingRequiredDto {
                    agent_id,
                    code,
                    expires_in,
                };
                let _ = handle.emit("pairing:required", &dto);
            }

            ConnectionEvent::Reconnecting {
                agent_id,
                attempt,
                next_retry_secs,
            } => {
                let dto = ReconnectingDto {
                    agent_id,
                    attempt,
                    next_retry_secs,
                };
                let _ = handle.emit("connection:reconnecting", &dto);
            }

            ConnectionEvent::AgentEvent {
                agent_id,
                msg_type,
                message,
            } => {
                let state = handle.state::<HubState>();

                match msg_type {
                    MessageType::TelemetryData => {
                        if let Some(data) = message.parse_payload::<TelemetryData>().ok().flatten()
                        {
                            state
                                .telemetry_hub
                                .lock()
                                .await
                                .process_data(&agent_id, &data);
                            let _ = handle.emit("telemetry:data", &data);
                        }
                    }

                    MessageType::TelemetryStatus => {
                        if let Some(status) = message
                            .parse_payload::<TelemetryStatusEvent>()
                            .ok()
                            .flatten()
                        {
                            state
                                .telemetry_hub
                                .lock()
                                .await
                                .process_status(&agent_id, &status);
                            let _ = handle.emit("telemetry:status", &status);
                        }
                    }

                    MessageType::ConsoleLogData => {
                        if let Some(batch) =
                            message.parse_payload::<ConsoleLogBatch>().ok().flatten()
                        {
                            state
                                .console_hub
                                .lock()
                                .await
                                .process_batch(&agent_id, &batch);
                            let _ = handle.emit("consolelog:data", &batch);
                        }
                    }

                    MessageType::ConsoleLogStatus => {
                        if let Some(status) = message
                            .parse_payload::<capydeploy_protocol::console_log::ConsoleLogStatusEvent>()
                            .ok()
                            .flatten()
                        {
                            state
                                .console_hub
                                .lock()
                                .await
                                .process_status(&agent_id, &status);
                            let _ = handle.emit("consolelog:status", &status);
                        }
                    }

                    MessageType::UploadProgress => {
                        if let Some(progress) = message
                            .parse_payload::<capydeploy_protocol::types::UploadProgress>()
                            .ok()
                            .flatten()
                        {
                            let dto = UploadProgressDto {
                                progress: progress.percentage(),
                                status: format!("{:?}", progress.status),
                                error: if progress.error.is_empty() {
                                    None
                                } else {
                                    Some(progress.error.clone())
                                },
                                done: matches!(
                                    progress.status,
                                    capydeploy_protocol::types::UploadStatus::Completed
                                        | capydeploy_protocol::types::UploadStatus::Failed
                                        | capydeploy_protocol::types::UploadStatus::Cancelled
                                ),
                            };
                            let _ = handle.emit("upload:progress", &dto);
                        }
                    }

                    MessageType::GameLogWrapperStatus => {
                        if let Some(status) = message
                            .parse_payload::<capydeploy_protocol::telemetry::GameLogWrapperStatusEvent>()
                            .ok()
                            .flatten()
                        {
                            let _ = handle.emit("gamelog:status", &status);
                        }
                    }

                    MessageType::OperationEvent => {
                        if let Some(evt) = message
                            .parse_payload::<capydeploy_protocol::messages::OperationEvent>()
                            .ok()
                            .flatten()
                        {
                            let dto = UploadProgressDto {
                                progress: evt.progress,
                                status: evt.status.clone(),
                                error: None,
                                done: evt.status == "completed" || evt.status == "failed",
                            };
                            let _ = handle.emit("upload:progress", &dto);
                        }
                    }

                    _ => {
                        debug!(msg_type = ?msg_type, "unhandled agent event");
                    }
                }
            }
        }
    }
}
