//! Handler trait for processing WebSocket messages.
//!
//! Implementors provide domain logic (shortcuts, uploads, telemetry, etc.)
//! while the server framework handles connection management, routing, and
//! the binary protocol.

use std::future::Future;
use std::pin::Pin;

use capydeploy_protocol::envelope::Message;

use crate::connection::Sender;

/// A boxed future returned by handler methods.
pub type HandlerFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

/// Trait for handling WebSocket messages from a Hub.
///
/// The server dispatches parsed messages to the appropriate method. Each
/// method receives:
/// - `sender`: channel to send responses/events back to the Hub
/// - `msg`: the parsed JSON envelope (for text) or binary message
///
/// Default implementations reply with "not implemented" so handlers only
/// need to override the message types they care about.
pub trait Handler: Send + Sync + 'static {
    /// Called when a Hub first connects (`hub_connected`).
    /// The handler should validate auth/pairing and send `agent_status`
    /// or `pairing_required` via the sender.
    fn on_hub_connected(&self, sender: Sender, msg: Message) -> HandlerFuture<'_>;

    /// Called when a Hub confirms pairing (`pair_confirm`).
    fn on_pair_confirm(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let _ = sender.send_error(&msg, 501, "pairing not implemented");
        })
    }

    /// Called for `ping` messages.
    fn on_ping(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            if let Ok(reply) =
                msg.reply(capydeploy_protocol::MessageType::Pong, Option::<&()>::None)
            {
                let _ = sender.send_msg(reply);
            }
        })
    }

    /// Called for `get_info`.
    fn on_get_info(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let _ = sender.send_error(&msg, 501, "not implemented");
        })
    }

    /// Called for `get_config`.
    fn on_get_config(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let _ = sender.send_error(&msg, 501, "not implemented");
        })
    }

    /// Called for `get_steam_users`.
    fn on_get_steam_users(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let _ = sender.send_error(&msg, 501, "not implemented");
        })
    }

    /// Called for `list_shortcuts`.
    fn on_list_shortcuts(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let _ = sender.send_error(&msg, 501, "not implemented");
        })
    }

    /// Called for `create_shortcut`.
    fn on_create_shortcut(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let _ = sender.send_error(&msg, 501, "not implemented");
        })
    }

    /// Called for `delete_shortcut`.
    fn on_delete_shortcut(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let _ = sender.send_error(&msg, 501, "not implemented");
        })
    }

    /// Called for `delete_game`.
    fn on_delete_game(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let _ = sender.send_error(&msg, 501, "not implemented");
        })
    }

    /// Called for `apply_artwork`.
    fn on_apply_artwork(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let _ = sender.send_error(&msg, 501, "not implemented");
        })
    }

    /// Called for `restart_steam`.
    fn on_restart_steam(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let _ = sender.send_error(&msg, 501, "not implemented");
        })
    }

    /// Called for `init_upload`.
    fn on_init_upload(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let _ = sender.send_error(&msg, 501, "not implemented");
        })
    }

    /// Called for `upload_chunk` (JSON metadata — binary data uses `on_binary_chunk`).
    fn on_upload_chunk(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let _ = sender.send_error(&msg, 501, "not implemented");
        })
    }

    /// Called for `complete_upload`.
    fn on_complete_upload(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let _ = sender.send_error(&msg, 501, "not implemented");
        })
    }

    /// Called for `cancel_upload`.
    fn on_cancel_upload(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let _ = sender.send_error(&msg, 501, "not implemented");
        })
    }

    /// Called for `set_console_log_filter`.
    fn on_set_console_log_filter(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let _ = sender.send_error(&msg, 501, "not implemented");
        })
    }

    /// Called for `set_console_log_enabled`.
    fn on_set_console_log_enabled(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let _ = sender.send_error(&msg, 501, "not implemented");
        })
    }

    /// Called for `set_game_log_wrapper`.
    fn on_set_game_log_wrapper(&self, sender: Sender, msg: Message) -> HandlerFuture<'_> {
        Box::pin(async move {
            let _ = sender.send_error(&msg, 501, "not implemented");
        })
    }

    /// Called for a binary upload chunk.
    fn on_binary_chunk(
        &self,
        sender: Sender,
        header: crate::binary::BinaryChunkHeader,
        data: Vec<u8>,
    ) -> HandlerFuture<'_> {
        let _ = (header, data);
        Box::pin(async move {
            // No message ID available for binary; log and drop.
            tracing::warn!("binary chunk received but handler not implemented");
            let _ = sender;
        })
    }

    /// Called for a binary artwork image.
    fn on_binary_artwork(
        &self,
        sender: Sender,
        header: crate::binary::BinaryArtworkHeader,
        data: Vec<u8>,
    ) -> HandlerFuture<'_> {
        let _ = (header, data);
        Box::pin(async move {
            tracing::warn!("binary artwork received but handler not implemented");
            let _ = sender;
        })
    }

    /// Called when the Hub disconnects (cleanup hook).
    fn on_hub_disconnected(&self) -> HandlerFuture<'_> {
        Box::pin(async {})
    }
}
