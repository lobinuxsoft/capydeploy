//! Artwork selector view functions.
//!
//! Renders the dialog layout: search panel, image grid with filters,
//! thumbnail cards, and selections preview panel.

use std::collections::HashMap;

use cosmic::iced::{Alignment, Color, Length};
use cosmic::widget::{self, container};
use cosmic::Element;

use capydeploy_steamgriddb::types::ImageData;

use crate::message::{ArtworkFilterField, Message};
use crate::theme;

use super::{ArtworkDialog, ArtworkTab};

// ---------------------------------------------------------------------------
// Main dialog view
// ---------------------------------------------------------------------------

/// Renders the artwork selector dialog as a modal overlay.
pub fn view(dialog: &ArtworkDialog) -> Element<'_, Message> {
    let mut content = widget::column().spacing(16).padding(24);

    // -- Header --
    content = content.push(
        widget::row()
            .push(widget::text::title3("Artwork Selector").width(Length::Fill))
            .push(widget::button::standard("Close").on_press(Message::ArtworkCancel))
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

    // -- Main area: games | images | selections --
    content = content.push(
        widget::row()
            .push(search_results_panel(dialog))
            .push(images_panel(dialog))
            .push(selections_panel(dialog))
            .spacing(12)
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

    let save_btn = widget::button::suggested("Save Selection").on_press(Message::ArtworkSave);

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
        .width(Length::Fixed(1060.0))
        .class(cosmic::theme::Container::Custom(Box::new(dialog_bg)));

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

// ---------------------------------------------------------------------------
// Panels
// ---------------------------------------------------------------------------

/// Left panel: game search results.
fn search_results_panel(dialog: &ArtworkDialog) -> Element<'_, Message> {
    let mut panel = widget::column().spacing(4);

    panel = panel.push(widget::text::caption("Games").class(theme::MUTED_TEXT));

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

/// Center panel: artwork tabs + filters + thumbnail grid.
fn images_panel(dialog: &ArtworkDialog) -> Element<'_, Message> {
    let mut panel = widget::column().spacing(8);

    // -- Tab bar + Local button --
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

        let btn = widget::button::custom(widget::text::caption(label_text).class(color))
            .on_press(Message::ArtworkTabChanged(tab));

        tabs = tabs.push(btn);
    }

    let local_btn = widget::button::standard("Local").on_press(Message::ArtworkSelectLocalFile);
    tabs = tabs.push(widget::horizontal_space()).push(local_btn);

    panel = panel.push(tabs);

    // -- Filters button --
    if dialog.selected_game_id.is_some() {
        let filter_label = if dialog.has_active_filters() {
            "Filters (active)"
        } else {
            "Filters"
        };
        let filter_btn_color = if dialog.has_active_filters() {
            FILTER_ACTIVE_COLOR
        } else {
            theme::MUTED_TEXT
        };
        panel = panel.push(
            widget::button::custom(
                widget::text::caption(filter_label).class(filter_btn_color),
            )
            .on_press(Message::ArtworkShowFilters),
        );
    }

    // -- Current selection for active tab --
    let selected_url = dialog.selected_url(dialog.active_tab);
    if !selected_url.is_empty() {
        let is_local = selected_url.starts_with("file://");
        let label_color = if is_local {
            theme::CYAN
        } else {
            theme::CONNECTED_COLOR
        };
        let badge = if is_local { "LOCAL: " } else { "Selected: " };

        panel = panel.push(
            widget::row()
                .push(widget::text::caption(badge).class(label_color))
                .push(
                    widget::text::caption(truncate_url(selected_url, 55))
                        .class(theme::MUTED_TEXT),
                )
                .spacing(4)
                .align_y(Alignment::Center),
        );
    }

    // -- Filter panel (shown when toggled) --
    if dialog.show_filters && dialog.selected_game_id.is_some() {
        panel = panel.push(
            widget::scrollable(filters_panel(dialog))
                .width(Length::Fill)
                .height(Length::Fill),
        );

        return container(panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8)
            .class(cosmic::theme::Container::Custom(Box::new(theme::canvas_bg)))
            .into();
    }

    // -- Image grid --
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
                    widget::text("No images found for this type").class(theme::MUTED_TEXT),
                );
            } else {
                let grid = thumbnail_grid(images, dialog);

                let load_more = widget::button::standard("Load More")
                    .on_press(Message::ArtworkLoadMore)
                    .width(Length::Fill);

                let scroll_content = widget::column().push(grid).push(load_more).spacing(8);

                panel = panel.push(
                    widget::scrollable(scroll_content)
                        .width(Length::Fill)
                        .height(Length::Fill),
                );
            }
        } else {
            panel = panel.push(widget::text("Loading...").class(theme::MUTED_TEXT));
        }
    }

    container(panel)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8)
        .class(cosmic::theme::Container::Custom(Box::new(theme::canvas_bg)))
        .into()
}

