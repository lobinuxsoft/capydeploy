//! Games manager — business logic for installed games CRUD.
//!
//! Uses the same `AgentConnection` trait pattern as hub-deploy,
//! but returns `GamesError` instead of `DeployError`.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;

use capydeploy_protocol::constants::MessageType;
use capydeploy_protocol::envelope::Message;
use capydeploy_protocol::messages::{
    DeleteGameRequest, DeleteGameResponse, ListShortcutsRequest, SetGameLogWrapperRequest,
    ShortcutsListResponse, SteamUsersResponse,
};
use capydeploy_protocol::telemetry::SetGameLogWrapperResponse;
use tracing::{debug, warn};

use crate::error::GamesError;
use crate::types::{ArtworkUpdate, InstalledGame};

/// Abstract connection to an Agent.
///
/// The Hub app implements this trait on top of `ConnectionManager`.
/// Same shape as hub-deploy's trait but returns `GamesError`.
pub trait AgentConnection: Send + Sync {
    /// Sends a JSON request and waits for the response.
    fn send_request(
        &self,
        msg_type: MessageType,
        payload: &serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<Message, GamesError>> + Send + '_>>;

    /// Sends binary data with a JSON header to the agent.
    fn send_binary(
        &self,
        header: &serde_json::Value,
        data: &[u8],
    ) -> Pin<Box<dyn Future<Output = Result<Message, GamesError>> + Send + '_>>;

    /// Returns the agent's unique identifier.
    fn agent_id(&self) -> &str;
}

/// Manages installed games on remote agents.
pub struct GamesManager {
    http_client: reqwest::Client,
}

impl GamesManager {
    /// Creates a new games manager with the given HTTP client.
    pub fn new(http_client: reqwest::Client) -> Self {
        Self { http_client }
    }

    /// Lists all installed games (non-Steam shortcuts) on the agent.
    ///
    /// Fetches Steam users, then lists shortcuts for the first user.
    /// Returns an empty list if no Steam users are found.
    pub async fn get_installed_games(
        &self,
        conn: &dyn AgentConnection,
    ) -> Result<Vec<InstalledGame>, GamesError> {
        // 1. Get Steam users.
        let payload = serde_json::json!({});
        let resp = conn
            .send_request(MessageType::GetSteamUsers, &payload)
            .await?;

        let users_resp: SteamUsersResponse = resp
            .parse_payload::<SteamUsersResponse>()?
            .ok_or_else(|| GamesError::Agent("empty steam users response".into()))?;

        if users_resp.users.is_empty() {
            return Ok(Vec::new());
        }

        // 2. List shortcuts for the first user.
        let user_id: u32 = users_resp.users[0]
            .id
            .parse()
            .map_err(|e| GamesError::Agent(format!("invalid user id: {e}")))?;

        let list_req = ListShortcutsRequest { user_id };
        let payload = serde_json::to_value(&list_req)?;
        let resp = conn
            .send_request(MessageType::ListShortcuts, &payload)
            .await?;

        let shortcuts_resp: ShortcutsListResponse = resp
            .parse_payload::<ShortcutsListResponse>()?
            .ok_or_else(|| GamesError::Agent("empty shortcuts response".into()))?;

        // 3. Map shortcuts to InstalledGame.
        let games = shortcuts_resp
            .shortcuts
            .into_iter()
            .map(|sc| InstalledGame {
                name: sc.name,
                path: sc.start_dir,
                size: "N/A".into(),
                app_id: sc.app_id,
            })
            .collect();

        Ok(games)
    }

    /// Deletes a game from the agent.
    ///
    /// The agent handles everything internally: user detection, file deletion,
    /// shortcut removal, and Steam restart.
    pub async fn delete_game(
        &self,
        conn: &dyn AgentConnection,
        app_id: u32,
    ) -> Result<DeleteGameResponse, GamesError> {
        let req = DeleteGameRequest { app_id };
        let payload = serde_json::to_value(&req)?;
        let resp = conn.send_request(MessageType::DeleteGame, &payload).await?;

        let delete_resp: DeleteGameResponse = resp
            .parse_payload::<DeleteGameResponse>()?
            .ok_or_else(|| GamesError::Agent("empty delete game response".into()))?;

        Ok(delete_resp)
    }

