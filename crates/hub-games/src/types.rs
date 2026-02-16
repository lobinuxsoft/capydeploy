//! Domain types for installed games management.

use serde::{Deserialize, Serialize};

/// An installed game on a remote agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledGame {
    pub name: String,
    pub path: String,
    pub size: String,
    pub app_id: u32,
}

/// Artwork URLs to update for an installed game.
///
/// Each field is a source string:
/// - `file://...` — local file path
/// - `http(s)://...` — remote URL (downloaded by the Hub)
/// - empty — skip this artwork type
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ArtworkUpdate {
    pub grid: String,
    pub banner: String,
    pub hero: String,
    pub logo: String,
    pub icon: String,
}