/// Right panel: current artwork selections with small image previews.
fn selections_panel(dialog: &ArtworkDialog) -> Element<'_, Message> {
    let mut panel = widget::column().spacing(8);

    panel = panel.push(widget::text::caption("Selected Artwork").class(theme::CYAN));

    let slots: [(ArtworkTab, &str, &str, f32, f32); 5] = [
        (ArtworkTab::Capsule, "Capsule", &dialog.grid_portrait, 40.0, 56.0),
        (ArtworkTab::Wide, "Wide", &dialog.grid_landscape, 70.0, 40.0),
        (ArtworkTab::Hero, "Hero", &dialog.hero_image, 80.0, 32.0),
        (ArtworkTab::Logo, "Logo", &dialog.logo_image, 50.0, 40.0),
        (ArtworkTab::Icon, "Icon", &dialog.icon_image, 40.0, 40.0),
    ];

    for (tab, label, url, pw, ph) in slots {
        let is_active = dialog.active_tab == tab;
        let label_color = if is_active {
            theme::CYAN
        } else {
            theme::MUTED_TEXT
        };

        let mut row = widget::row().spacing(8).align_y(Alignment::Center);
        row = row.push(
            widget::text::caption(label)
                .class(label_color)
                .width(Length::Fixed(52.0)),
        );

        if url.is_empty() {
            row = row.push(
                container(widget::text::caption("\u{2014}").class(theme::MUTED_TEXT))
                    .width(Length::Fixed(pw))
                    .height(Length::Fixed(ph))
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .class(cosmic::theme::Container::Custom(Box::new(placeholder_dashed))),
            );
        } else if let Some(handle) = dialog.preview_handle(url) {
            let mut img_col = widget::column().spacing(0);
            img_col = img_col.push(
                container(
                    widget::image(handle.clone())
                        .width(Length::Fill)
                        .height(Length::Fixed(ph)),
                )
                .width(Length::Fixed(pw))
                .height(Length::Fixed(ph))
                .class(cosmic::theme::Container::Custom(Box::new(preview_border))),
            );
            if url.starts_with("file://") {
                img_col = img_col.push(widget::text::caption("LOCAL").class(theme::CYAN));
            }
            row = row.push(img_col);
        } else {
            row = row.push(
                widget::text::caption(truncate_url(url, 12))
                    .class(theme::MUTED_TEXT)
                    .width(Length::Fixed(pw)),
            );
        }

        let tab_btn = widget::button::custom(row).on_press(Message::ArtworkTabChanged(tab));
        panel = panel.push(tab_btn);
    }

    container(panel)
        .width(Length::Fixed(160.0))
        .height(Length::Fill)
        .padding(8)
        .class(cosmic::theme::Container::Custom(Box::new(theme::canvas_bg)))
        .into()
}

// ---------------------------------------------------------------------------
// Filters
// ---------------------------------------------------------------------------

