//! SteamGridDB artwork selector dialog.
//!
//! Allows searching for games, browsing artwork by type (capsule, wide,
//! hero, logo, icon), and selecting image URLs to assign to a game setup.

use std::collections::HashMap;

use cosmic::iced::{Alignment, Color, Length};
use cosmic::widget::{self, container};
use cosmic::Element;

use capydeploy_steamgriddb::types::{ImageData, SearchResult};

use crate::message::Message;
use crate::theme;

/// Artwork category tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtworkTab {
    Capsule,
    Wide,
    Hero,
    Logo,
    Icon,
}

impl ArtworkTab {
    pub const ALL: [ArtworkTab; 5] = [
        Self::Capsule,
        Self::Wide,
        Self::Hero,
        Self::Logo,
        Self::Icon,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Capsule => "Capsule",
            Self::Wide => "Wide",
            Self::Hero => "Hero",
            Self::Logo => "Logo",
            Self::Icon => "Icon",
        }
    }
}

/// The artwork selections resulting from the dialog.
#[derive(Debug, Clone, Default)]
pub struct ArtworkSelection {
    pub griddb_game_id: i32,
    pub grid_portrait: String,
    pub grid_landscape: String,
    pub hero_image: String,
    pub logo_image: String,
    pub icon_image: String,
}

/// Dialog state for the SteamGridDB artwork selector.
pub struct ArtworkDialog {
    pub search_query: String,
    pub search_results: Vec<SearchResult>,
    pub searching: bool,

    pub selected_game_id: Option<i32>,
    pub selected_game_name: String,

    pub active_tab: ArtworkTab,
    pub images: HashMap<ArtworkTab, Vec<ImageData>>,
    pub loading: HashMap<ArtworkTab, bool>,

    // Current selections (URLs).
    pub grid_portrait: String,
    pub grid_landscape: String,
    pub hero_image: String,
    pub logo_image: String,
    pub icon_image: String,
    pub griddb_game_id: i32,
}

impl ArtworkDialog {
    /// Creates a new artwork dialog, optionally pre-filled from existing setup values.
    pub fn new(
        game_name: &str,
        griddb_game_id: i32,
        grid_portrait: &str,
        grid_landscape: &str,
        hero_image: &str,
        logo_image: &str,
        icon_image: &str,
    ) -> Self {
        Self {
            search_query: game_name.to_string(),
            search_results: Vec::new(),
            searching: false,
            selected_game_id: if griddb_game_id > 0 {
                Some(griddb_game_id)
            } else {
                None
            },
            selected_game_name: game_name.to_string(),
            active_tab: ArtworkTab::Capsule,
            images: HashMap::new(),
            loading: HashMap::new(),
            grid_portrait: grid_portrait.to_string(),
            grid_landscape: grid_landscape.to_string(),
            hero_image: hero_image.to_string(),
            logo_image: logo_image.to_string(),
            icon_image: icon_image.to_string(),
            griddb_game_id,
        }
    }

    /// Builds the final selection from current state.
    pub fn selection(&self) -> ArtworkSelection {
        ArtworkSelection {
            griddb_game_id: self.griddb_game_id,
            grid_portrait: self.grid_portrait.clone(),
            grid_landscape: self.grid_landscape.clone(),
            hero_image: self.hero_image.clone(),
            logo_image: self.logo_image.clone(),
            icon_image: self.icon_image.clone(),
        }
    }

    /// Sets the selected URL for the current tab.
    pub fn select_image(&mut self, tab: ArtworkTab, url: &str) {
        match tab {
            ArtworkTab::Capsule => self.grid_portrait = url.to_string(),
            ArtworkTab::Wide => self.grid_landscape = url.to_string(),
            ArtworkTab::Hero => self.hero_image = url.to_string(),
            ArtworkTab::Logo => self.logo_image = url.to_string(),
            ArtworkTab::Icon => self.icon_image = url.to_string(),
        }
    }

    /// Returns the currently selected URL for a tab.
    fn selected_url(&self, tab: ArtworkTab) -> &str {
        match tab {
            ArtworkTab::Capsule => &self.grid_portrait,
            ArtworkTab::Wide => &self.grid_landscape,
            ArtworkTab::Hero => &self.hero_image,
            ArtworkTab::Logo => &self.logo_image,
            ArtworkTab::Icon => &self.icon_image,
        }
    }

    /// Count of non-empty selections.
    fn selection_count(&self) -> usize {
        [
            &self.grid_portrait,
            &self.grid_landscape,
            &self.hero_image,
            &self.logo_image,
            &self.icon_image,
        ]
        .iter()
        .filter(|s| !s.is_empty())
        .count()
    }
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

/// Renders the artwork selector dialog as a modal overlay.
pub fn view(dialog: &ArtworkDialog) -> Element<'_, Message> {
    let mut content = widget::column().spacing(16).padding(24);

