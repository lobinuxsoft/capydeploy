//! SteamGridDB API client.
//!
//! Async HTTP client using `reqwest` with Bearer token authentication.

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

use crate::types::{ApiResponse, ImageData, ImageFilters, SearchResult};

const DEFAULT_BASE_URL: &str = "https://www.steamgriddb.com/api/v2";

/// Errors from the SteamGridDB client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error {status}: {body}")]
    Api { status: u16, body: String },

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid API key")]
    InvalidKey,
}

/// SteamGridDB API client.
pub struct Client {
    http: reqwest::Client,
    base_url: String,
}

impl Client {
    /// Creates a new client with the given API key.
    pub fn new(api_key: &str) -> Result<Self, Error> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_key}")).map_err(|_| Error::InvalidKey)?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            http,
            base_url: DEFAULT_BASE_URL.to_string(),
        })
    }

    /// Sets a custom base URL (for testing).
    #[cfg(test)]
    pub(crate) fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    /// Performs an authenticated GET request.
    async fn get(&self, endpoint: &str, params: &[(String, String)]) -> Result<Vec<u8>, Error> {
        let url = format!("{}{}", self.base_url, endpoint);
        let resp = self.http.get(&url).query(params).send().await?;
        let status = resp.status();

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::Api {
                status: status.as_u16(),
                body,
            });
        }

        Ok(resp.bytes().await?.to_vec())
    }

    /// Searches for games by name.
    pub async fn search(&self, term: &str) -> Result<Vec<SearchResult>, Error> {
        let encoded = utf8_percent_encode(term, NON_ALPHANUMERIC).to_string();
        let body = self
            .get(&format!("/search/autocomplete/{encoded}"), &[])
            .await?;
        let resp: ApiResponse<Vec<SearchResult>> = serde_json::from_slice(&body)?;
        Ok(resp.data)
    }

    /// Returns grid images for a game.
    pub async fn get_grids(
        &self,
        game_id: i32,
        filters: Option<&ImageFilters>,
        page: i32,
    ) -> Result<Vec<ImageData>, Error> {
        let params = build_params(filters, page);
        let body = self.get(&format!("/grids/game/{game_id}"), &params).await?;
        let resp: ApiResponse<Vec<ImageData>> = serde_json::from_slice(&body)?;
        Ok(resp.data)
    }

    /// Returns hero images for a game.
    pub async fn get_heroes(
        &self,
        game_id: i32,
        filters: Option<&ImageFilters>,
        page: i32,
    ) -> Result<Vec<ImageData>, Error> {
        let params = build_params(filters, page);
        let body = self
            .get(&format!("/heroes/game/{game_id}"), &params)
            .await?;
        let resp: ApiResponse<Vec<ImageData>> = serde_json::from_slice(&body)?;
        Ok(resp.data)
    }

    /// Returns logo images for a game.
    pub async fn get_logos(
        &self,
        game_id: i32,
        filters: Option<&ImageFilters>,
        page: i32,
    ) -> Result<Vec<ImageData>, Error> {
        let params = build_params(filters, page);
        let body = self.get(&format!("/logos/game/{game_id}"), &params).await?;
        let resp: ApiResponse<Vec<ImageData>> = serde_json::from_slice(&body)?;
        Ok(resp.data)
    }

    /// Returns icon images for a game.
    pub async fn get_icons(
        &self,
        game_id: i32,
        filters: Option<&ImageFilters>,
        page: i32,
    ) -> Result<Vec<ImageData>, Error> {
        let params = build_params(filters, page);
        let body = self.get(&format!("/icons/game/{game_id}"), &params).await?;
        let resp: ApiResponse<Vec<ImageData>> = serde_json::from_slice(&body)?;
        Ok(resp.data)
    }

    /// Downloads image data from a URL.
    pub async fn download_image(&self, url: &str) -> Result<Vec<u8>, Error> {
        let resp = self.http.get(url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                body: "download failed".into(),
            });
        }
        Ok(resp.bytes().await?.to_vec())
    }
}

