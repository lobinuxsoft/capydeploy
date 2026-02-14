//! Settings view — Hub configuration (name, SteamGridDB API key, log directory).

use cosmic::iced::Length;
use cosmic::widget::{self, container};
use cosmic::Element;

use crate::config::HubConfig;
use crate::message::{Message, SettingField};
use crate::theme;

/// Renders the settings view.
pub fn view(config: &HubConfig, dirty: bool) -> Element<'_, Message> {
    let mut content = widget::column().spacing(16);

    content = content.push(widget::text::title3("Settings"));

    // Settings form.
    let form = widget::column()
        .push(setting_field("Hub Name", &config.name, SettingField::Name))
        .push(setting_field(
            "SteamGridDB API Key",
            &config.steamgriddb_api_key,
            SettingField::SteamGridDbApiKey,
        ))
        .push(setting_field(
            "Game Log Directory",
            &config.game_log_dir,
            SettingField::GameLogDir,
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

    // Hub info footer.
    content = content.push(
        widget::text::caption(format!("Hub ID: {}", config.hub_id)).class(theme::MUTED_TEXT),
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
