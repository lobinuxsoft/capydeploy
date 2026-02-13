//! Chrome DevTools Protocol client for Steam's embedded Chromium (CEF).
//!
//! Connects to Steam's CEF debug endpoint at `localhost:8080`, finds the
//! `SharedJSContext` or `SP` tab, and evaluates JavaScript via WebSocket.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;

use crate::SteamError;

/// CEF debug HTTP endpoint.
const CEF_HTTP_ENDPOINT: &str = "127.0.0.1:8080";

/// Timeout for the HTTP tab discovery request.
const HTTP_TIMEOUT: Duration = Duration::from_secs(5);

/// Timeout for the WebSocket handshake.
const WS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// Read timeout for each WebSocket message.
const WS_READ_TIMEOUT: Duration = Duration::from_secs(10);

/// CEF artwork asset type constants (match Steam's internal enum).
pub const CEF_ASSET_GRID_PORTRAIT: i32 = 0;
pub const CEF_ASSET_HERO: i32 = 1;
pub const CEF_ASSET_LOGO: i32 = 2;
pub const CEF_ASSET_GRID_LANDSCAPE: i32 = 3;
pub const CEF_ASSET_ICON: i32 = 4;

/// Maps an artwork type string to its CEF asset constant.
pub fn artwork_type_to_cef_asset(artwork_type: &str) -> Option<i32> {
    match artwork_type {
        "grid" => Some(CEF_ASSET_GRID_PORTRAIT),
        "banner" => Some(CEF_ASSET_GRID_LANDSCAPE),
        "hero" => Some(CEF_ASSET_HERO),
        "logo" => Some(CEF_ASSET_LOGO),
        "icon" => Some(CEF_ASSET_ICON),
        _ => None,
    }
}

/// A CEF browser tab from the debug endpoint.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CefTab {
    pub title: String,
    #[serde(rename = "type")]
    pub tab_type: String,
    pub id: String,
    pub url: String,
    #[serde(default)]
    pub web_socket_debugger_url: String,
}

/// CEF/CDP client for communicating with Steam.
pub struct CefClient {
    endpoint: String,
}

impl CefClient {
    /// Creates a new client targeting the local Steam CEF debugger.
    pub fn new() -> Self {
        Self {
            endpoint: CEF_HTTP_ENDPOINT.to_string(),
        }
    }

    /// Fetches the list of debuggable tabs from CEF.
    pub async fn get_tabs(&self) -> Result<Vec<CefTab>, SteamError> {
        let raw = http_get_json(&self.endpoint).await?;
        let tabs: Vec<CefTab> = serde_json::from_str(&raw)
            .map_err(|e| SteamError::Cef(format!("failed to parse CEF tabs: {e}")))?;
        Ok(tabs)
    }

    /// Finds the best tab for JS evaluation (SharedJSContext > SP).
    pub fn find_js_context(tabs: &[CefTab]) -> Result<&CefTab, SteamError> {
        let mut sp_tab: Option<&CefTab> = None;

        for tab in tabs {
            if tab.web_socket_debugger_url.is_empty() {
                continue;
            }
            if tab.title == "SharedJSContext" {
                return Ok(tab);
            }
            if tab.title == "SP" && sp_tab.is_none() {
                sp_tab = Some(tab);
            }
        }

        sp_tab.ok_or_else(|| {
            SteamError::Cef("no suitable JS context found (need SharedJSContext or SP tab)".into())
        })
    }

    /// Evaluates a JavaScript expression via CDP `Runtime.evaluate`.
    ///
    /// Returns the raw JSON value from the evaluation result.
    pub async fn evaluate(&self, js_expr: &str) -> Result<serde_json::Value, SteamError> {
        let tabs = self.get_tabs().await?;
        let tab = Self::find_js_context(&tabs)?;
        let ws_url = &tab.web_socket_debugger_url;

        evaluate_async(ws_url, js_expr).await
    }

    /// Evaluates JS and ignores the return value (for void operations).
    pub async fn evaluate_void(&self, js_expr: &str) -> Result<(), SteamError> {
        self.evaluate(js_expr).await?;
        Ok(())
    }

    // ---------------------------------------------------------------
    // High-level Steam API operations
    // ---------------------------------------------------------------

    /// Creates a Steam shortcut and returns the assigned AppID.
    pub async fn add_shortcut(
        &self,
        name: &str,
        exe: &str,
        start_dir: &str,
        launch_options: &str,
    ) -> Result<u32, SteamError> {
        let js = format!(
            "SteamClient.Apps.AddShortcut({}, {}, {}, {})",
            js_string(name),
            js_string(exe),
            js_string(start_dir),
            js_string(launch_options),
        );

        let result = self.evaluate(&js).await?;

        let app_id = result.as_f64().map(|v| v as u32).ok_or_else(|| {
            SteamError::Cef(format!(
                "failed to parse AddShortcut result: expected number, got {result}"
            ))
        })?;

        if app_id == 0 {
            return Err(SteamError::Cef(format!(
                "AddShortcut returned invalid appID: {app_id}"
            )));
        }

        Ok(app_id)
    }