/// Full filter panel with sections for styles, dimensions, formats, types, tags.
fn filters_panel(dialog: &ArtworkDialog) -> Element<'_, Message> {
    let tab = dialog.active_tab;
    let mut panel = widget::column().spacing(16).padding(8);

    // -- Styles section --
    let styles = tab.style_options();
    if !styles.is_empty() {
        panel = panel.push(filter_section(
            "Styles",
            styles
                .iter()
                .map(|&s| {
                    let label = s.replace('_', " ");
                    filter_chip(
                        &label,
                        dialog.is_filter_selected(&ArtworkFilterField::Style, s),
                        Message::ArtworkToggleFilter(ArtworkFilterField::Style, s.to_string()),
                    )
                })
                .collect(),
        ));
    }

    // -- Dimensions section --
    let dims = tab.dimension_options();
    if !dims.is_empty() {
        panel = panel.push(filter_section(
            "Dimensions",
            dims.iter()
                .map(|&d| {
                    let label = d.replace('x', "\u{00d7}");
                    filter_chip(
                        &label,
                        dialog.is_filter_selected(&ArtworkFilterField::Dimension, d),
                        Message::ArtworkToggleFilter(
                            ArtworkFilterField::Dimension,
                            d.to_string(),
                        ),
                    )
                })
                .collect(),
        ));
    }

    // -- File Types section --
    let mimes = tab.mime_options();
    if !mimes.is_empty() {
        panel = panel.push(filter_section(
            "File Types",
            mimes
                .iter()
                .map(|&m| {
                    let label = m
                        .replace("image/", "")
                        .replace("vnd.microsoft.", "")
                        .to_uppercase();
                    filter_chip(
                        &label,
                        dialog.is_filter_selected(&ArtworkFilterField::MimeType, m),
                        Message::ArtworkToggleFilter(ArtworkFilterField::MimeType, m.to_string()),
                    )
                })
                .collect(),
        ));
    }

    // -- Types section (Animated / Static) --
    panel = panel.push(filter_section(
        "Types",
        vec![
            filter_chip(
                "Static",
                dialog.is_filter_selected(&ArtworkFilterField::ImageType, "Static Only"),
                Message::ArtworkToggleFilter(
                    ArtworkFilterField::ImageType,
                    "Static Only".to_string(),
                ),
            ),
            filter_chip(
                "Animated",
                dialog.is_filter_selected(&ArtworkFilterField::ImageType, "Animated Only"),
                Message::ArtworkToggleFilter(
                    ArtworkFilterField::ImageType,
                    "Animated Only".to_string(),
                ),
            ),
        ],
    ));

    // -- Tags section (NSFW / Humor) --
    panel = panel.push(filter_section(
        "Tags",
        vec![
            filter_chip("Adult Content", dialog.filters.show_nsfw, Message::ArtworkToggleNsfw),
            filter_chip("Humor", dialog.filters.show_humor, Message::ArtworkToggleHumor),
        ],
    ));

    // -- Actions: Reset + Close --
    let mut actions = widget::row().spacing(8).align_y(Alignment::Center);

    if dialog.has_active_filters() {
        actions = actions.push(
            widget::button::custom(
                widget::text::caption("Reset").class(FILTER_ACTIVE_COLOR),
            )
            .on_press(Message::ArtworkResetFilters),
        );
    }

    actions = actions.push(widget::horizontal_space());
    actions = actions.push(
        widget::button::suggested("Close Filters").on_press(Message::ArtworkShowFilters),
    );

    panel = panel.push(actions);

    panel.into()
}

/// A labeled section with a title and a row of filter chips.
fn filter_section<'a>(
    title: &str,
    chips: Vec<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut section = widget::column().spacing(6);
    section = section.push(widget::text::caption(title.to_string()).class(theme::MUTED_TEXT));

    let mut row = widget::row().spacing(6);
    for chip in chips {
        row = row.push(chip);
    }
    section = section.push(row);

    section.into()
}

/// A single toggle-style filter chip button.
fn filter_chip<'a>(label: &str, selected: bool, on_press: Message) -> Element<'a, Message> {
    let (fg, bg_color) = if selected {
        (
            Color::WHITE,
            Color::from_rgba(0.02, 0.71, 0.83, 0.25),
        )
    } else {
        (
            theme::MUTED_TEXT,
            Color::from_rgba(1.0, 1.0, 1.0, 0.05),
        )
    };

    let label_widget = widget::text::caption(label.to_string()).class(fg);

    widget::button::custom(
        container(label_widget)
            .padding([4, 10])
            .class(cosmic::theme::Container::Custom(Box::new(
                move |_theme: &cosmic::Theme| cosmic::iced::widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(bg_color)),
                    border: cosmic::iced::Border {
                        color: if selected {
                            Color::from_rgba(0.02, 0.71, 0.83, 0.5)
                        } else {
                            Color::TRANSPARENT
                        },
                        width: 1.0,
                        radius: 16.0.into(),
                    },
                    ..Default::default()
                },
            ))),
    )
    .on_press(on_press)
    .into()
}

// ---------------------------------------------------------------------------
// Thumbnail grid
// ---------------------------------------------------------------------------

/// Renders a responsive grid of thumbnail cards.
fn thumbnail_grid<'a>(images: &'a [ImageData], dialog: &'a ArtworkDialog) -> Element<'a, Message> {
    let tab = dialog.active_tab;
    let selected_url = dialog.selected_url(tab);
    let card_w = tab.card_width();
    let thumb_h = tab.thumb_height();

    let cards: Vec<Element<'a, Message>> = images
        .iter()
        .map(|img| thumbnail_card(img, tab, selected_url, &dialog.thumb_cache, card_w, thumb_h))
        .collect();

    widget::flex_row(cards)
        .column_spacing(8)
        .row_spacing(8)
        .into()
}

