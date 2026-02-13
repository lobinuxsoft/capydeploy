//! CDP WebSocket connection for console log streaming.
//!
//! Connects to Steam's CEF debugger and subscribes to `Runtime.consoleAPICalled`
//! and `Log.entryAdded` events. Parses `%c` CSS styled segments.

use std::time::Duration;

use capydeploy_protocol::console_log::{ConsoleLogEntry, StyledSegment};
use capydeploy_protocol::constants::log_level_bit;
use capydeploy_steam::cef::CefClient;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;

/// Timeout for the WebSocket handshake.
const WS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// A raw CDP event received from the browser.
#[derive(Debug, Deserialize)]
struct CdpEvent {
    method: Option<String>,
    params: Option<serde_json::Value>,
}

/// CDP message sent to the browser.
#[derive(serde::Serialize)]
struct CdpCommand {
    id: i32,
    method: &'static str,
    params: serde_json::Value,
}

/// `Runtime.consoleAPICalled` parameters.
#[derive(Deserialize)]
struct ConsoleApiCalledParams {
    #[serde(rename = "type")]
    call_type: String,
    args: serde_json::Value,
}

/// `Log.entryAdded` parameters.
#[derive(Deserialize)]
struct LogEntryAddedParams {
    entry: LogEntry,
}

#[derive(Deserialize)]
struct LogEntry {
    #[serde(default)]
    source: String,
    #[serde(default)]
    level: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    url: String,
    #[serde(default, rename = "lineNumber")]
    line: i32,
}

/// Discovers the CEF debug tab and returns its WebSocket URL.
pub async fn discover_ws_url() -> Result<String, String> {
    let client = CefClient::new();
    let tabs = client
        .get_tabs()
        .await
        .map_err(|e| format!("failed to discover CEF tabs: {e}"))?;

    let tab = CefClient::find_js_context(&tabs).map_err(|e| format!("no suitable tab: {e}"))?;

    if tab.web_socket_debugger_url.is_empty() {
        return Err("tab has no WebSocket debugger URL".into());
    }

    Ok(tab.web_socket_debugger_url.clone())
}

/// Result of connecting and streaming: why the connection ended.
pub enum StreamEnd {
    /// Context was cancelled (graceful shutdown).
    Cancelled,
    /// Connection or protocol error.
    Error(String),
}

/// Connects to a CEF tab's WebSocket, enables `Runtime` + `Log` domains,
/// and streams console events until the cancellation token fires or an error occurs.
///
/// Each parsed entry is passed to `on_entry` with its level bit already resolved.
pub async fn stream_events<F>(
    ws_url: &str,
    level_mask: impl Fn() -> u32,
    cancel: tokio_util::sync::CancellationToken,
    on_entry: F,
) -> StreamEnd
where
    F: Fn(ConsoleLogEntry),
{
    // Connect WebSocket.
    let ws = match tokio::time::timeout(
        WS_HANDSHAKE_TIMEOUT,
        tokio_tungstenite::connect_async(ws_url),
    )
    .await
    {
        Ok(Ok((ws, _))) => ws,
        Ok(Err(e)) => return StreamEnd::Error(format!("WebSocket connect failed: {e}")),
        Err(_) => return StreamEnd::Error("WebSocket handshake timeout".into()),
    };

    let (mut write, mut read) = ws.split();

    // Enable Runtime and Log domains.
    for (id, method) in [(1, "Runtime.enable"), (2, "Log.enable")] {
        let cmd = CdpCommand {
            id,
            method,
            params: serde_json::json!({}),
        };
        let json = match serde_json::to_string(&cmd) {
            Ok(j) => j,
            Err(e) => return StreamEnd::Error(format!("failed to serialize CDP command: {e}")),
        };
        if let Err(e) = write.send(WsMessage::Text(json.into())).await {
            return StreamEnd::Error(format!("failed to send CDP command: {e}"));
        }
    }

    tracing::debug!("CDP console log streaming started");

    // Read loop.
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                let _ = write.close().await;
                return StreamEnd::Cancelled;
            }
            frame = read.next() => {
                match frame {
                    Some(Ok(WsMessage::Text(text))) => {
                        if let Ok(event) = serde_json::from_str::<CdpEvent>(&text)
                            && let Some(entry) = parse_event(&event, &level_mask)
                        {
                            on_entry(entry);
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) | None => {
                        return StreamEnd::Error("CDP WebSocket closed".into());
                    }
                    Some(Err(e)) => {
                        return StreamEnd::Error(format!("CDP WebSocket read error: {e}"));
                    }
                    _ => {} // Ping/Pong/Binary — ignore.
                }
            }
        }
    }
}