    /// Removes a Steam shortcut by AppID.
    pub async fn remove_shortcut(&self, app_id: u32) -> Result<(), SteamError> {
        let js = format!("SteamClient.Apps.RemoveShortcut({app_id})");
        self.evaluate_void(&js).await
    }

    /// Renames a shortcut.
    pub async fn set_shortcut_name(&self, app_id: u32, name: &str) -> Result<(), SteamError> {
        let js = format!(
            "SteamClient.Apps.SetShortcutName({app_id}, {})",
            js_string(name),
        );
        self.evaluate_void(&js).await
    }

    /// Sets launch options for a shortcut.
    pub async fn set_shortcut_launch_options(
        &self,
        app_id: u32,
        options: &str,
    ) -> Result<(), SteamError> {
        let js = format!(
            "SteamClient.Apps.SetShortcutLaunchOptions({app_id}, {})",
            js_string(options),
        );
        self.evaluate_void(&js).await
    }

    /// Sets the compatibility tool (e.g. Proton) for a shortcut.
    pub async fn specify_compat_tool(
        &self,
        app_id: u32,
        tool_name: &str,
    ) -> Result<(), SteamError> {
        let js = format!(
            "SteamClient.Apps.SpecifyCompatTool({app_id}, {})",
            js_string(tool_name),
        );
        self.evaluate_void(&js).await
    }

    /// Applies custom artwork to a Steam app.
    pub async fn set_custom_artwork(
        &self,
        app_id: u32,
        base64_data: &str,
        asset_type: i32,
    ) -> Result<(), SteamError> {
        // Clear first, then set (matching Go behavior).
        self.clear_custom_artwork(app_id, asset_type).await?;

        let js = format!(
            r#"SteamClient.Apps.SetCustomArtworkForApp({app_id}, "{base64_data}", "png", {asset_type})"#,
        );
        self.evaluate_void(&js).await
    }

    /// Clears custom artwork from a Steam app.
    pub async fn clear_custom_artwork(
        &self,
        app_id: u32,
        asset_type: i32,
    ) -> Result<(), SteamError> {
        let js = format!("SteamClient.Apps.ClearCustomArtworkForApp({app_id}, {asset_type})");
        self.evaluate_void(&js).await
    }
}

