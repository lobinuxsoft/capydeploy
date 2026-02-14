//! Installed Games view — list games on the connected agent with delete actions.

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{self, container};
use cosmic::Element;

use capydeploy_hub_games::InstalledGame;

use crate::message::Message;
use crate::theme;

/// Renders the installed games view.
pub fn view<'a>(
    games: &'a [InstalledGame],
    loading: bool,
) -> Element<'a, Message> {
    let mut content = widget::column().spacing(16);

    content = content.push(widget::text::title3("Installed Games"));

    // Refresh button.
    let refresh_btn = if loading {
        widget::button::standard("Refreshing...")
    } else {
        widget::button::suggested("Refresh").on_press(Message::RefreshGames)
    };
    content = content.push(refresh_btn);

    if loading {
        content = content.push(
            widget::text("Fetching installed games from agent...").class(theme::MUTED_TEXT),
        );
    } else if games.is_empty() {
        content = content.push(
            container(
                widget::column()
                    .push(widget::text::heading("No games installed"))
                    .push(
                        widget::text(
                            "Deploy a game setup to install games on the connected agent.",
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
        content = content.push(
            widget::text::caption(format!("{} game(s) installed", games.len()))
                .class(theme::MUTED_TEXT),
        );

        for game in games {
            content = content.push(game_card(game));
        }
    }

    widget::scrollable(content).into()
}

/// Renders a card for a single installed game.
fn game_card(game: &InstalledGame) -> Element<'_, Message> {
    let info = widget::column()
        .push(widget::text::heading(&game.name))
        .push(
            widget::text::caption(format!("AppID: {}", game.app_id))
                .class(theme::MUTED_TEXT),
        )
        .push(widget::text::caption(&game.path).class(theme::MUTED_TEXT))
        .spacing(2);

    let delete_btn = widget::button::destructive("Delete")
        .on_press(Message::DeleteGame(game.app_id));

    let row = widget::row()
        .push(info.width(Length::Fill))
        .push(delete_btn)
        .spacing(16)
        .align_y(Alignment::Center)
        .padding(16);

    container(row)
        .width(Length::Fill)
        .class(cosmic::theme::Container::Custom(Box::new(theme::canvas_bg)))
        .into()
}
