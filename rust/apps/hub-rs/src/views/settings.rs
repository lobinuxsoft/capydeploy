//! Settings view — Hub configuration (name, SteamGridDB API key, log directory).

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, container};
use cosmic::Element;

use crate::config::HubConfig;
use crate::message::{Message, SettingField};
use crate::theme;

/// Renders the settings view.
pub fn view(config: &HubConfig, dirty: bool, api_key_hidden: bool) -> Element<'_, Message> {
    let mut content = widget::column().spacing(16);

    content = content.push(widget::text::title3("Settings"));

    // Settings form.
    let form = widget::column()
        .push(setting_field("Hub Name", &config.name, SettingField::Name))
        .push(api_key_field(
            &config.steamgriddb_api_key,
            api_key_hidden,
        ))
        .push(path_setting_field(
            "Game Log Directory",
            &config.game_log_dir,
        ))
        .spacing(12);

    content = content.push(
        container(
            widget::column()
                .push(form)
                .spacing(16)
                .padding(24),
        )
        .width(Length::Fill)
        .class(cosmic::theme::Container::Custom(Box::new(theme::canvas_bg))),
    );

    // Save button — only enabled when changes are pending.
    let save_btn = if dirty {
        widget::button::suggested("Save Settings").on_press(Message::SaveSettings)
    } else {
        widget::button::standard("Settings Saved")
    };
    content = content.push(save_btn);

    // About section.
    let about = widget::column()
        .push(widget::text::title4("About"))
        .push(
            widget::text::caption(format!("Version: {}", env!("CARGO_PKG_VERSION")))
                .class(theme::MUTED_TEXT),
        )
        .push(
            widget::text::caption(format!("Hub ID: {}", config.hub_id))
                .class(theme::MUTED_TEXT),
        )
        .push(
            widget::text::caption(format!("Platform: {} ({})", std::env::consts::OS, std::env::consts::ARCH))
                .class(theme::MUTED_TEXT),
        )
        .spacing(4)
        .padding(24);

    content = content.push(
        container(about)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Custom(Box::new(theme::canvas_bg))),
    );

    content.into()
}

/// Renders a labeled text input for a setting field.
fn setting_field<'a>(
    label: &'a str,
    value: &'a str,
    field: SettingField,
) -> Element<'a, Message> {
    widget::column()
        .push(widget::text::caption(label).class(theme::MUTED_TEXT))
        .push(
            widget::text_input(label, value)
                .on_input(move |v| Message::UpdateSetting(field.clone(), v)),
        )
        .spacing(4)
        .into()
}

/// Renders the SteamGridDB API key as a secure (password) input with toggle.
fn api_key_field(value: &str, hidden: bool) -> Element<'_, Message> {
    widget::column()
        .push(
            widget::text::caption("SteamGridDB API Key").class(theme::MUTED_TEXT),
        )
        .push(
            widget::secure_input("Enter API key...", value, Some(Message::ToggleApiKeyVisibility), hidden)
                .on_input(|v| Message::UpdateSetting(SettingField::SteamGridDbApiKey, v)),
        )
        .spacing(4)
        .into()
}

/// Renders the game log directory field with Browse and Clear buttons.
fn path_setting_field<'a>(label: &'a str, value: &'a str) -> Element<'a, Message> {
    let mut row = widget::row()
        .push(
            widget::text_input(label, value)
                .on_input(|v| Message::UpdateSetting(SettingField::GameLogDir, v))
                .width(Length::Fill),
        )
        .push(widget::button::standard("Browse").on_press(Message::BrowseGameLogDir))
        .spacing(8)
        .align_y(Alignment::Center);

    if !value.is_empty() {
        row = row.push(widget::button::destructive("Clear").on_press(Message::ClearGameLogDir));
    }

    widget::column()
        .push(widget::text::caption(label).class(theme::MUTED_TEXT))
        .push(row)
        .spacing(4)
        .into()
}