impl Default for CefClient {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Escapes a string for safe JavaScript literal embedding.
///
/// Uses JSON encoding which produces valid JS string literals.
fn js_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

/// CDP message sent to the browser.
#[derive(Serialize)]
struct CdpRequest {
    id: i32,
    method: String,
    params: serde_json::Value,
}

/// CDP response from the browser.
#[derive(Deserialize)]
struct CdpResponse {
    id: i32,
    result: Option<CdpEvalResult>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CdpEvalResult {
    result: CdpResultValue,
    exception_details: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct CdpResultValue {
    #[serde(default)]
    pub value: serde_json::Value,
}

/// Evaluates JS via a CDP WebSocket connection.
async fn evaluate_async(ws_url: &str, js_expr: &str) -> Result<serde_json::Value, SteamError> {
    let (mut ws, _) = tokio::time::timeout(
        WS_HANDSHAKE_TIMEOUT,
        tokio_tungstenite::connect_async(ws_url),
    )
    .await
    .map_err(|_| SteamError::Cef("CEF WebSocket handshake timeout".into()))?
    .map_err(|e| SteamError::Cef(format!("failed to connect to CEF WebSocket: {e}")))?;

    let msg = CdpRequest {
        id: 1,
        method: "Runtime.evaluate".into(),
        params: serde_json::json!({
            "expression": js_expr,
            "returnByValue": true,
            "awaitPromise": true,
        }),
    };

    let json = serde_json::to_string(&msg)
        .map_err(|e| SteamError::Cef(format!("failed to serialize CDP message: {e}")))?;

    ws.send(WsMessage::Text(json.into()))
        .await
        .map_err(|e| SteamError::Cef(format!("failed to send CEF message: {e}")))?;

    // Read responses until we get the one with our ID.
    loop {
        let frame = tokio::time::timeout(WS_READ_TIMEOUT, ws.next())
            .await
            .map_err(|_| SteamError::Cef("CEF response read timeout".into()))?
            .ok_or_else(|| SteamError::Cef("CEF WebSocket closed unexpectedly".into()))?
            .map_err(|e| SteamError::Cef(format!("failed to read CEF response: {e}")))?;

        let text = match frame {
            WsMessage::Text(t) => t,
            _ => continue,
        };

        let resp: CdpResponse = match serde_json::from_str(&text) {
            Ok(r) => r,
            Err(_) => continue, // Skip non-response messages (events, etc.)
        };

        if resp.id != 1 {
            continue;
        }

        let eval_result = resp
            .result
            .ok_or_else(|| SteamError::Cef("CEF response missing result".into()))?;

        if let Some(exception) = eval_result.exception_details {
            return Err(SteamError::Cef(format!("JS exception: {exception}")));
        }

        // Close the WebSocket gracefully.
        let _ = ws.close(None).await;

        return Ok(eval_result.result.value);
    }
}

/// Minimal HTTP GET to fetch JSON from the CEF debug endpoint.
async fn http_get_json(addr: &str) -> Result<String, SteamError> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let stream = tokio::time::timeout(HTTP_TIMEOUT, tokio::net::TcpStream::connect(addr))
        .await
        .map_err(|_| SteamError::Cef("CEF HTTP connection timeout".into()))?
        .map_err(|e| SteamError::Cef(format!("failed to connect to CEF debugger: {e}")))?;

    let request = format!("GET /json HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    let mut stream = stream;
    stream
        .write_all(request.as_bytes())
        .await
        .map_err(|e| SteamError::Cef(format!("failed to send HTTP request: {e}")))?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .map_err(|e| SteamError::Cef(format!("failed to read CEF response: {e}")))?;

    let response_str = String::from_utf8_lossy(&response);

    // Find the body after \r\n\r\n.
    let body_start = response_str
        .find("\r\n\r\n")
        .map(|i| i + 4)
        .ok_or_else(|| SteamError::Cef("invalid HTTP response from CEF".into()))?;

    Ok(response_str[body_start..].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_string_escaping() {
        assert_eq!(js_string("My Game"), r#""My Game""#);
        assert_eq!(js_string(r#"Say "hello""#), r#""Say \"hello\"""#);
        assert_eq!(js_string(r"path\to\file"), r#""path\\to\\file""#);
        assert_eq!(
            js_string(r"C:\Program Files\Steam\game.exe"),
            r#""C:\\Program Files\\Steam\\game.exe""#
        );
        assert_eq!(js_string(""), r#""""#);
        assert_eq!(js_string("日本語ゲーム"), r#""日本語ゲーム""#);
    }

    #[test]
    fn artwork_type_mapping() {
        assert_eq!(artwork_type_to_cef_asset("grid"), Some(0));
        assert_eq!(artwork_type_to_cef_asset("banner"), Some(3));
        assert_eq!(artwork_type_to_cef_asset("hero"), Some(1));
        assert_eq!(artwork_type_to_cef_asset("logo"), Some(2));
        assert_eq!(artwork_type_to_cef_asset("icon"), Some(4));
        assert_eq!(artwork_type_to_cef_asset("unknown"), None);
    }

    #[test]
    fn find_js_context_prefers_shared() {
        let tabs = vec![
            CefTab {
                title: "SP".into(),
                tab_type: "page".into(),
                id: "1".into(),
                url: "about:blank".into(),
                web_socket_debugger_url: "ws://localhost:8080/devtools/page/1".into(),
            },
            CefTab {
                title: "SharedJSContext".into(),
                tab_type: "page".into(),
                id: "2".into(),
                url: "about:blank".into(),
                web_socket_debugger_url: "ws://localhost:8080/devtools/page/2".into(),
            },
        ];

        let tab = CefClient::find_js_context(&tabs).unwrap();
        assert_eq!(tab.title, "SharedJSContext");
    }

    #[test]
    fn find_js_context_falls_back_to_sp() {
        let tabs = vec![
            CefTab {
                title: "Other".into(),
                tab_type: "page".into(),
                id: "1".into(),
                url: "about:blank".into(),
                web_socket_debugger_url: "ws://localhost:8080/devtools/page/1".into(),
            },
            CefTab {
                title: "SP".into(),
                tab_type: "page".into(),
                id: "2".into(),
                url: "about:blank".into(),
                web_socket_debugger_url: "ws://localhost:8080/devtools/page/2".into(),
            },
        ];

        let tab = CefClient::find_js_context(&tabs).unwrap();
        assert_eq!(tab.title, "SP");
    }

    #[test]
    fn find_js_context_skips_no_ws_url() {
        let tabs = vec![CefTab {
            title: "SharedJSContext".into(),
            tab_type: "page".into(),
            id: "1".into(),
            url: "about:blank".into(),
            web_socket_debugger_url: String::new(), // No WS URL.
        }];

        let result = CefClient::find_js_context(&tabs);
        assert!(result.is_err());
    }

    #[test]
    fn find_js_context_empty_tabs() {
        let result = CefClient::find_js_context(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn cef_client_default() {
        let client = CefClient::default();
        assert_eq!(client.endpoint, "127.0.0.1:8080");
    }
}