/// Parses a CDP event into a `ConsoleLogEntry`, applying level filtering.
fn parse_event(event: &CdpEvent, level_mask: &impl Fn() -> u32) -> Option<ConsoleLogEntry> {
    let method = event.method.as_deref()?;
    let params = event.params.as_ref()?;

    match method {
        "Runtime.consoleAPICalled" => {
            let api: ConsoleApiCalledParams = serde_json::from_value(params.clone()).ok()?;
            let level = normalize_level(&api.call_type);
            let bit = log_level_bit(&level);
            if bit == 0 || level_mask() & bit == 0 {
                return None;
            }
            let (text, segments) = format_console_args(&api.args);
            Some(ConsoleLogEntry {
                timestamp: now_millis(),
                level,
                source: "console".into(),
                text,
                url: String::new(),
                line: 0,
                segments,
            })
        }
        "Log.entryAdded" => {
            let log: LogEntryAddedParams = serde_json::from_value(params.clone()).ok()?;
            let entry = &log.entry;
            let level = normalize_level(&entry.level);
            let bit = log_level_bit(&level);
            if bit == 0 || level_mask() & bit == 0 {
                return None;
            }
            Some(ConsoleLogEntry {
                timestamp: now_millis(),
                level,
                source: entry.source.clone(),
                text: entry.text.clone(),
                url: entry.url.clone(),
                line: entry.line,
                segments: vec![],
            })
        }
        _ => None,
    }
}

/// Normalizes CDP level strings to our canonical names.
fn normalize_level(level: &str) -> String {
    match level {
        "warning" => "warn".into(),
        "verbose" => "debug".into(),
        other => other.to_string(),
    }
}

/// Formats console API arguments, handling `%c` styled segments.
///
/// If the first argument contains `%c` directives, subsequent arguments are
/// consumed as CSS styles for the corresponding text segments.
fn format_console_args(args: &serde_json::Value) -> (String, Vec<StyledSegment>) {
    let arr = match args.as_array() {
        Some(a) if !a.is_empty() => a,
        _ => return (String::new(), vec![]),
    };

    // Extract text value from a console argument object.
    let arg_text = |arg: &serde_json::Value| -> String {
        if let Some(v) = arg.get("value") {
            match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Null => "null".into(),
                other => other.to_string(),
            }
        } else if let Some(desc) = arg.get("description").and_then(|d| d.as_str()) {
            desc.to_string()
        } else {
            String::new()
        }
    };

    let first_text = arg_text(&arr[0]);

    // Check if first arg contains %c directives.
    if !first_text.contains("%c") {
        // Plain text: concatenate all args.
        let text: String = arr.iter().map(arg_text).collect::<Vec<_>>().join(" ");
        return (text, vec![]);
    }

    // Split by %c and build styled segments.
    let parts: Vec<&str> = first_text.split("%c").collect();
    let mut segments = Vec::new();
    let mut css_arg_idx = 1usize; // CSS args start at index 1.

    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            // Text before the first %c — no CSS.
            if !part.is_empty() {
                segments.push(StyledSegment {
                    text: part.to_string(),
                    css: String::new(),
                });
            }
        } else {
            // Text after %c[i], consumes args[css_arg_idx] as CSS.
            let css = if css_arg_idx < arr.len() {
                let css_val = arg_text(&arr[css_arg_idx]);
                css_arg_idx += 1;
                css_val
            } else {
                String::new()
            };
            if !part.is_empty() || !css.is_empty() {
                segments.push(StyledSegment {
                    text: part.to_string(),
                    css,
                });
            }
        }
    }

    // Remaining args after all CSS values are appended as plain text.
    for arg in arr.iter().skip(css_arg_idx) {
        let text = arg_text(arg);
        if !text.is_empty() {
            segments.push(StyledSegment {
                text,
                css: String::new(),
            });
        }
    }

    // Build plain text from all segments.
    let plain: String = segments.iter().map(|s| s.text.as_str()).collect();

    (plain, segments)
}

