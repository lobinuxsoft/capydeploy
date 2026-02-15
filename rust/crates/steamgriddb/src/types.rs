//! API response types for SteamGridDB.

use serde::{Deserialize, Serialize};

/// A game search result from the SteamGridDB API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: i32,
    pub name: String,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub verified: bool,
}

/// Image metadata from the SteamGridDB API.
///
/// Used for grids, heroes, logos, and icons (they share the same API schema).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ImageData {
    pub id: i32,
    #[serde(default)]
    pub score: i32,
    #[serde(default)]
    pub style: String,
    #[serde(default)]
    pub width: i32,
    #[serde(default)]
    pub height: i32,
    #[serde(default)]
    pub nsfw: bool,
    #[serde(default)]
    pub humor: bool,
    #[serde(default)]
    pub mime: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub thumb: String,
    #[serde(default)]
    pub lock: bool,
    #[serde(default)]
    pub epilepsy: bool,
    #[serde(default)]
    pub upvotes: i32,
    #[serde(default)]
    pub downvotes: i32,
}

/// Filters for image queries.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageFilters {
    #[serde(default)]
    pub style: String,
    #[serde(default)]
    pub mime_type: String,
    /// `"static"`, `"animated"`, `"Static Only"`, `"Animated Only"`, or empty for all.
    #[serde(default)]
    pub image_type: String,
    #[serde(default)]
    pub dimension: String,
    #[serde(default)]
    pub show_nsfw: bool,
    #[serde(default)]
    pub show_humor: bool,
}

/// API response wrapper (internal).
#[derive(Debug, Deserialize)]
pub(crate) struct ApiResponse<T> {
    #[allow(dead_code)]
    pub success: bool,
    #[serde(default)]
    #[allow(dead_code)]
    pub errors: Vec<String>,
    #[serde(default)]
    pub data: T,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_result_roundtrip() {
        let json = r#"{"id":42,"name":"Test Game","types":["steam"],"verified":true}"#;
        let result: SearchResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.id, 42);
        assert_eq!(result.name, "Test Game");
        assert!(result.verified);
        assert_eq!(result.types, vec!["steam"]);
    }

    #[test]
    fn search_result_defaults() {
        let json = r#"{"id":1,"name":"Minimal"}"#;
        let result: SearchResult = serde_json::from_str(json).unwrap();
        assert!(!result.verified);
        assert!(result.types.is_empty());
    }

    #[test]
    fn image_data_roundtrip() {
        let json = r#"{
            "id": 100,
            "score": 5,
            "style": "alternate",
            "width": 920,
            "height": 430,
            "nsfw": false,
            "humor": false,
            "mime": "image/png",
            "language": "en",
            "url": "https://example.com/grid.png",
            "thumb": "https://example.com/thumb.png",
            "lock": false,
            "epilepsy": false,
            "upvotes": 10,
            "downvotes": 2
        }"#;
        let img: ImageData = serde_json::from_str(json).unwrap();
        assert_eq!(img.id, 100);
        assert_eq!(img.width, 920);
        assert_eq!(img.height, 430);
        assert_eq!(img.style, "alternate");
        assert_eq!(img.mime, "image/png");
    }

    #[test]
    fn image_data_defaults() {
        let json = r#"{"id": 1}"#;
        let img: ImageData = serde_json::from_str(json).unwrap();
        assert_eq!(img.score, 0);
        assert!(img.url.is_empty());
        assert!(!img.nsfw);
    }

    #[test]
    fn image_filters_default() {
        let f = ImageFilters::default();
        assert!(f.style.is_empty());
        assert!(!f.show_nsfw);
        assert!(!f.show_humor);
    }

    #[test]
    fn image_filters_serde_camel_case() {
        let f = ImageFilters {
            mime_type: "image/png".into(),
            show_nsfw: true,
            ..Default::default()
        };
        let json = serde_json::to_string(&f).unwrap();
        assert!(json.contains("mimeType"));
        assert!(json.contains("showNsfw"));
    }

    #[test]
    fn api_response_parse() {
        let json = r#"{"success":true,"data":[{"id":1,"name":"Game"}]}"#;
        let resp: ApiResponse<Vec<SearchResult>> = serde_json::from_str(json).unwrap();
        assert!(resp.success);
        assert_eq!(resp.data.len(), 1);
        assert_eq!(resp.data[0].name, "Game");
    }

    #[test]
    fn api_response_with_errors() {
        let json = r#"{"success":false,"errors":["Unauthorized"],"data":[]}"#;
        let resp: ApiResponse<Vec<SearchResult>> = serde_json::from_str(json).unwrap();
        assert!(!resp.success);
        assert_eq!(resp.errors, vec!["Unauthorized"]);
    }
}