/// A single thumbnail card with image preview, metadata overlay, and click-to-select.
fn thumbnail_card<'a>(
    img: &'a ImageData,
    tab: ArtworkTab,
    selected_url: &'a str,
    thumb_cache: &'a HashMap<String, cosmic::iced::widget::image::Handle>,
    card_w: f32,
    thumb_h: f32,
) -> Element<'a, Message> {
    let is_selected = img.url == selected_url;

    let thumb_url = if !img.thumb.is_empty() {
        &img.thumb
    } else {
        &img.url
    };

    let image_content: Element<'a, Message> = if let Some(handle) = thumb_cache.get(thumb_url) {
        container(
            widget::image(handle.clone())
                .width(Length::Fill)
                .height(Length::Fixed(thumb_h)),
        )
        .width(Length::Fixed(card_w))
        .height(Length::Fixed(thumb_h))
        .into()
    } else {
        container(widget::text::caption("...").class(theme::MUTED_TEXT))
            .width(Length::Fixed(card_w))
            .height(Length::Fixed(thumb_h))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .class(cosmic::theme::Container::Custom(Box::new(placeholder_bg)))
            .into()
    };

    // Metadata: dimensions + style.
    let mut meta = widget::row().spacing(4).align_y(Alignment::Center);
    meta = meta.push(
        widget::text::caption(format!("{}x{}", img.width, img.height)).class(theme::MUTED_TEXT),
    );
    if !img.style.is_empty() {
        meta = meta.push(widget::text::caption(&img.style).class(STYLE_COLOR));
    }

    let card_content = widget::column()
        .push(image_content)
        .push(meta)
        .spacing(2)
        .width(Length::Fixed(card_w));

    let card_style = if is_selected {
        CARD_SELECTED_STYLE
    } else {
        CARD_NORMAL_STYLE
    };

    let card = container(card_content.padding(4))
        .class(cosmic::theme::Container::Custom(Box::new(card_style)));

    widget::button::custom(card)
        .on_press(Message::ArtworkSelectImage(tab, img.url.clone()))
        .into()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Truncate a URL for display.
pub fn truncate_url(url: &str, max: usize) -> String {
    if url.len() <= max {
        url.to_string()
    } else {
        format!("...{}", &url[url.len() - max..])
    }
}

// ---------------------------------------------------------------------------
// Colors & container styles
// ---------------------------------------------------------------------------

const STYLE_COLOR: Color = Color::from_rgb(0.65, 0.55, 0.85);
const FILTER_ACTIVE_COLOR: Color = Color::from_rgb(1.0, 0.55, 0.2);

const CARD_NORMAL_STYLE: fn(&cosmic::Theme) -> cosmic::iced::widget::container::Style =
    |_theme| cosmic::iced::widget::container::Style {
        background: Some(cosmic::iced::Background::Color(Color::from_rgba(
            1.0, 1.0, 1.0, 0.03,
        ))),
        border: cosmic::iced::Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..Default::default()
    };

const CARD_SELECTED_STYLE: fn(&cosmic::Theme) -> cosmic::iced::widget::container::Style =
    |_theme| cosmic::iced::widget::container::Style {
        background: Some(cosmic::iced::Background::Color(Color::from_rgba(
            1.0, 1.0, 1.0, 0.03,
        ))),
        border: cosmic::iced::Border {
            color: Color::from_rgb(0.063, 0.725, 0.506), // CONNECTED_COLOR
            width: 2.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    };

fn placeholder_dashed(_theme: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    cosmic::iced::widget::container::Style {
        background: None,
        border: cosmic::iced::Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.15),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

fn preview_border(_theme: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    cosmic::iced::widget::container::Style {
        border: cosmic::iced::Border {
            color: Color::from_rgb(0.063, 0.725, 0.506), // CONNECTED_COLOR
            width: 2.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

fn placeholder_bg(_theme: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    cosmic::iced::widget::container::Style {
        background: Some(cosmic::iced::Background::Color(Color::from_rgba(
            1.0, 1.0, 1.0, 0.05,
        ))),
        border: cosmic::iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn dialog_bg(_theme: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    cosmic::iced::widget::container::Style {
        background: Some(cosmic::iced::Background::Color(theme::DARK_BG)),
        border: cosmic::iced::Border {
            color: Color::from_rgba(0.278, 0.333, 0.412, 0.5),
            width: 1.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    }
}

fn backdrop_bg(_theme: &cosmic::Theme) -> cosmic::iced::widget::container::Style {
    cosmic::iced::widget::container::Style {
        background: Some(cosmic::iced::Background::Color(Color::from_rgba(
            0.0, 0.0, 0.0, 0.6,
        ))),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_url_short_unchanged() {
        assert_eq!(truncate_url("https://img.png", 60), "https://img.png");
    }

    #[test]
    fn truncate_url_long_truncated() {
        let long = "https://cdn.steamgriddb.com/images/very/long/path/to/image.png";
        let result = truncate_url(long, 20);
        assert!(result.starts_with("..."));
        assert_eq!(result.len(), 23);
    }
}