    // -- Header --
    content = content.push(
        widget::row()
            .push(widget::text::title3("Artwork Selector").width(Length::Fill))
            .push(
                widget::button::standard("Close").on_press(Message::ArtworkCancel),
            )
            .align_y(Alignment::Center),
    );

    // -- Search bar --
    let search_input = widget::text_input("Search game...", &dialog.search_query)
        .on_input(Message::ArtworkSearchInput)
        .on_submit(|_| Message::ArtworkSearchSubmit);

    let search_btn = if dialog.searching {
        widget::button::standard("Searching...")
    } else {
        widget::button::suggested("Search").on_press(Message::ArtworkSearchSubmit)
    };

    content = content.push(
        widget::row()
            .push(search_input.width(Length::Fill))
            .push(search_btn)
            .spacing(8)
            .align_y(Alignment::Center),
    );

    // -- Main area: split left (search results) + right (images + selection) --
    let left_panel = search_results_panel(dialog);
    let right_panel = images_panel(dialog);

    content = content.push(
        widget::row()
            .push(left_panel)
            .push(right_panel)
            .spacing(16)
            .height(Length::Fixed(480.0)),
    );

    // -- Footer: selection summary + buttons --
    let count = dialog.selection_count();
    let summary_text = if count > 0 {
        format!("{count} artwork selected")
    } else {
        "No artwork selected".to_string()
    };

    let clear_btn = if count > 0 {
        widget::button::destructive("Clear All").on_press(Message::ArtworkClearAll)
    } else {
        widget::button::destructive("Clear All")
    };

    let save_btn = widget::button::suggested("Save Selection")
        .on_press(Message::ArtworkSave);

    content = content.push(
        widget::row()
            .push(
                widget::text(summary_text)
                    .class(theme::MUTED_TEXT)
                    .width(Length::Fill),
            )
            .push(clear_btn)
            .push(save_btn)
            .spacing(8)
            .align_y(Alignment::Center),
    );

    // Wrap in a dark backdrop.
    let dialog_box = container(content)
        .width(Length::Fixed(900.0))
        .class(cosmic::theme::Container::Custom(Box::new(dialog_bg)));

    // Center the dialog in the overlay.
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

/// Left panel: game search results.
fn search_results_panel(dialog: &ArtworkDialog) -> Element<'_, Message> {
    let mut panel = widget::column().spacing(4);

    panel = panel.push(
        widget::text::caption("Games").class(theme::MUTED_TEXT),
    );

    if dialog.search_results.is_empty() {
        let hint = if dialog.searching {
            "Searching..."
        } else if dialog.search_query.is_empty() {
            "Type a game name to search"
        } else {
            "No results found"
        };
        panel = panel.push(
            widget::text::caption(hint)
                .class(theme::MUTED_TEXT)
                .width(Length::Fill),
        );
    } else {
        let mut list = widget::column().spacing(2);
        for result in &dialog.search_results {
            let is_selected = dialog.selected_game_id == Some(result.id);
            let color = if is_selected {
                theme::CYAN
            } else {
                Color::WHITE
            };
            let label = widget::text::caption(&result.name).class(color);
            let btn = widget::button::custom(label)
                .on_press(Message::ArtworkSelectGame(
                    result.id,
                    result.name.clone(),
                ))
                .width(Length::Fill);
            list = list.push(btn);
        }
        panel = panel.push(
            widget::scrollable(list)
                .width(Length::Fill)
                .height(Length::Fill),
        );
    }

    container(panel)
        .width(Length::Fixed(220.0))
        .height(Length::Fill)
        .padding(8)
        .class(cosmic::theme::Container::Custom(Box::new(theme::canvas_bg)))
        .into()
}

/// Right panel: artwork tabs + image list + current selections.
fn images_panel(dialog: &ArtworkDialog) -> Element<'_, Message> {
    let mut panel = widget::column().spacing(8);

    // -- Tab bar --
    let mut tabs = widget::row().spacing(4);
    for tab in ArtworkTab::ALL {
        let is_active = dialog.active_tab == tab;
        let has_selection = !dialog.selected_url(tab).is_empty();

        let label_text = if has_selection {
            format!("{} *", tab.label())
        } else {
            tab.label().to_string()
        };

        let color = if is_active {
            theme::CYAN
        } else {
            theme::MUTED_TEXT
        };

        let btn = widget::button::custom(
            widget::text::caption(label_text).class(color),
        )
        .on_press(Message::ArtworkTabChanged(tab));

        tabs = tabs.push(btn);
    }
    panel = panel.push(tabs);

    // -- Current selection for active tab --
    let selected_url = dialog.selected_url(dialog.active_tab);
    if !selected_url.is_empty() {
        panel = panel.push(
            widget::row()
                .push(widget::text::caption("Selected: ").class(theme::CONNECTED_COLOR))
                .push(
                    widget::text::caption(truncate_url(selected_url, 60))
                        .class(theme::MUTED_TEXT),
                )
                .spacing(4)
                .align_y(Alignment::Center),
        );
    }

    // -- Image list --
    if dialog.selected_game_id.is_none() {
        panel = panel.push(
            widget::text("Select a game to browse artwork")
                .class(theme::MUTED_TEXT)
                .width(Length::Fill),
        );
    } else {
        let is_loading = dialog
            .loading
            .get(&dialog.active_tab)
            .copied()
            .unwrap_or(false);

        if is_loading {
            panel = panel.push(widget::text("Loading images...").class(theme::MUTED_TEXT));
        } else if let Some(images) = dialog.images.get(&dialog.active_tab) {
            if images.is_empty() {
                panel = panel.push(
                    widget::text("No images found for this type")
                        .class(theme::MUTED_TEXT),
                );
            } else {
                let mut list = widget::column().spacing(2);
                for img in images {
                    list = list.push(image_row(img, dialog.active_tab, selected_url));
                }
                panel = panel.push(
                    widget::scrollable(list)
                        .width(Length::Fill)
                        .height(Length::Fill),
                );
            }
        } else {
            panel = panel.push(
                widget::text("Loading...")
                    .class(theme::MUTED_TEXT),
            );
        }
    }

    container(panel)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8)
        .class(cosmic::theme::Container::Custom(Box::new(theme::canvas_bg)))
        .into()
}

