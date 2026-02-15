//! Devices view — discovered agents list with connection controls.

use cosmic::iced::Length;
use cosmic::widget::{self, container};
use cosmic::Element;

use capydeploy_discovery::types::DiscoveredAgent;
use capydeploy_hub_connection::ConnectionState;

use crate::message::Message;
use crate::theme;

/// Renders the devices page.
pub fn view<'a>(
    discovered: &'a [DiscoveredAgent],
    connected_agent_id: Option<&'a str>,
    states: &'a std::collections::HashMap<String, ConnectionState>,
) -> Element<'a, Message> {
    let mut content = widget::column().spacing(16);

    content = content.push(
        widget::row()
            .push(widget::text::title3("Devices").width(Length::Fill))
            .push(widget::button::standard("Refresh").on_press(Message::RefreshDiscovery))
            .align_y(cosmic::iced::Alignment::Center),
    );
    content = content.push(
        widget::text("Agents discovered on the local network").class(theme::MUTED_TEXT),
    );

    if discovered.is_empty() {
        content = content.push(
            container(
                widget::column()
                    .push(widget::text::heading("No agents found"))
                    .push(
                        widget::text(
                            "Make sure a CapyDeploy Agent is running on the same network.",
                        )
                        .class(theme::MUTED_TEXT),
                    )
                    .spacing(4)
                    .padding(24),
            )
            .width(Length::Fill)
            .class(cosmic::theme::Container::Custom(Box::new(theme::canvas_bg))),
        );
    } else {
        for agent in discovered {
            let agent_id = &agent.info.id;
            let state = states.get(agent_id).cloned();
            let is_connected = connected_agent_id == Some(agent_id);
            content = content.push(agent_card(agent, &state, is_connected));
        }
    }

    content.into()
}

/// Renders a single agent card with info and action button.
fn agent_card<'a>(
    agent: &'a DiscoveredAgent,
    state: &Option<ConnectionState>,
    is_connected: bool,
) -> Element<'a, Message> {
    let agent_id = agent.info.id.clone();

    // Agent info column.
    let info = widget::column()
        .push(widget::text::heading(&agent.info.name))
        .push(
            widget::row()
                .push(widget::text::caption(&agent.info.platform).class(theme::MUTED_TEXT))
                .push(widget::text::caption(" | ").class(theme::MUTED_TEXT))
                .push(
                    widget::text::caption(format!("v{}", agent.info.version))
                        .class(theme::MUTED_TEXT),
                )
                .push(widget::text::caption(" | ").class(theme::MUTED_TEXT))
                .push(widget::text::caption(agent.address()).class(theme::MUTED_TEXT)),
        )
        .spacing(2);

    // Status badge.
    let (badge_text, badge_color) = match state {
        Some(ConnectionState::Connected) => ("Connected".to_string(), theme::CONNECTED_COLOR),
        Some(ConnectionState::Connecting) => ("Connecting...".to_string(), theme::CYAN),
        Some(ConnectionState::Reconnecting { attempt }) => {
            (format!("Reconnecting ({attempt})..."), theme::ORANGE)
        }
        Some(ConnectionState::PairingRequired) => ("Pairing required".to_string(), theme::ORANGE),
        Some(ConnectionState::Disconnected) => ("Disconnected".to_string(), theme::MUTED_TEXT),
        Some(ConnectionState::Discovered) | None => ("Available".to_string(), theme::MUTED_TEXT),
    };

    let badge = widget::text::caption(badge_text).class(badge_color);

    // Action button.
    let action: Element<'_, Message> = if is_connected {
        widget::button::destructive("Disconnect")
            .on_press(Message::DisconnectAgent)
            .into()
    } else if matches!(state, Some(ConnectionState::Reconnecting { .. })) {
        widget::button::destructive("Cancel")
            .on_press(Message::CancelReconnect(agent_id))
            .into()
    } else {
        let busy = matches!(state, Some(ConnectionState::Connecting));
        let btn = widget::button::suggested("Connect");
        if busy {
            btn.into()
        } else {
            btn.on_press(Message::ConnectAgent(agent_id)).into()
        }
    };

    // Assemble row.
    let row = widget::row()
        .push(info.width(Length::Fill))
        .push(
            widget::column()
                .push(badge)
                .push(action)
                .spacing(4)
                .align_x(cosmic::iced::Alignment::End),
        )
        .spacing(16)
        .align_y(cosmic::iced::Alignment::Center)
        .padding(16);

    container(row)
        .width(Length::Fill)
        .class(cosmic::theme::Container::Custom(Box::new(theme::canvas_bg)))
        .into()
}
