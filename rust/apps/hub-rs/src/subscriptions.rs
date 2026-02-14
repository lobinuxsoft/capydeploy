//! Async-to-sync bridge via iced subscriptions.
//!
//! The `ConnectionManager` lives in tokio-land. This module exposes
//! its event stream as an `iced::Subscription` that the `Hub` app
//! consumes synchronously in `update()`.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::stream;

use capydeploy_hub_connection::ConnectionManager;
use capydeploy_hub_deploy::DeployEvent;

use crate::message::Message;

/// Internal state machine for the connection subscription.
enum SubState {
    /// First poll: take the events receiver and start discovery.
    Init(Arc<ConnectionManager>, tokio::runtime::Handle),
    /// Subsequent polls: forward events from the receiver.
    Running(tokio::sync::mpsc::Receiver<capydeploy_hub_connection::ConnectionEvent>),
}

/// Internal state machine for the deploy events subscription.
enum DeploySubState {
    /// First poll: take the receiver from the shared holder.
    Init(Arc<Mutex<Option<tokio::sync::mpsc::Receiver<DeployEvent>>>>),
    /// Subsequent polls: forward events from the receiver.
    Running(tokio::sync::mpsc::Receiver<DeployEvent>),
}

/// Discovery interval for mDNS scanning.
const DISCOVERY_INTERVAL: Duration = Duration::from_secs(5);

/// Creates a subscription that streams `ConnectionEvent`s as `Message`s.
///
/// On first poll, it takes the event receiver from the manager and
/// starts mDNS discovery on the provided tokio runtime. Subsequent
/// polls forward events to the iced runtime.
pub fn connection_events(
    mgr: Arc<ConnectionManager>,
    handle: tokio::runtime::Handle,
) -> cosmic::iced::Subscription<Message> {
    cosmic::iced::Subscription::run_with_id(
        "connection-events",
        stream::unfold(SubState::Init(mgr, handle), |state| async move {
            match state {
                SubState::Init(mgr, handle) => {
                    // ConnectionManager::start_discovery() uses tokio::spawn internally.
                    // iced's subscription executor doesn't provide a tokio context, so
                    // we use the Hub's global tokio runtime via block_on (safe because
                    // this runs on iced's executor, not inside tokio).
                    let mgr_init = mgr.clone();
                    let rx = handle.block_on(async move {
                        let rx = mgr_init.take_events().await;
                        if rx.is_some() {
                            mgr_init.start_discovery(DISCOVERY_INTERVAL).await;
                        }
                        rx
                    })?;

                    tracing::info!("mDNS discovery started");
                    Some((Message::DiscoveryStarted, SubState::Running(rx)))
                }
                SubState::Running(mut rx) => match rx.recv().await {
                    Some(event) => Some((
                        Message::ConnectionEvent(event),
                        SubState::Running(rx),
                    )),
                    None => {
                        tracing::debug!("connection event channel closed");
                        None
                    }
                },
            }
        }),
    )
}

/// Creates a subscription that streams `DeployEvent`s as `Message`s.
///
/// Takes a shared holder for the events receiver. On first poll, it
/// extracts the receiver; subsequent polls forward events to the runtime.
/// The subscription terminates when the channel closes (deploy completed).
pub fn deploy_events(
    rx_holder: Arc<Mutex<Option<tokio::sync::mpsc::Receiver<DeployEvent>>>>,
) -> cosmic::iced::Subscription<Message> {
    cosmic::iced::Subscription::run_with_id(
        "deploy-events",
        stream::unfold(DeploySubState::Init(rx_holder), |state| async move {
            match state {
                DeploySubState::Init(holder) => {
                    let mut rx = holder.lock().ok()?.take()?;
                    // Immediately try to read the first event.
                    rx.recv()
                        .await
                        .map(|event| (Message::DeployProgress(event), DeploySubState::Running(rx)))
                }
                DeploySubState::Running(mut rx) => match rx.recv().await {
                    Some(event) => {
                        Some((Message::DeployProgress(event), DeploySubState::Running(rx)))
                    }
                    None => {
                        tracing::debug!("deploy event channel closed");
                        None
                    }
                },
            }
        }),
    )
}