    /// Updates artwork for an installed game.
    ///
    /// For each non-empty field in `artwork`:
    /// - `file://` prefix: reads the local file and detects content type from extension.
    /// - `http(s)://` prefix: downloads via HTTP and gets content type from response headers.
    /// - Empty: skipped.
    ///
    /// Errors on individual artwork items are logged and skipped (non-fatal),
    /// matching the Go implementation behavior.
    pub async fn update_game_artwork(
        &self,
        conn: &dyn AgentConnection,
        app_id: u32,
        artwork: &ArtworkUpdate,
    ) -> Result<(), GamesError> {
        let fields = [
            ("grid", &artwork.grid),
            ("banner", &artwork.banner),
            ("hero", &artwork.hero),
            ("logo", &artwork.logo),
            ("icon", &artwork.icon),
        ];

        for (art_type, src) in fields {
            if src.is_empty() {
                continue;
            }

            let result = self.resolve_artwork_source(src).await;

            let (data, content_type) = match result {
                Ok(pair) => pair,
                Err(e) => {
                    warn!(art_type, src, error = %e, "failed to resolve artwork source");
                    continue;
                }
            };

            if content_type.is_empty() {
                warn!(art_type, src, "unknown content type for artwork");
                continue;
            }

            let header = serde_json::json!({
                "appId": app_id,
                "artworkType": art_type,
                "contentType": content_type,
            });

            match conn.send_binary(&header, &data).await {
                Ok(_) => {
                    debug!(art_type, app_id, "sent artwork");
                }
                Err(e) => {
                    warn!(art_type, app_id, error = %e, "failed to send artwork");
                }
            }
        }

        Ok(())
    }

    /// Enables or disables the game log wrapper for a specific game.
    pub async fn set_game_log_wrapper(
        &self,
        conn: &dyn AgentConnection,
        app_id: u32,
        enabled: bool,
    ) -> Result<SetGameLogWrapperResponse, GamesError> {
        let req = SetGameLogWrapperRequest { app_id, enabled };
        let payload = serde_json::to_value(&req)?;
        let resp = conn
            .send_request(MessageType::SetGameLogWrapper, &payload)
            .await?;

        let wrapper_resp: SetGameLogWrapperResponse = resp
            .parse_payload::<SetGameLogWrapperResponse>()?
            .ok_or_else(|| GamesError::Agent("empty set game log wrapper response".into()))?;

        Ok(wrapper_resp)
    }

    /// Resolves an artwork source string to (data, content_type).
    async fn resolve_artwork_source(&self, src: &str) -> Result<(Vec<u8>, String), GamesError> {
        if let Some(path) = src.strip_prefix("file://") {
            let data = tokio::fs::read(path).await?;
            let content_type = detect_content_type(path).unwrap_or("").to_string();
            Ok((data, content_type))
        } else if src.starts_with("http://") || src.starts_with("https://") {
            let response = self
                .http_client
                .get(src)
                .send()
                .await
                .map_err(|e| GamesError::Http(format!("failed to download {src}: {e}")))?;

            if !response.status().is_success() {
                return Err(GamesError::Http(format!(
                    "download {src} returned status {}",
                    response.status()
                )));
            }

            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();

            let data = response
                .bytes()
                .await
                .map_err(|e| GamesError::Http(format!("failed to read response from {src}: {e}")))?
                .to_vec();

            Ok((data, content_type))
        } else {
            Err(GamesError::Artwork(format!(
                "unknown artwork source scheme: {src}"
            )))
        }
    }
}

/// Detects MIME content type from a file path extension.
fn detect_content_type(path: &str) -> Option<&'static str> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match ext.as_deref() {
        Some("png") => Some("image/png"),
        Some("jpg" | "jpeg") => Some("image/jpeg"),
        Some("webp") => Some("image/webp"),
        Some("gif") => Some("image/gif"),
        Some("ico") => Some("image/x-icon"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use capydeploy_protocol::messages::SteamUser;
    use capydeploy_protocol::types::ShortcutInfo;
    use std::sync::Mutex;

    /// Mock agent connection that records requests and returns canned responses.
    struct MockConn {
        id: String,
        responses: Mutex<Vec<Message>>,
        requests: Mutex<Vec<(String, serde_json::Value)>>,
        binary_sends: Mutex<Vec<(serde_json::Value, Vec<u8>)>>,
    }

    impl MockConn {
        fn new(id: &str, responses: Vec<Message>) -> Self {
            Self {
                id: id.into(),
                responses: Mutex::new(responses),
                requests: Mutex::new(Vec::new()),
                binary_sends: Mutex::new(Vec::new()),
            }
        }

        fn request_count(&self) -> usize {
            self.requests.lock().unwrap().len()
        }

        fn binary_count(&self) -> usize {
            self.binary_sends.lock().unwrap().len()
        }

        fn last_request_payload(&self) -> serde_json::Value {
            self.requests.lock().unwrap().last().unwrap().1.clone()
        }

        fn binary_headers(&self) -> Vec<serde_json::Value> {
            self.binary_sends
                .lock()
                .unwrap()
                .iter()
                .map(|(h, _)| h.clone())
                .collect()
        }
    }

    impl AgentConnection for MockConn {
        fn send_request(
            &self,
            msg_type: MessageType,
            payload: &serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = Result<Message, GamesError>> + Send + '_>> {
            let msg_type_str = format!("{msg_type:?}");
            self.requests
                .lock()
                .unwrap()
                .push((msg_type_str, payload.clone()));

            Box::pin(async move {
                let mut responses = self.responses.lock().unwrap();
                if responses.is_empty() {
                    Err(GamesError::Agent("no mock response available".into()))
                } else {
                    Ok(responses.remove(0))
                }
            })
        }

        fn send_binary(
            &self,
            header: &serde_json::Value,
            data: &[u8],
        ) -> Pin<Box<dyn Future<Output = Result<Message, GamesError>> + Send + '_>> {
            self.binary_sends
                .lock()
                .unwrap()
                .push((header.clone(), data.to_vec()));

            Box::pin(async move {
                let msg = Message::new::<()>("bin-resp", MessageType::Pong, None).unwrap();
                Ok(msg)
            })
        }

        fn agent_id(&self) -> &str {
            &self.id
        }
    }