/// Current timestamp in milliseconds.
fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_level_mapping() {
        assert_eq!(normalize_level("warning"), "warn");
        assert_eq!(normalize_level("verbose"), "debug");
        assert_eq!(normalize_level("log"), "log");
        assert_eq!(normalize_level("error"), "error");
        assert_eq!(normalize_level("info"), "info");
    }

    #[test]
    fn format_plain_args() {
        let args = serde_json::json!([
            {"type": "string", "value": "hello"},
            {"type": "string", "value": "world"}
        ]);
        let (text, segments) = format_console_args(&args);
        assert_eq!(text, "hello world");
        assert!(segments.is_empty());
    }

    #[test]
    fn format_styled_args() {
        // console.log("%cHello%c World", "color: red", "color: blue")
        let args = serde_json::json!([
            {"type": "string", "value": "%cHello%c World"},
            {"type": "string", "value": "color: red"},
            {"type": "string", "value": "color: blue"}
        ]);
        let (text, segments) = format_console_args(&args);
        assert_eq!(text, "Hello World");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].text, "Hello");
        assert_eq!(segments[0].css, "color: red");
        assert_eq!(segments[1].text, " World");
        assert_eq!(segments[1].css, "color: blue");
    }

    #[test]
    fn format_styled_with_trailing_args() {
        // console.log("%cStyled", "color: red", "extra text")
        let args = serde_json::json!([
            {"type": "string", "value": "%cStyled"},
            {"type": "string", "value": "color: red"},
            {"type": "string", "value": "extra text"}
        ]);
        let (text, segments) = format_console_args(&args);
        assert_eq!(text, "Styledextra text");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].css, "color: red");
        assert_eq!(segments[1].text, "extra text");
        assert!(segments[1].css.is_empty());
    }

    #[test]
    fn format_empty_args() {
        let args = serde_json::json!([]);
        let (text, segments) = format_console_args(&args);
        assert!(text.is_empty());
        assert!(segments.is_empty());
    }

    #[test]
    fn format_non_string_args() {
        let args = serde_json::json!([
            {"type": "number", "value": 42},
            {"type": "boolean", "value": true},
            {"type": "object", "description": "[object Object]"}
        ]);
        let (text, segments) = format_console_args(&args);
        assert_eq!(text, "42 true [object Object]");
        assert!(segments.is_empty());
    }

    #[test]
    fn format_null_value() {
        let args = serde_json::json!([
            {"type": "object", "subtype": "null", "value": null}
        ]);
        let (text, _) = format_console_args(&args);
        assert_eq!(text, "null");
    }

    #[test]
    fn parse_console_api_event() {
        let event = CdpEvent {
            method: Some("Runtime.consoleAPICalled".into()),
            params: Some(serde_json::json!({
                "type": "log",
                "args": [{"type": "string", "value": "test message"}]
            })),
        };
        let mask = || 0xFFu32;
        let entry = parse_event(&event, &mask).unwrap();
        assert_eq!(entry.level, "log");
        assert_eq!(entry.source, "console");
        assert_eq!(entry.text, "test message");
        assert!(entry.timestamp > 0);
    }

    #[test]
    fn parse_log_entry_added_event() {
        let event = CdpEvent {
            method: Some("Log.entryAdded".into()),
            params: Some(serde_json::json!({
                "entry": {
                    "source": "javascript",
                    "level": "error",
                    "text": "Uncaught TypeError",
                    "url": "https://example.com/app.js",
                    "lineNumber": 42
                }
            })),
        };
        let mask = || 0xFFu32;
        let entry = parse_event(&event, &mask).unwrap();
        assert_eq!(entry.level, "error");
        assert_eq!(entry.source, "javascript");
        assert_eq!(entry.text, "Uncaught TypeError");
        assert_eq!(entry.url, "https://example.com/app.js");
        assert_eq!(entry.line, 42);
    }

    #[test]
    fn parse_event_filters_by_mask() {
        let event = CdpEvent {
            method: Some("Runtime.consoleAPICalled".into()),
            params: Some(serde_json::json!({
                "type": "debug",
                "args": [{"type": "string", "value": "debug msg"}]
            })),
        };
        // Default mask (15) excludes debug (16).
        let mask = || capydeploy_protocol::constants::LOG_LEVEL_DEFAULT;
        let entry = parse_event(&event, &mask);
        assert!(entry.is_none(), "debug should be filtered out");
    }

    #[test]
    fn parse_event_passes_debug_when_enabled() {
        let event = CdpEvent {
            method: Some("Runtime.consoleAPICalled".into()),
            params: Some(serde_json::json!({
                "type": "debug",
                "args": [{"type": "string", "value": "debug msg"}]
            })),
        };
        let mask = || 0xFFu32; // All levels enabled.
        let entry = parse_event(&event, &mask);
        assert!(entry.is_some());
    }

    #[test]
    fn parse_unknown_event_returns_none() {
        let event = CdpEvent {
            method: Some("Page.loadEventFired".into()),
            params: Some(serde_json::json!({})),
        };
        let mask = || 0xFFu32;
        assert!(parse_event(&event, &mask).is_none());
    }

    #[test]
    fn parse_event_normalizes_warning() {
        let event = CdpEvent {
            method: Some("Log.entryAdded".into()),
            params: Some(serde_json::json!({
                "entry": {
                    "source": "network",
                    "level": "warning",
                    "text": "resource not found"
                }
            })),
        };
        let mask = || 0xFFu32;
        let entry = parse_event(&event, &mask).unwrap();
        assert_eq!(entry.level, "warn");
    }
}