/// A single image row with metadata and select button.
fn image_row<'a>(
    img: &'a ImageData,
    tab: ArtworkTab,
    selected_url: &'a str,
) -> Element<'a, Message> {
    let is_selected = img.url == selected_url;

    let mut info = widget::row().spacing(8).align_y(Alignment::Center);

    // Dimensions badge.
    info = info.push(
        widget::text::caption(format!("{}x{}", img.width, img.height))
            .class(theme::MUTED_TEXT)
            .width(Length::Fixed(90.0)),
    );

    // Style badge.
    if !img.style.is_empty() {
        info = info.push(
            widget::text::caption(img.style.clone())
                .class(STYLE_COLOR)
                .width(Length::Fixed(80.0)),
        );
    }

    // Score.
    if img.score > 0 {
        info = info.push(
            widget::text::caption(format!("score:{}", img.score))
                .class(theme::MUTED_TEXT),
        );
    }

    // URL preview (truncated).
    info = info.push(
        widget::text::caption(truncate_url(&img.url, 40))
            .class(theme::MUTED_TEXT)
            .width(Length::Fill),
    );

    // Select/Selected button.
    if is_selected {
        info = info.push(
            widget::text::caption("Selected")
                .class(theme::CONNECTED_COLOR),
        );
    } else {
        info = info.push(
            widget::button::custom(
                widget::text::caption("Select").class(theme::CYAN),
            )
            .on_press(Message::ArtworkSelectImage(tab, img.url.clone())),
        );
    }

    let row_style = if is_selected {
        ROW_SELECTED_STYLE
    } else {
        ROW_NORMAL_STYLE
    };

    container(info.padding([4, 8]))
        .width(Length::Fill)
        .class(cosmic::theme::Container::Custom(Box::new(row_style)))
        .into()
}

/// Truncate a URL for display.
fn truncate_url(url: &str, max: usize) -> String {
    if url.len() <= max {
        url.to_string()
    } else {
        format!("...{}", &url[url.len() - max..])
    }
}

// Colors.
const STYLE_COLOR: Color = Color::from_rgb(0.65, 0.55, 0.85);

/// Row style for normal (unselected) image rows.
fn row_normal_style(_theme: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    cosmic::iced::widget::container::Style {
        background: Some(cosmic::iced::Background::Color(Color::from_rgba(
            1.0, 1.0, 1.0, 0.03,
        ))),
        border: cosmic::iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Row style for selected image rows.
fn row_selected_style(_theme: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    cosmic::iced::widget::container::Style {
        background: Some(cosmic::iced::Background::Color(Color::from_rgba(
            1.0, 1.0, 1.0, 0.03,
        ))),
        border: cosmic::iced::Border {
            color: Color::from_rgb(0.18, 0.80, 0.44),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

const ROW_NORMAL_STYLE: fn(&cosmic::Theme) -> cosmic::iced::widget::container::Style =
    row_normal_style;
const ROW_SELECTED_STYLE: fn(&cosmic::Theme) -> cosmic::iced::widget::container::Style =
    row_selected_style;

/// Dialog background.
fn dialog_bg(theme: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    let _ = theme;
    cosmic::iced::widget::container::Style {
        background: Some(cosmic::iced::Background::Color(Color::from_rgb(
            0.12, 0.12, 0.14,
        ))),
        border: cosmic::iced::Border {
            color: Color::from_rgb(0.25, 0.25, 0.30),
            width: 1.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    }
}

/// Backdrop overlay.
fn backdrop_bg(theme: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    let _ = theme;
    cosmic::iced::widget::container::Style {
        background: Some(cosmic::iced::Background::Color(Color::from_rgba(
            0.0, 0.0, 0.0, 0.6,
        ))),
        ..Default::default()
    }
}