/// Builds query parameters from filters and page.
fn build_params(filters: Option<&ImageFilters>, page: i32) -> Vec<(String, String)> {
    let mut params = Vec::new();

    if let Some(f) = filters {
        if !f.style.is_empty() && f.style != "All Styles" {
            params.push(("styles".into(), f.style.clone()));
        }
        if !f.mime_type.is_empty() && f.mime_type != "All Formats" {
            params.push(("mimes".into(), f.mime_type.clone()));
        }
        match f.image_type.as_str() {
            "static" | "Static Only" => params.push(("types".into(), "static".into())),
            "animated" | "Animated Only" => params.push(("types".into(), "animated".into())),
            _ => {}
        }
        if !f.dimension.is_empty() && f.dimension != "All Sizes" {
            params.push(("dimensions".into(), f.dimension.clone()));
        }
        params.push((
            "nsfw".into(),
            if f.show_nsfw { "any" } else { "false" }.into(),
        ));
        params.push((
            "humor".into(),
            if f.show_humor { "any" } else { "false" }.into(),
        ));
    }

    if page > 0 {
        params.push(("page".into(), page.to_string()));
    }

    params
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    /// Starts a mock HTTP server that responds with the given JSON body.
    async fn mock_server(body: &str) -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{port}");
        let body = body.to_string();

        let handle = tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = vec![0u8; 8192];
                let _ = stream.read(&mut buf).await;

                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.shutdown().await;
            }
        });

        (url, handle)
    }

    /// Starts a mock HTTP server that responds with an error status.
    async fn mock_server_error(status: u16, body: &str) -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{port}");
        let body = body.to_string();

        let handle = tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = vec![0u8; 8192];
                let _ = stream.read(&mut buf).await;

                let resp = format!(
                    "HTTP/1.1 {status} Error\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.shutdown().await;
            }
        });

        (url, handle)
    }

    #[tokio::test]
    async fn search_returns_results() {
        let json = r#"{"success":true,"data":[
            {"id":1,"name":"Test Game","types":["steam"],"verified":true},
            {"id":2,"name":"Test Game 2","types":["origin"]}
        ]}"#;
        let (url, handle) = mock_server(json).await;

        let client = Client::new("test-key").unwrap().with_base_url(url);
        let results = client.search("Test Game").await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "Test Game");
        assert!(results[0].verified);
        assert_eq!(results[1].id, 2);

        handle.abort();
    }

    #[tokio::test]
    async fn get_grids_returns_images() {
        let json = r#"{"success":true,"data":[
            {"id":100,"url":"https://example.com/grid.png","width":920,"height":430}
        ]}"#;
        let (url, handle) = mock_server(json).await;

        let client = Client::new("test-key").unwrap().with_base_url(url);
        let grids = client.get_grids(42, None, 0).await.unwrap();

        assert_eq!(grids.len(), 1);
        assert_eq!(grids[0].width, 920);
        assert_eq!(grids[0].height, 430);

        handle.abort();
    }

    #[tokio::test]
    async fn get_heroes_returns_images() {
        let json = r#"{"success":true,"data":[
            {"id":200,"url":"https://example.com/hero.png","width":1920,"height":620}
        ]}"#;
        let (url, handle) = mock_server(json).await;

        let client = Client::new("test-key").unwrap().with_base_url(url);
        let heroes = client.get_heroes(42, None, 0).await.unwrap();

        assert_eq!(heroes.len(), 1);
        assert_eq!(heroes[0].width, 1920);

        handle.abort();
    }

    #[tokio::test]
    async fn get_logos_returns_images() {
        let json = r#"{"success":true,"data":[{"id":300,"url":"https://example.com/logo.png"}]}"#;
        let (url, handle) = mock_server(json).await;

        let client = Client::new("test-key").unwrap().with_base_url(url);
        let logos = client.get_logos(42, None, 0).await.unwrap();

        assert_eq!(logos.len(), 1);
        assert_eq!(logos[0].id, 300);

        handle.abort();
    }

    #[tokio::test]
    async fn get_icons_returns_images() {
        let json = r#"{"success":true,"data":[{"id":400,"url":"https://example.com/icon.png"}]}"#;
        let (url, handle) = mock_server(json).await;

        let client = Client::new("test-key").unwrap().with_base_url(url);
        let icons = client.get_icons(42, None, 0).await.unwrap();

        assert_eq!(icons.len(), 1);
        assert_eq!(icons[0].id, 400);

        handle.abort();
    }

    #[tokio::test]
    async fn search_api_error() {
        let (url, handle) =
            mock_server_error(401, r#"{"success":false,"errors":["Unauthorized"]}"#).await;

        let client = Client::new("bad-key").unwrap().with_base_url(url);
        let err = client.search("test").await.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("401"),
            "error should mention 401: {err_msg}"
        );

        handle.abort();
    }

    #[test]
    fn client_new_succeeds() {
        let client = Client::new("valid-key");
        assert!(client.is_ok());
    }

    #[test]
    fn build_params_nil_filters() {
        let params = build_params(None, 0);
        assert!(params.is_empty());
    }

    #[test]
    fn build_params_page_only() {
        let params = build_params(None, 2);
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], ("page".into(), "2".into()));
    }

    #[test]
    fn build_params_style_filter() {
        let filters = ImageFilters {
            style: "alternate".into(),
            ..Default::default()
        };
        let params = build_params(Some(&filters), 0);
        assert!(
            params
                .iter()
                .any(|(k, v)| k == "styles" && v == "alternate")
        );
        assert!(params.iter().any(|(k, v)| k == "nsfw" && v == "false"));
        assert!(params.iter().any(|(k, v)| k == "humor" && v == "false"));
    }

    #[test]
    fn build_params_all_styles_ignored() {
        let filters = ImageFilters {
            style: "All Styles".into(),
            ..Default::default()
        };
        let params = build_params(Some(&filters), 0);
        assert!(!params.iter().any(|(k, _)| k == "styles"));
    }

    #[test]
    fn build_params_static_animation() {
        let filters = ImageFilters {
            image_type: "static".into(),
            ..Default::default()
        };
        let params = build_params(Some(&filters), 0);
        assert!(params.iter().any(|(k, v)| k == "types" && v == "static"));
    }

    #[test]
    fn build_params_animated_frontend_label() {
        let filters = ImageFilters {
            image_type: "Animated Only".into(),
            ..Default::default()
        };
        let params = build_params(Some(&filters), 0);
        assert!(params.iter().any(|(k, v)| k == "types" && v == "animated"));
    }

    #[test]
    fn build_params_nsfw_humor_enabled() {
        let filters = ImageFilters {
            show_nsfw: true,
            show_humor: true,
            ..Default::default()
        };
        let params = build_params(Some(&filters), 0);
        assert!(params.iter().any(|(k, v)| k == "nsfw" && v == "any"));
        assert!(params.iter().any(|(k, v)| k == "humor" && v == "any"));
    }

    #[test]
    fn build_params_mime_filter() {
        let filters = ImageFilters {
            mime_type: "image/png".into(),
            ..Default::default()
        };
        let params = build_params(Some(&filters), 0);
        assert!(params.iter().any(|(k, v)| k == "mimes" && v == "image/png"));
    }

    #[test]
    fn build_params_dimension_filter() {
        let filters = ImageFilters {
            dimension: "600x900".into(),
            ..Default::default()
        };
        let params = build_params(Some(&filters), 0);
        assert!(
            params
                .iter()
                .any(|(k, v)| k == "dimensions" && v == "600x900")
        );
    }
}
