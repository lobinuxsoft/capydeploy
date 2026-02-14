//! Deploy view — game setup CRUD and deployment to agents.

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, container};
use cosmic::Element;

use capydeploy_hub_deploy::GameSetup;

use crate::message::{Message, SetupField};
use crate::theme;

/// Deploy status shown after a deploy attempt.
#[derive(Debug, Clone)]
pub enum DeployStatus {
    Deploying {
        setup_name: String,
        progress: f64,
        status_msg: String,
    },
    Success { setup_name: String, app_id: u32 },
    Failed { setup_name: String, error: String },
}

/// Renders the deploy view.
pub fn view<'a>(
    setups: &'a [GameSetup],
    editing: Option<&'a GameSetup>,
    deploy_status: Option<&'a DeployStatus>,
    is_connected: bool,
    deploying: bool,
) -> Element<'a, Message> {
    let mut content = widget::column().spacing(16);

    content = content.push(widget::text::title3("Deploy"));

    // Show deploy status banner if present.
    if let Some(status) = deploy_status {
        content = content.push(status_banner(status));
    }

    // Show edit form or setup list.
    if let Some(setup) = editing {
        content = content.push(edit_form(setup));
    } else {
        let new_btn = widget::button::suggested("+ New Setup").on_press(Message::NewSetup);
        content = content.push(new_btn);

        if setups.is_empty() {
            content = content.push(
                container(
                    widget::column()
                        .push(widget::text::heading("No game setups"))
                        .push(
                            widget::text(
                                "Create a game setup to deploy games to the connected agent.",
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
            for setup in setups {
                content = content.push(setup_card(setup, is_connected, deploying));
            }
        }
    }

    widget::scrollable(content).into()
}

/// Renders a game setup card with info and action buttons.
fn setup_card<'a>(setup: &'a GameSetup, is_connected: bool, deploying: bool) -> Element<'a, Message> {
    let info = widget::column()
        .push(widget::text::heading(&setup.name))
        .push(
            widget::row()
                .push(widget::text::caption(&setup.local_path).class(theme::MUTED_TEXT))
                .push(widget::text::caption(" → ").class(theme::MUTED_TEXT))
                .push(widget::text::caption(&setup.install_path).class(theme::MUTED_TEXT)),
        )
        .push(
            widget::text::caption(format!("Exe: {}", setup.executable))
                .class(theme::MUTED_TEXT),
        )
        .spacing(2);

    let edit_btn = widget::button::standard("Edit")
        .on_press(Message::EditSetup(setup.id.clone()));

    let deploy_btn = if is_connected && !deploying {
        widget::button::suggested("Deploy").on_press(Message::StartDeploy(setup.id.clone()))
    } else {
        widget::button::suggested("Deploy")
    };

    let delete_btn = widget::button::destructive("Delete")
        .on_press(Message::DeleteSetup(setup.id.clone()));

    let actions = widget::row()
        .push(edit_btn)
        .push(deploy_btn)
        .push(delete_btn)
        .spacing(6)
        .align_y(Alignment::Center);

    let row = widget::row()
        .push(info.width(Length::Fill))
        .push(actions)
        .spacing(16)
        .align_y(Alignment::Center)
        .padding(16);

    container(row)
        .width(Length::Fill)
        .class(cosmic::theme::Container::Custom(Box::new(theme::canvas_bg)))
        .into()
}

/// Renders the game setup edit form.
fn edit_form(setup: &GameSetup) -> Element<'_, Message> {
    let form = widget::column()
        .push(widget::text::title4("Edit Game Setup"))
        .push(form_field("Name", &setup.name, SetupField::Name))
        .push(form_field("Local Path", &setup.local_path, SetupField::LocalPath))
        .push(form_field("Executable", &setup.executable, SetupField::Executable))
        .push(form_field(
            "Install Path (on agent)",
            &setup.install_path,
            SetupField::InstallPath,
        ))
        .push(form_field(
            "Launch Options",
            &setup.launch_options,
            SetupField::LaunchOptions,
        ))
        .push(form_field("Tags (comma-separated)", &setup.tags, SetupField::Tags))
        .spacing(12);

    // Artwork section.
    let artwork_count = [
        &setup.grid_portrait,
        &setup.grid_landscape,
        &setup.hero_image,
        &setup.logo_image,
        &setup.icon_image,
    ]
    .iter()
    .filter(|s| !s.is_empty())
    .count();

    let artwork_label = if artwork_count > 0 {
        format!("Artwork ({artwork_count} selected)")
    } else {
        "Select Artwork".to_string()
    };
    let artwork_btn = widget::button::standard(artwork_label)
        .on_press(Message::OpenArtworkSelector);

    let can_save = !setup.name.is_empty()
        && !setup.local_path.is_empty()
        && !setup.executable.is_empty();

    let save_btn = if can_save {
        widget::button::suggested("Save").on_press(Message::SaveSetup)
    } else {
        widget::button::suggested("Save")
    };
    let cancel_btn = widget::button::standard("Cancel").on_press(Message::CancelEditSetup);

    let buttons = widget::row()
        .push(cancel_btn)
        .push(artwork_btn)
        .push(save_btn)
        .spacing(8)
        .align_y(Alignment::Center);

    container(
        widget::column()
            .push(form)
            .push(buttons)
            .spacing(16)
            .padding(24),
    )
    .width(Length::Fill)
    .class(cosmic::theme::Container::Custom(Box::new(theme::canvas_bg)))
    .into()
}