    fn make_users_response(users: Vec<SteamUser>) -> Message {
        let resp = SteamUsersResponse { users };
        Message::new("u1", MessageType::SteamUsersResponse, Some(&resp)).unwrap()
    }

    fn make_shortcuts_response(shortcuts: Vec<ShortcutInfo>) -> Message {
        let resp = ShortcutsListResponse { shortcuts };
        Message::new("s1", MessageType::ShortcutsResponse, Some(&resp)).unwrap()
    }

    fn make_delete_response(status: &str, game_name: &str) -> Message {
        let resp = DeleteGameResponse {
            status: status.into(),
            game_name: game_name.into(),
            steam_restarted: true,
        };
        Message::new("d1", MessageType::OperationResult, Some(&resp)).unwrap()
    }

    fn make_log_wrapper_response(app_id: u32, enabled: bool) -> Message {
        let resp = SetGameLogWrapperResponse { app_id, enabled };
        Message::new("lw1", MessageType::SetGameLogWrapper, Some(&resp)).unwrap()
    }

    // -----------------------------------------------------------------------
    // get_installed_games
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn get_installed_games_returns_mapped_shortcuts() {
        let users = vec![SteamUser {
            id: "12345".into(),
            name: "Player".into(),
            avatar_url: String::new(),
            last_login_at: 0,
        }];
        let shortcuts = vec![
            ShortcutInfo {
                app_id: 100,
                name: "Game A".into(),
                exe: "/bin/a".into(),
                start_dir: "/games/a".into(),
                launch_options: String::new(),
                tags: vec![],
                last_played: 0,
            },
            ShortcutInfo {
                app_id: 200,
                name: "Game B".into(),
                exe: "/bin/b".into(),
                start_dir: "/games/b".into(),
                launch_options: String::new(),
                tags: vec![],
                last_played: 0,
            },
        ];

        let conn = MockConn::new(
            "agent-1",
            vec![
                make_users_response(users),
                make_shortcuts_response(shortcuts),
            ],
        );

        let mgr = GamesManager::new(reqwest::Client::new());
        let games = mgr.get_installed_games(&conn).await.unwrap();

        assert_eq!(games.len(), 2);
        assert_eq!(games[0].name, "Game A");
        assert_eq!(games[0].path, "/games/a");
        assert_eq!(games[0].size, "N/A");
        assert_eq!(games[0].app_id, 100);
        assert_eq!(games[1].name, "Game B");
        assert_eq!(games[1].app_id, 200);
        assert_eq!(conn.request_count(), 2);
    }

    #[tokio::test]
    async fn get_installed_games_no_users_returns_empty() {
        let conn = MockConn::new("agent-1", vec![make_users_response(vec![])]);

        let mgr = GamesManager::new(reqwest::Client::new());
        let games = mgr.get_installed_games(&conn).await.unwrap();

        assert!(games.is_empty());
        assert_eq!(conn.request_count(), 1); // Only GetSteamUsers.
    }

