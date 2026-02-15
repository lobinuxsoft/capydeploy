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
    source_filter: &'a str,
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

    // -- Source filter buttons --
    let sources = ["All", "console", "network", "game"];
    let mut source_row = widget::row().spacing(6);
    for src in sources {
        let is_active = if src == "All" {
            source_filter.is_empty()
        } else {
            source_filter == src
        };
        let color = if is_active {
            theme::CYAN
        } else {
            theme::MUTED_TEXT
        };
        let filter_val = if src == "All" {
            String::new()
        } else {
            src.to_string()
        };
        source_row = source_row.push(
            widget::button::custom(widget::text::caption(src).class(color))
                .on_press(Message::ConsoleSourceFilter(filter_val)),
        );
    }
    content = content.push(source_row);

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

            // Source filter.
            if !source_filter.is_empty() && entry.source != source_filter {
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

    let mut row = widget::row()
        .push(widget::text::caption(ts).class(theme::MUTED_TEXT))
        .push(widget::text::caption(level_tag).class(color))
        .spacing(8)
        .align_y(Alignment::Center);

    if !entry.source.is_empty() {
        row = row.push(
            widget::text::caption(format!("[{}]", entry.source))
                .class(source_color(&entry.source)),
        );
    }

    row = row.push(widget::text::caption(&entry.text));

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
const INFO_COLOR: Color = Color::from_rgb(0.024, 0.714, 0.831);
const DEBUG_COLOR: Color = Color::from_rgb(0.580, 0.639, 0.722);

// Source colors.
const SOURCE_CONSOLE_COLOR: Color = Color::from_rgb(0.55, 0.75, 0.55);
const SOURCE_NETWORK_COLOR: Color = Color::from_rgb(0.55, 0.65, 0.85);
const SOURCE_GAME_COLOR: Color = Color::from_rgb(0.85, 0.65, 0.45);

/// Maps log source string to display color.
fn source_color(source: &str) -> Color {
    match source {
        "console" => SOURCE_CONSOLE_COLOR,
        "network" => SOURCE_NETWORK_COLOR,
        "game" => SOURCE_GAME_COLOR,
        _ => theme::MUTED_TEXT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_timestamp_zero() {
        assert_eq!(format_timestamp(0), "00:00:00");
    }

    #[test]
    fn format_timestamp_seconds() {
        // 45 seconds = 45_000 ms.
        assert_eq!(format_timestamp(45_000), "00:00:45");
    }

    #[test]
    fn format_timestamp_minutes() {
        // 5 minutes 30 seconds = 330_000 ms.
        assert_eq!(format_timestamp(330_000), "00:05:30");
    }

    #[test]
    fn format_timestamp_hours() {
        // 2 hours 15 minutes 10 seconds.
        let ms = (2 * 3600 + 15 * 60 + 10) * 1000;
        assert_eq!(format_timestamp(ms), "02:15:10");
    }

    #[test]
    fn format_timestamp_wraps_at_24h() {
        // 25 hours should wrap to 01:00:00.
        let ms = 25 * 3600 * 1000;
        assert_eq!(format_timestamp(ms), "01:00:00");
    }

    #[test]
    fn color_for_level_known_levels() {
        assert_eq!(color_for_level("error"), ERROR_COLOR);
        assert_eq!(color_for_level("warn"), WARN_COLOR);
        assert_eq!(color_for_level("warning"), WARN_COLOR);
        assert_eq!(color_for_level("info"), INFO_COLOR);
        assert_eq!(color_for_level("debug"), DEBUG_COLOR);
        assert_eq!(color_for_level("verbose"), DEBUG_COLOR);
    }

    #[test]
    fn color_for_level_unknown_falls_to_log() {
        assert_eq!(color_for_level("log"), LOG_COLOR);
        assert_eq!(color_for_level("trace"), LOG_COLOR);
        assert_eq!(color_for_level("unknown"), LOG_COLOR);
    }

    #[test]
    fn source_color_known_sources() {
        assert_eq!(source_color("console"), SOURCE_CONSOLE_COLOR);
        assert_eq!(source_color("network"), SOURCE_NETWORK_COLOR);
        assert_eq!(source_color("game"), SOURCE_GAME_COLOR);
    }

    #[test]
    fn source_color_unknown_falls_to_muted() {
        assert_eq!(source_color("other"), theme::MUTED_TEXT);
        assert_eq!(source_color(""), theme::MUTED_TEXT);
    }
}
