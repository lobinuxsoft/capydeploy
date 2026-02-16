//! Artwork selection, classification, and config building.
//!
//! Wraps the SteamGridDB client for artwork search and provides
//! utilities for classifying artwork sources and building protocol configs.

use std::path::Path;

use capydeploy_protocol::types::ArtworkConfig;
use capydeploy_steamgriddb::Client as SteamGridDbClient;
use capydeploy_steamgriddb::types::{ImageData, ImageFilters, SearchResult};

use crate::error::DeployError;
use crate::types::{ArtworkAssignment, ArtworkSource, GameSetup, LocalArtwork};

/// Wrapper around the SteamGridDB client for artwork search and download.
pub struct ArtworkSelector {
    client: Option<SteamGridDbClient>,
}

impl ArtworkSelector {
    /// Creates a new selector with an optional SteamGridDB client.
    pub fn new(client: Option<SteamGridDbClient>) -> Self {
        Self { client }
    }

    /// Searches for games on SteamGridDB.
    pub async fn search_games(&self, term: &str) -> Result<Vec<SearchResult>, DeployError> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DeployError::Artwork("SteamGridDB client not configured".into()))?;
        Ok(client.search(term).await?)
    }

    /// Returns artwork images for a game by type.
    pub async fn get_artwork(
        &self,
        game_id: i32,
        art_type: &str,
        filters: Option<&ImageFilters>,
        page: i32,
    ) -> Result<Vec<ImageData>, DeployError> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DeployError::Artwork("SteamGridDB client not configured".into()))?;

        let result = match art_type {
            "grid" => client.get_grids(game_id, filters, page).await?,
            "hero" => client.get_heroes(game_id, filters, page).await?,
            "logo" => client.get_logos(game_id, filters, page).await?,
            "icon" => client.get_icons(game_id, filters, page).await?,
            other => {
                return Err(DeployError::Artwork(format!(
                    "unknown artwork type: {other}"
                )));
            }
        };

        Ok(result)
    }

    /// Downloads an image from a URL.
    pub async fn download_image(&self, url: &str) -> Result<Vec<u8>, DeployError> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| DeployError::Artwork("SteamGridDB client not configured".into()))?;
        Ok(client.download_image(url).await?)
    }
}

/// Classifies an artwork path as Remote, Local, or None.
pub fn classify_artwork_source(path: &str) -> ArtworkSource {
    if path.is_empty() {
        ArtworkSource::None
    } else if path.starts_with("http://") || path.starts_with("https://") {
        ArtworkSource::Remote(path.to_string())
    } else if path.starts_with("file://") {
        ArtworkSource::Local(path.strip_prefix("file://").unwrap_or(path).to_string())
    } else {
        // Treat bare paths as local files.
        ArtworkSource::Local(path.to_string())
    }
}

/// Builds an `ArtworkAssignment` from a `GameSetup`.
pub fn build_artwork_assignment(setup: &GameSetup) -> ArtworkAssignment {
    ArtworkAssignment {
        grid: classify_artwork_source(&setup.grid_portrait),
        banner: classify_artwork_source(&setup.grid_landscape),
        hero: classify_artwork_source(&setup.hero_image),
        logo: classify_artwork_source(&setup.logo_image),
        icon: classify_artwork_source(&setup.icon_image),
    }
}

/// Builds a protocol `ArtworkConfig` with only remote (HTTP) URLs.
///
/// Local `file://` artwork is excluded — those are sent as binary WS messages.
/// Returns `None` if no remote artwork is configured.
pub fn build_remote_artwork_config(assignment: &ArtworkAssignment) -> Option<ArtworkConfig> {
    let mut cfg = ArtworkConfig {
        grid: String::new(),
        hero: String::new(),
        logo: String::new(),
        icon: String::new(),
        banner: String::new(),
    };
    let mut has_any = false;

    if let ArtworkSource::Remote(url) = &assignment.grid {
        cfg.grid = url.clone();
        has_any = true;
    }
    if let ArtworkSource::Remote(url) = &assignment.banner {
        cfg.banner = url.clone();
        has_any = true;
    }
    if let ArtworkSource::Remote(url) = &assignment.hero {
        cfg.hero = url.clone();
        has_any = true;
    }
    if let ArtworkSource::Remote(url) = &assignment.logo {
        cfg.logo = url.clone();
        has_any = true;
    }
    if let ArtworkSource::Remote(url) = &assignment.icon {
        cfg.icon = url.clone();
        has_any = true;
    }

    if has_any { Some(cfg) } else { None }
}

