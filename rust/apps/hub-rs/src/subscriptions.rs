//! Async-to-sync bridge via iced subscriptions.
//!
//! The `ConnectionManager` lives in tokio-land. This module exposes
//! its event stream as an `iced::Subscription` that the `Hub` app
//! consumes synchronously in `update()`.

use std::sync::Arc;
use std::time::Duration;

use futures_util::stream;

use capydeploy_hub_connection::ConnectionManager;

use crate::message::Message;

/// Internal state machine for the connection subscription.
enum SubState {
    /// First poll: take the events receiver and start discovery.
    Init(Arc<ConnectionManager>),
    /// Subsequent polls: forward events from the receiver.
    Running(tokio::sync::mpsc::Receiver<capydeploy_hub_connection::ConnectionEvent>),
}

/// Discovery interval for mDNS scanning.
const DISCOVERY_INTERVAL: Duration = Duration::from_secs(5);

/// Creates a subscription that streams `ConnectionEvent`s as `Message`s.
///
/// On first poll, it takes the event receiver from the manager and
/// starts mDNS discovery. Subsequent polls forward events to the
/// iced runtime.
pub fn connection_events(mgr: Arc<ConnectionManager>) -> cosmic::iced::Subscription<Message> {
    cosmic::iced::Subscription::run_with_id(
        "connection-events",
        stream::unfold(SubState::Init(mgr), |state| async move {
            match state {
                SubState::Init(mgr) => {
                    if let Some(rx) = mgr.take_events().await {
                        mgr.start_discovery(DISCOVERY_INTERVAL).await;
                        tracing::info!("mDNS discovery started");
                        Some((Message::DiscoveryStarted, SubState::Running(rx)))
                    } else {
                        tracing::warn!("connection events already taken");
                        None
                    }
                }
                SubState::Running(mut rx) => match rx.recv().await {
                    Some(event) => {
                        Some((Message::ConnectionEvent(event), SubState::Running(rx)))
                    }
                    None => {
                        tracing::debug!("connection event channel closed");
                        None
                    }
                },
            }
        }),
    )
}