    // -----------------------------------------------------------------------
    // delete_game
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn delete_game_sends_correct_app_id() {
        let conn = MockConn::new(
            "agent-1",
            vec![make_delete_response("success", "Test Game")],
        );

        let mgr = GamesManager::new(reqwest::Client::new());
        let resp = mgr.delete_game(&conn, 42).await.unwrap();

        assert_eq!(resp.status, "success");
        assert_eq!(resp.game_name, "Test Game");
        assert!(resp.steam_restarted);

        let payload = conn.last_request_payload();
        assert_eq!(payload["appId"], 42);
    }

    #[tokio::test]
    async fn delete_game_empty_response_errors() {
        let conn = MockConn::new("agent-1", vec![]);

        let mgr = GamesManager::new(reqwest::Client::new());
        let result = mgr.delete_game(&conn, 42).await;
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // update_game_artwork
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn update_artwork_sends_local_files() {
        let dir = tempfile::tempdir().unwrap();
        let img_path = dir.path().join("hero.png");
        std::fs::write(&img_path, b"PNG_DATA").unwrap();

        let file_url = format!("file://{}", img_path.to_string_lossy());
        let artwork = ArtworkUpdate {
            hero: file_url,
            ..Default::default()
        };

        let conn = MockConn::new("agent-1", vec![]);
        let mgr = GamesManager::new(reqwest::Client::new());
        mgr.update_game_artwork(&conn, 999, &artwork).await.unwrap();

        assert_eq!(conn.binary_count(), 1);
        let headers = conn.binary_headers();
        assert_eq!(headers[0]["appId"], 999);
        assert_eq!(headers[0]["artworkType"], "hero");
        assert_eq!(headers[0]["contentType"], "image/png");
    }

    #[tokio::test]
    async fn update_artwork_skips_empty_fields() {
        let artwork = ArtworkUpdate::default(); // All empty.
        let conn = MockConn::new("agent-1", vec![]);
        let mgr = GamesManager::new(reqwest::Client::new());
        mgr.update_game_artwork(&conn, 999, &artwork).await.unwrap();

        assert_eq!(conn.binary_count(), 0);
    }

    #[tokio::test]
    async fn update_artwork_skips_missing_local_files() {
        let artwork = ArtworkUpdate {
            grid: "file:///nonexistent/path/grid.png".into(),
            ..Default::default()
        };

        let conn = MockConn::new("agent-1", vec![]);
        let mgr = GamesManager::new(reqwest::Client::new());
        // Should not error — just logs and skips.
        mgr.update_game_artwork(&conn, 999, &artwork).await.unwrap();

        assert_eq!(conn.binary_count(), 0);
    }

    // -----------------------------------------------------------------------
    // set_game_log_wrapper
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn set_game_log_wrapper_enable() {
        let conn = MockConn::new("agent-1", vec![make_log_wrapper_response(42, true)]);

        let mgr = GamesManager::new(reqwest::Client::new());
        let resp = mgr.set_game_log_wrapper(&conn, 42, true).await.unwrap();

        assert_eq!(resp.app_id, 42);
        assert!(resp.enabled);

        let payload = conn.last_request_payload();
        assert_eq!(payload["appId"], 42);
        assert_eq!(payload["enabled"], true);
    }

    #[tokio::test]
    async fn set_game_log_wrapper_disable() {
        let conn = MockConn::new("agent-1", vec![make_log_wrapper_response(99, false)]);

        let mgr = GamesManager::new(reqwest::Client::new());
        let resp = mgr.set_game_log_wrapper(&conn, 99, false).await.unwrap();

        assert_eq!(resp.app_id, 99);
        assert!(!resp.enabled);
    }

    // -----------------------------------------------------------------------
    // detect_content_type
    // -----------------------------------------------------------------------

    #[test]
    fn detect_content_type_known_extensions() {
        assert_eq!(detect_content_type("image.png"), Some("image/png"));
        assert_eq!(detect_content_type("photo.jpg"), Some("image/jpeg"));
        assert_eq!(detect_content_type("photo.jpeg"), Some("image/jpeg"));
        assert_eq!(detect_content_type("art.webp"), Some("image/webp"));
        assert_eq!(detect_content_type("anim.gif"), Some("image/gif"));
        assert_eq!(detect_content_type("favicon.ico"), Some("image/x-icon"));
    }

    #[test]
    fn detect_content_type_case_insensitive() {
        assert_eq!(detect_content_type("IMAGE.PNG"), Some("image/png"));
        assert_eq!(detect_content_type("Photo.JPG"), Some("image/jpeg"));
    }

    #[test]
    fn detect_content_type_unknown() {
        assert_eq!(detect_content_type("document.pdf"), None);
        assert_eq!(detect_content_type("noext"), None);
        assert_eq!(detect_content_type(""), None);
    }
}