/// Collects local artwork files that need to be sent via binary WS messages.
///
/// Reads each `file://` path, detects content type, and returns the data.
pub fn collect_local_artwork(assignment: &ArtworkAssignment) -> Vec<LocalArtwork> {
    let slots: [(&str, &ArtworkSource); 5] = [
        ("grid", &assignment.grid),
        ("banner", &assignment.banner),
        ("hero", &assignment.hero),
        ("logo", &assignment.logo),
        ("icon", &assignment.icon),
    ];

    let mut result = Vec::new();

    for (art_type, source) in slots {
        if let ArtworkSource::Local(path) = source {
            let content_type = match detect_content_type(path) {
                Some(ct) => ct.to_string(),
                None => {
                    tracing::warn!("unknown content type for artwork: {path}");
                    continue;
                }
            };

            match std::fs::read(path) {
                Ok(data) => {
                    result.push(LocalArtwork {
                        art_type: art_type.to_string(),
                        content_type,
                        data,
                    });
                }
                Err(e) => {
                    tracing::warn!("failed to read local artwork {path}: {e}");
                }
            }
        }
    }

    result
}

/// Detects MIME content type from a file path extension.
pub fn detect_content_type(path: &str) -> Option<&'static str> {
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

/// Parses a comma-separated tag string into a vector of trimmed tags.
pub fn parse_tags(tags_str: &str) -> Vec<String> {
    if tags_str.is_empty() {
        return Vec::new();
    }
    tags_str
        .split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

/// Builds a protocol `ShortcutConfig` from a game setup and artwork assignment.
pub fn build_shortcut_config(
    setup: &GameSetup,
    artwork: &ArtworkAssignment,
) -> capydeploy_protocol::types::ShortcutConfig {
    capydeploy_protocol::types::ShortcutConfig {
        name: setup.name.clone(),
        exe: setup.executable.clone(),
        start_dir: String::new(),
        launch_options: setup.launch_options.clone(),
        tags: parse_tags(&setup.tags),
        artwork: build_remote_artwork_config(artwork),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_http_url() {
        assert_eq!(
            classify_artwork_source("https://cdn.example.com/grid.png"),
            ArtworkSource::Remote("https://cdn.example.com/grid.png".into())
        );
        assert_eq!(
            classify_artwork_source("http://example.com/hero.jpg"),
            ArtworkSource::Remote("http://example.com/hero.jpg".into())
        );
    }

    #[test]
    fn classify_file_url() {
        assert_eq!(
            classify_artwork_source("file:///home/user/art.png"),
            ArtworkSource::Local("/home/user/art.png".into())
        );
    }

    #[test]
    fn classify_empty() {
        assert_eq!(classify_artwork_source(""), ArtworkSource::None);
    }

    #[test]
    fn classify_bare_path() {
        assert_eq!(
            classify_artwork_source("/some/path/image.png"),
            ArtworkSource::Local("/some/path/image.png".into())
        );
    }

    #[test]
    fn detect_content_type_known() {
        assert_eq!(detect_content_type("image.png"), Some("image/png"));
        assert_eq!(detect_content_type("photo.jpg"), Some("image/jpeg"));
        assert_eq!(detect_content_type("photo.jpeg"), Some("image/jpeg"));
        assert_eq!(detect_content_type("art.webp"), Some("image/webp"));
        assert_eq!(detect_content_type("anim.gif"), Some("image/gif"));
        assert_eq!(detect_content_type("favicon.ico"), Some("image/x-icon"));
    }

    #[test]
    fn detect_content_type_unknown() {
        assert_eq!(detect_content_type("document.pdf"), None);
        assert_eq!(detect_content_type("noext"), None);
        assert_eq!(detect_content_type(""), None);
    }

    #[test]
    fn detect_content_type_case_insensitive() {
        assert_eq!(detect_content_type("IMAGE.PNG"), Some("image/png"));
        assert_eq!(detect_content_type("Photo.JPG"), Some("image/jpeg"));
    }

    #[test]
    fn parse_tags_basic() {
        let tags = parse_tags("Action, RPG, Indie");
        assert_eq!(tags, vec!["Action", "RPG", "Indie"]);
    }

    #[test]
    fn parse_tags_empty() {
        let tags = parse_tags("");
        assert!(tags.is_empty());
    }

    #[test]
    fn parse_tags_trims_whitespace() {
        let tags = parse_tags("  Action ,  RPG  , , Indie ");
        assert_eq!(tags, vec!["Action", "RPG", "Indie"]);
    }

    #[test]
    fn build_remote_config_only_http() {
        let assignment = ArtworkAssignment {
            grid: ArtworkSource::Remote("https://cdn.com/grid.png".into()),
            banner: ArtworkSource::Local("/local/banner.png".into()),
            hero: ArtworkSource::Remote("https://cdn.com/hero.png".into()),
            logo: ArtworkSource::None,
            icon: ArtworkSource::None,
        };

        let cfg = build_remote_artwork_config(&assignment).unwrap();
        assert_eq!(cfg.grid, "https://cdn.com/grid.png");
        assert!(cfg.banner.is_empty());
        assert_eq!(cfg.hero, "https://cdn.com/hero.png");
        assert!(cfg.logo.is_empty());
        assert!(cfg.icon.is_empty());
    }

    #[test]
    fn build_remote_config_none_when_all_local() {
        let assignment = ArtworkAssignment {
            grid: ArtworkSource::Local("/local/grid.png".into()),
            banner: ArtworkSource::None,
            hero: ArtworkSource::None,
            logo: ArtworkSource::None,
            icon: ArtworkSource::None,
        };

        assert!(build_remote_artwork_config(&assignment).is_none());
    }

    #[test]
    fn build_artwork_assignment_from_setup() {
        let setup = GameSetup {
            id: "g1".into(),
            name: "Test".into(),
            local_path: "/games/test".into(),
            executable: "game.exe".into(),
            launch_options: String::new(),
            tags: String::new(),
            install_path: "/home/deck/Games/test".into(),
            griddb_game_id: 0,
            grid_portrait: "https://cdn.com/grid.png".into(),
            grid_landscape: "file:///home/user/banner.png".into(),
            hero_image: String::new(),
            logo_image: "https://cdn.com/logo.png".into(),
            icon_image: String::new(),
        };

        let assignment = build_artwork_assignment(&setup);
        assert!(matches!(assignment.grid, ArtworkSource::Remote(_)));
        assert!(matches!(assignment.banner, ArtworkSource::Local(_)));
        assert_eq!(assignment.hero, ArtworkSource::None);
        assert!(matches!(assignment.logo, ArtworkSource::Remote(_)));
        assert_eq!(assignment.icon, ArtworkSource::None);
    }

    #[test]
    fn build_shortcut_config_populates_fields() {
        let setup = GameSetup {
            id: "g1".into(),
            name: "My Game".into(),
            local_path: "/games/my".into(),
            executable: "my_game.exe".into(),
            launch_options: "-fullscreen".into(),
            tags: "Action, RPG".into(),
            install_path: "/home/deck/Games/my".into(),
            griddb_game_id: 0,
            grid_portrait: "https://cdn.com/grid.png".into(),
            grid_landscape: String::new(),
            hero_image: String::new(),
            logo_image: String::new(),
            icon_image: String::new(),
        };
        let assignment = build_artwork_assignment(&setup);
        let sc = build_shortcut_config(&setup, &assignment);

        assert_eq!(sc.name, "My Game");
        assert_eq!(sc.exe, "my_game.exe");
        assert_eq!(sc.launch_options, "-fullscreen");
        assert_eq!(sc.tags, vec!["Action", "RPG"]);
        assert!(sc.artwork.is_some());
        assert_eq!(sc.artwork.unwrap().grid, "https://cdn.com/grid.png");
    }

    #[test]
    fn collect_local_artwork_reads_files() {
        let dir = tempfile::tempdir().unwrap();
        let img_path = dir.path().join("hero.png");
        std::fs::write(&img_path, b"PNG_DATA").unwrap();

        let assignment = ArtworkAssignment {
            grid: ArtworkSource::None,
            banner: ArtworkSource::None,
            hero: ArtworkSource::Local(img_path.to_string_lossy().into_owned()),
            logo: ArtworkSource::None,
            icon: ArtworkSource::None,
        };

        let locals = collect_local_artwork(&assignment);
        assert_eq!(locals.len(), 1);
        assert_eq!(locals[0].art_type, "hero");
        assert_eq!(locals[0].content_type, "image/png");
        assert_eq!(locals[0].data, b"PNG_DATA");
    }

    #[test]
    fn collect_local_artwork_skips_missing_files() {
        let assignment = ArtworkAssignment {
            grid: ArtworkSource::Local("/nonexistent/path/grid.png".into()),
            banner: ArtworkSource::None,
            hero: ArtworkSource::None,
            logo: ArtworkSource::None,
            icon: ArtworkSource::None,
        };

        let locals = collect_local_artwork(&assignment);
        assert!(locals.is_empty());
    }
}
