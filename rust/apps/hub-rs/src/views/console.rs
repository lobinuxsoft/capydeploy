//! Console Log viewer — streaming log entries with level filters.

use cosmic::iced::{Alignment, Color, Length};
use cosmic::widget::{self, container};
use cosmic::Element;

use capydeploy_hub_console_log::ConsoleLogHub;
use capydeploy_protocol::constants::{
    LOG_LEVEL_DEBUG, LOG_LEVEL_ERROR, LOG_LEVEL_INFO, LOG_LEVEL_LOG, LOG_LEVEL_WARN,
};

use crate::message::Message;
use crate::theme;

/// Renders the console log viewer.
pub fn view<'a>(
    hub: &'a ConsoleLogHub,
    agent_id: Option<&str>,
    level_filter: u32,
    search: &'a str,
) -> Element<'a, Message> {
    let mut content = widget::column().spacing(12);

    content = content.push(widget::text::title3("Console Log"));

    let agent = agent_id.and_then(|id| hub.get_agent(id));

    // -- Controls bar --
    let enabled = agent.is_some_and(|a| a.enabled());

    let toggle_label = if enabled { "Enabled" } else { "Disabled" };
    let toggle = widget::toggler(enabled)
        .label(toggle_label)
        .on_toggle(Message::ConsoleSetEnabled);

    let clear_btn = widget::button::standard("Clear").on_press(Message::ConsoleClear);

    let controls = widget::row()
        .push(toggle)
        .push(widget::Space::with_width(Length::Fill))
        .push(clear_btn)
        .align_y(Alignment::Center)
        .spacing(12);
    content = content.push(controls);

    // -- Level filter buttons --
    let filters = widget::row()
        .push(level_btn("LOG", LOG_LEVEL_LOG, level_filter, LOG_COLOR))
        .push(level_btn("WARN", LOG_LEVEL_WARN, level_filter, WARN_COLOR))
        .push(level_btn(
            "ERROR",
            LOG_LEVEL_ERROR,
            level_filter,
            ERROR_COLOR,
        ))
        .push(level_btn("INFO", LOG_LEVEL_INFO, level_filter, INFO_COLOR))
        .push(level_btn(
            "DEBUG",
            LOG_LEVEL_DEBUG,
            level_filter,
            DEBUG_COLOR,
        ))
        .spacing(6);
    content = content.push(filters);

    // -- Search input --
    let search_input = widget::text_input("Search logs...", search)
        .on_input(Message::ConsoleSearchInput);
    content = content.push(search_input);

    // -- Log entries --
    if let Some(agent) = agent {
        let search_lower = search.to_lowercase();

        let mut log_list = widget::column().spacing(2);
        let mut visible_count = 0u32;

        for entry in agent.entries().iter() {
            // Level filter.
            let level_bit = capydeploy_protocol::constants::log_level_bit(&entry.level);
            if level_bit & level_filter == 0 {
                continue;
            }

            // Text search filter.
            if !search_lower.is_empty() && !entry.text.to_lowercase().contains(&search_lower) {
                continue;
            }

            log_list = log_list.push(log_entry_row(entry));
            visible_count += 1;
        }

        let log_scroll = widget::scrollable(log_list)
            .width(Length::Fill)
            .height(Length::Fill);

        content = content.push(
            container(log_scroll)
                .width(Length::Fill)
                .height(Length::Fill)
                .class(cosmic::theme::Container::Custom(Box::new(
                    theme::canvas_bg,
                ))),
        );

        // -- Status bar --
        let total = agent.entries().len();
        let dropped = agent.total_dropped();

        let mut status_parts = vec![format!("{visible_count}/{total} entries")];
        if dropped > 0 {
            status_parts.push(format!("{dropped} dropped"));
        }

        content = content.push(
            widget::text::caption(status_parts.join(" | ")).class(theme::MUTED_TEXT),
        );
    } else {
        content = content.push(
            widget::text("Waiting for console log data...").class(theme::MUTED_TEXT),
        );
    }

    content.into()
}

/// Renders a single log entry row.
fn log_entry_row(entry: &capydeploy_protocol::console_log::ConsoleLogEntry) -> Element<'_, Message> {
    let color = color_for_level(&entry.level);

    let ts = format_timestamp(entry.timestamp);
    let level_tag = format!("[{}]", entry.level.to_uppercase());

    let row = widget::row()
        .push(widget::text::caption(ts).class(theme::MUTED_TEXT))
        .push(widget::text::caption(level_tag).class(color))
        .push(widget::text::caption(&entry.text))
        .spacing(8)
        .align_y(Alignment::Center);

    row.into()
}

/// Level filter toggle button.
fn level_btn<'a>(
    label: &'a str,
    bit: u32,
    current_mask: u32,
    active_color: Color,
) -> Element<'a, Message> {
    let is_active = current_mask & bit != 0;
    let color = if is_active {
        active_color
    } else {
        theme::MUTED_TEXT
    };

    widget::button::custom(widget::text::caption(label).class(color))
        .on_press(Message::ConsoleToggleLevel(bit))
        .into()
}

/// Maps log level string to display color.
fn color_for_level(level: &str) -> Color {
    match level {
        "error" => ERROR_COLOR,
        "warn" | "warning" => WARN_COLOR,
        "info" => INFO_COLOR,
        "debug" | "verbose" => DEBUG_COLOR,
        _ => LOG_COLOR,
    }
}

/// Formats a millisecond timestamp as HH:MM:SS.
fn format_timestamp(ms: i64) -> String {
    let secs = ms / 1000;
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

// Level colors.
const LOG_COLOR: Color = Color::from_rgb(0.75, 0.75, 0.75);
const WARN_COLOR: Color = Color::from_rgb(0.95, 0.77, 0.06);
const ERROR_COLOR: Color = Color::from_rgb(0.91, 0.30, 0.24);
const INFO_COLOR: Color = Color::from_rgb(0.30, 0.70, 1.0);
const DEBUG_COLOR: Color = Color::from_rgb(0.50, 0.50, 0.55);