/// Renders the deploy status banner.
fn status_banner(status: &DeployStatus) -> Element<'_, Message> {
    let mut col = widget::column().spacing(6).padding(12);

    match status {
        DeployStatus::Deploying {
            setup_name,
            progress,
            status_msg,
        } => {
            let pct = (*progress * 100.0) as u32;
            let header = widget::row()
                .push(
                    widget::text(format!("Deploying {setup_name}... {pct}%"))
                        .class(theme::CYAN)
                        .width(Length::Fill),
                )
                .align_y(Alignment::Center);
            col = col.push(header);

            col = col.push(widget::progress_bar(0.0..=1.0, *progress as f32));

            if !status_msg.is_empty() {
                col = col.push(
                    widget::text::caption(status_msg).class(theme::MUTED_TEXT),
                );
            }
        }
        DeployStatus::Success {
            setup_name,
            app_id,
        } => {
            let row = widget::row()
                .push(
                    widget::text(format!("{setup_name} deployed (AppID: {app_id})"))
                        .class(theme::CONNECTED_COLOR)
                        .width(Length::Fill),
                )
                .push(
                    widget::button::standard("Dismiss")
                        .on_press(Message::DismissDeployStatus),
                )
                .spacing(12)
                .align_y(Alignment::Center);
            col = col.push(row);
        }
        DeployStatus::Failed { setup_name, error } => {
            let row = widget::row()
                .push(
                    widget::text(format!("{setup_name} failed: {error}"))
                        .class(ERROR_COLOR)
                        .width(Length::Fill),
                )
                .push(
                    widget::button::standard("Dismiss")
                        .on_press(Message::DismissDeployStatus),
                )
                .spacing(12)
                .align_y(Alignment::Center);
            col = col.push(row);
        }
    }

    container(col)
        .width(Length::Fill)
        .class(cosmic::theme::Container::Custom(Box::new(theme::canvas_bg)))
        .into()
}

/// Renders a labeled text input for a setup field.
fn form_field<'a>(
    label: &'a str,
    value: &'a str,
    field: SetupField,
) -> Element<'a, Message> {
    widget::column()
        .push(widget::text::caption(label).class(theme::MUTED_TEXT))
        .push(
            widget::text_input(label, value)
                .on_input(move |v| Message::UpdateSetupField(field.clone(), v)),
        )
        .spacing(4)
        .into()
}

const ERROR_COLOR: cosmic::iced::Color = cosmic::iced::Color::from_rgb(0.91, 0.30, 0.24);
