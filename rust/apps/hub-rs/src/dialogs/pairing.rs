//! Pairing dialog — modal overlay for entering the agent's pairing code.

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, container};
use cosmic::Element;

use crate::message::Message;
use crate::theme;

/// State for the pairing dialog.
#[derive(Debug, Clone)]
pub struct PairingDialog {
    /// Agent ID that requires pairing.
    pub agent_id: String,
    /// The code displayed on the agent.
    pub code: String,
    /// User's input (code confirmation).
    pub input: String,
    /// Seconds until the code expires.
    pub expires_in: i32,
    /// Whether a pairing attempt is in progress.
    pub confirming: bool,
}

impl PairingDialog {
    /// Creates a new pairing dialog.
    pub fn new(agent_id: String, code: String, expires_in: i32) -> Self {
        Self {
            agent_id,
            code,
            input: String::new(),
            expires_in,
            confirming: false,
        }
    }
}

/// Renders the pairing dialog as an overlay element.
pub fn view(dialog: &PairingDialog) -> Element<'_, Message> {
    let title = widget::text::title4("Pairing Required");

    let instructions = widget::column()
        .push(widget::text(
            "Enter the code shown on the agent to confirm pairing.",
        ))
        .push(
            widget::text(format!("Code expires in {} seconds.", dialog.expires_in))
                .class(theme::MUTED_TEXT),
        )
        .spacing(4);

    // Display code prominently.
    let code_display = container(
        widget::text::title3(&dialog.code).class(theme::CYAN),
    )
    .padding([12, 24])
    .class(cosmic::theme::Container::Custom(Box::new(theme::canvas_bg)));

    let input = widget::text_input("Enter code...", &dialog.input)
        .on_input(Message::PairingCodeInput);

    let can_confirm = !dialog.input.is_empty() && !dialog.confirming;

    let confirm_btn = widget::button::suggested("Confirm");
    let confirm_btn = if can_confirm {
        confirm_btn.on_press(Message::ConfirmPairing)
    } else {
        confirm_btn
    };

    let cancel_btn = if dialog.confirming {
        widget::button::standard("Cancel")
    } else {
        widget::button::standard("Cancel").on_press(Message::CancelPairing)
    };

    let buttons = widget::row()
        .push(cancel_btn)
        .push(confirm_btn)
        .spacing(8)
        .align_y(Alignment::Center);

    let dialog_content = widget::column()
        .push(title)
        .push(instructions)
        .push(code_display)
        .push(input)
        .push(buttons)
        .spacing(16)
        .padding(24)
        .max_width(420.0);

    // Dialog box centered on screen.
    let dialog_box = container(dialog_content)
        .class(cosmic::theme::Container::Custom(Box::new(dialog_bg)));

    // Dimmed backdrop.
    container(
        container(dialog_box)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .class(cosmic::theme::Container::Custom(Box::new(backdrop_bg)))
    .into()
}

/// Dialog box background.
fn dialog_bg(_theme: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    cosmic::iced::widget::container::Style {
        background: Some(cosmic::iced::Background::Color(cosmic::iced::Color::from_rgb(
            0.14, 0.14, 0.16,
        ))),
        border: cosmic::iced::Border {
            radius: 12.0.into(),
            width: 1.0,
            color: cosmic::iced::Color::from_rgba(1.0, 1.0, 1.0, 0.1),
        },
        ..Default::default()
    }
}

/// Semi-transparent backdrop.
fn backdrop_bg(_theme: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    cosmic::iced::widget::container::Style {
        background: Some(cosmic::iced::Background::Color(cosmic::iced::Color::from_rgba(
            0.0, 0.0, 0.0, 0.6,
        ))),
        ..Default::default()
    }
}
