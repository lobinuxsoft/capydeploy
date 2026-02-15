//! Artwork dialog state and logic.
//!
//! Manages search results, image selections per tab, thumbnail caching,
//! filter state, and preview handle resolution.

use std::collections::HashMap;

use capydeploy_steamgriddb::types::{ImageData, ImageFilters, SearchResult};

use crate::message::ArtworkFilterField;

use super::{ArtworkSelection, ArtworkTab};

/// Dialog state for the SteamGridDB artwork selector.
pub struct ArtworkDialog {
    pub search_query: String,
    pub search_results: Vec<SearchResult>,
    pub searching: bool,

    pub selected_game_id: Option<i32>,
    pub selected_game_name: String,

    pub active_tab: ArtworkTab,
    pub images: HashMap<ArtworkTab, Vec<ImageData>>,
    pub loading: HashMap<ArtworkTab, bool>,
    pub pages: HashMap<ArtworkTab, i32>,

    // Current selections (URLs).
    pub grid_portrait: String,
    pub grid_landscape: String,
    pub hero_image: String,
    pub logo_image: String,
    pub icon_image: String,
    pub griddb_game_id: i32,

    /// Cached thumbnail image handles, keyed by thumb URL or full URL.
    pub thumb_cache: HashMap<String, cosmic::iced::widget::image::Handle>,

    /// Maps full image URL → thumb URL for preview lookups.
    pub url_to_thumb: HashMap<String, String>,

    /// Current image filters applied to API queries.
    pub filters: ImageFilters,

    /// Whether the filter panel is visible.
    pub show_filters: bool,
}

impl ArtworkDialog {
    /// Creates a new artwork dialog, optionally pre-filled from existing setup values.
    pub fn new(
        game_name: &str,
        griddb_game_id: i32,
        grid_portrait: &str,
        grid_landscape: &str,
        hero_image: &str,
        logo_image: &str,
        icon_image: &str,
    ) -> Self {
        let mut dialog = Self {
            search_query: game_name.to_string(),
            search_results: Vec::new(),
            searching: false,
            selected_game_id: if griddb_game_id > 0 {
                Some(griddb_game_id)
            } else {
                None
            },
            selected_game_name: game_name.to_string(),
            active_tab: ArtworkTab::Capsule,
            images: HashMap::new(),
            loading: HashMap::new(),
            pages: HashMap::new(),
            grid_portrait: grid_portrait.to_string(),
            grid_landscape: grid_landscape.to_string(),
            hero_image: hero_image.to_string(),
            logo_image: logo_image.to_string(),
            icon_image: icon_image.to_string(),
            griddb_game_id,
            thumb_cache: HashMap::new(),
            url_to_thumb: HashMap::new(),
            filters: ImageFilters {
                show_humor: true,
                ..Default::default()
            },
            show_filters: false,
        };

        // Pre-cache handles for local file selections.
        for url in dialog.local_file_urls() {
            let path = &url[7..]; // strip "file://"
            let handle = cosmic::iced::widget::image::Handle::from_path(path);
            dialog.thumb_cache.insert(url, handle);
        }

        dialog
    }

    /// Returns all current selections that are local files.
    fn local_file_urls(&self) -> Vec<String> {
        self.all_urls()
            .filter(|s| s.starts_with("file://"))
            .map(|s| s.to_string())
            .collect()
    }

    /// Iterator over all five selection URLs.
    fn all_urls(&self) -> impl Iterator<Item = &str> {
        [
            self.grid_portrait.as_str(),
            self.grid_landscape.as_str(),
            self.hero_image.as_str(),
            self.logo_image.as_str(),
            self.icon_image.as_str(),
        ]
        .into_iter()
    }

    /// Builds the final selection from current state.
    pub fn selection(&self) -> ArtworkSelection {
        ArtworkSelection {
            griddb_game_id: self.griddb_game_id,
            grid_portrait: self.grid_portrait.clone(),
            grid_landscape: self.grid_landscape.clone(),
            hero_image: self.hero_image.clone(),
            logo_image: self.logo_image.clone(),
            icon_image: self.icon_image.clone(),
        }
    }

    /// Sets the selected URL for the current tab.
    pub fn select_image(&mut self, tab: ArtworkTab, url: &str) {
        match tab {
            ArtworkTab::Capsule => self.grid_portrait = url.to_string(),
            ArtworkTab::Wide => self.grid_landscape = url.to_string(),
            ArtworkTab::Hero => self.hero_image = url.to_string(),
            ArtworkTab::Logo => self.logo_image = url.to_string(),
            ArtworkTab::Icon => self.icon_image = url.to_string(),
        }
    }

    /// Returns the currently selected URL for a tab.
    pub(crate) fn selected_url(&self, tab: ArtworkTab) -> &str {
        match tab {
            ArtworkTab::Capsule => &self.grid_portrait,
            ArtworkTab::Wide => &self.grid_landscape,
            ArtworkTab::Hero => &self.hero_image,
            ArtworkTab::Logo => &self.logo_image,
            ArtworkTab::Icon => &self.icon_image,
        }
    }

    /// Count of non-empty selections.
    pub(crate) fn selection_count(&self) -> usize {
        self.all_urls().filter(|s| !s.is_empty()).count()
    }

    // -- Thumbnail cache --

    /// Returns uncached thumbnail URLs for the given images and builds
    /// the `url_to_thumb` mapping.
    pub fn uncached_thumb_urls(&mut self, images: &[ImageData]) -> Vec<String> {
        let mut to_fetch = Vec::new();
        for img in images {
            let thumb = if !img.thumb.is_empty() {
                &img.thumb
            } else if !img.url.is_empty() {
                &img.url
            } else {
                continue;
            };

            // Build reverse mapping: full URL → thumb URL.
            if !img.url.is_empty() && !img.thumb.is_empty() {
                self.url_to_thumb
                    .insert(img.url.clone(), img.thumb.clone());
            }

            if !self.thumb_cache.contains_key(thumb) {
                to_fetch.push(thumb.clone());
            }
        }
        to_fetch
    }

    /// Inserts a batch of fetched thumbnails into the cache.
    pub fn insert_thumbnails(&mut self, batch: Vec<(String, Vec<u8>)>) {
        for (url, bytes) in batch {
            let handle = cosmic::iced::widget::image::Handle::from_bytes(bytes);
            self.thumb_cache.insert(url, handle);
        }
    }

    /// Returns the cached image handle for a selection URL, if available.
    ///
    /// Tries direct lookup, then `url_to_thumb` reverse mapping.
    pub fn preview_handle(&self, url: &str) -> Option<&cosmic::iced::widget::image::Handle> {
        if let Some(h) = self.thumb_cache.get(url) {
            return Some(h);
        }
        if let Some(thumb) = self.url_to_thumb.get(url) {
            if let Some(h) = self.thumb_cache.get(thumb) {
                return Some(h);
            }
        }
        None
    }

    /// Returns all current selection URLs that need their previews fetched
    /// (remote URLs that aren't cached yet).
    pub fn uncached_selection_urls(&self) -> Vec<String> {
        self.all_urls()
            .filter(|s| !s.is_empty() && !s.starts_with("file://") && self.preview_handle(s).is_none())
            .map(|s| s.to_string())
            .collect()
    }

    // -- Filters --

    /// Toggles a CSV value in a filter field. If the value is present, removes
    /// it; if absent, adds it.
    pub fn toggle_filter(&mut self, field: &ArtworkFilterField, value: &str) {
        let csv = match field {
            ArtworkFilterField::Style => &mut self.filters.style,
            ArtworkFilterField::Dimension => &mut self.filters.dimension,
            ArtworkFilterField::MimeType => &mut self.filters.mime_type,
            ArtworkFilterField::ImageType => &mut self.filters.image_type,
        };

        // For ImageType, it's a single value toggle (not CSV).
        if *field == ArtworkFilterField::ImageType {
            if csv == value {
                csv.clear();
            } else {
                *csv = value.to_string();
            }
            return;
        }

        let mut parts: Vec<&str> = csv.split(',').filter(|s| !s.is_empty()).collect();
        if let Some(pos) = parts.iter().position(|&s| s == value) {
            parts.remove(pos);
        } else {
            parts.push(value);
        }
        *csv = parts.join(",");
    }

    /// Resets all filters to their defaults.
    pub fn reset_filters(&mut self) {
        self.filters = ImageFilters {
            show_humor: true,
            ..Default::default()
        };
    }

    /// Whether any filter is active (non-default).
    pub fn has_active_filters(&self) -> bool {
        !self.filters.style.is_empty()
            || !self.filters.mime_type.is_empty()
            || !self.filters.dimension.is_empty()
            || !self.filters.image_type.is_empty()
            || self.filters.show_nsfw
            || !self.filters.show_humor
    }

    /// Whether a CSV filter value is currently selected.
    pub fn is_filter_selected(&self, field: &ArtworkFilterField, value: &str) -> bool {
        let csv = match field {
            ArtworkFilterField::Style => &self.filters.style,
            ArtworkFilterField::Dimension => &self.filters.dimension,
            ArtworkFilterField::MimeType => &self.filters.mime_type,
            ArtworkFilterField::ImageType => &self.filters.image_type,
        };

        if *field == ArtworkFilterField::ImageType {
            return csv == value;
        }

        csv.split(',').any(|s| s == value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_new_defaults() {
        let dlg = ArtworkDialog::new("Test Game", 0, "", "", "", "", "");
        assert_eq!(dlg.search_query, "Test Game");
        assert!(dlg.selected_game_id.is_none());
        assert_eq!(dlg.active_tab, ArtworkTab::Capsule);
        assert!(dlg.grid_portrait.is_empty());
        assert!(dlg.grid_landscape.is_empty());
        assert!(dlg.hero_image.is_empty());
        assert!(dlg.logo_image.is_empty());
        assert!(dlg.icon_image.is_empty());
        assert!(dlg.thumb_cache.is_empty());
    }

    #[test]
    fn dialog_new_with_existing_game_id() {
        let dlg = ArtworkDialog::new("Portal 2", 42, "p.png", "w.png", "h.png", "l.png", "i.png");
        assert_eq!(dlg.selected_game_id, Some(42));
        assert_eq!(dlg.griddb_game_id, 42);
        assert_eq!(dlg.grid_portrait, "p.png");
        assert_eq!(dlg.grid_landscape, "w.png");
        assert_eq!(dlg.hero_image, "h.png");
        assert_eq!(dlg.logo_image, "l.png");
        assert_eq!(dlg.icon_image, "i.png");
    }

    #[test]
    fn select_image_per_tab() {
        let mut dlg = ArtworkDialog::new("Test", 0, "", "", "", "", "");

        dlg.select_image(ArtworkTab::Capsule, "capsule.png");
        assert_eq!(dlg.grid_portrait, "capsule.png");

        dlg.select_image(ArtworkTab::Wide, "wide.png");
        assert_eq!(dlg.grid_landscape, "wide.png");

        dlg.select_image(ArtworkTab::Hero, "hero.png");
        assert_eq!(dlg.hero_image, "hero.png");

        dlg.select_image(ArtworkTab::Logo, "logo.png");
        assert_eq!(dlg.logo_image, "logo.png");

        dlg.select_image(ArtworkTab::Icon, "icon.png");
        assert_eq!(dlg.icon_image, "icon.png");
    }

    #[test]
    fn selection_builds_correctly() {
        let mut dlg = ArtworkDialog::new("Test", 99, "", "", "", "", "");
        dlg.select_image(ArtworkTab::Capsule, "c.png");
        dlg.select_image(ArtworkTab::Hero, "h.png");

        let sel = dlg.selection();
        assert_eq!(sel.griddb_game_id, 99);
        assert_eq!(sel.grid_portrait, "c.png");
        assert!(sel.grid_landscape.is_empty());
        assert_eq!(sel.hero_image, "h.png");
    }

    #[test]
    fn selection_count() {
        let mut dlg = ArtworkDialog::new("Test", 0, "", "", "", "", "");
        assert_eq!(dlg.selection_count(), 0);

        dlg.select_image(ArtworkTab::Capsule, "a.png");
        assert_eq!(dlg.selection_count(), 1);

        dlg.select_image(ArtworkTab::Wide, "b.png");
        dlg.select_image(ArtworkTab::Hero, "c.png");
        dlg.select_image(ArtworkTab::Logo, "d.png");
        dlg.select_image(ArtworkTab::Icon, "e.png");
        assert_eq!(dlg.selection_count(), 5);
    }

    #[test]
    fn uncached_thumb_urls_filters_correctly() {
        use capydeploy_steamgriddb::types::ImageData;

        let mut dlg = ArtworkDialog::new("Test", 0, "", "", "", "", "");

        let images = vec![
            ImageData {
                thumb: "http://thumb1.jpg".into(),
                url: "http://full1.jpg".into(),
                ..Default::default()
            },
            ImageData {
                thumb: "http://thumb2.jpg".into(),
                url: "http://full2.jpg".into(),
                ..Default::default()
            },
        ];

        let urls = dlg.uncached_thumb_urls(&images);
        assert_eq!(urls.len(), 2);

        dlg.thumb_cache.insert(
            "http://thumb1.jpg".into(),
            cosmic::iced::widget::image::Handle::from_bytes(vec![]),
        );

        let urls = dlg.uncached_thumb_urls(&images);
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0], "http://thumb2.jpg");

        assert_eq!(
            dlg.url_to_thumb.get("http://full1.jpg"),
            Some(&"http://thumb1.jpg".to_string())
        );
        assert!(dlg.preview_handle("http://full1.jpg").is_some());
        assert!(dlg.preview_handle("http://full2.jpg").is_none());
    }

    #[test]
    fn toggle_filter_csv_add_remove() {
        let mut dlg = ArtworkDialog::new("Test", 0, "", "", "", "", "");

        dlg.toggle_filter(&ArtworkFilterField::Style, "alternate");
        assert_eq!(dlg.filters.style, "alternate");
        assert!(dlg.is_filter_selected(&ArtworkFilterField::Style, "alternate"));

        dlg.toggle_filter(&ArtworkFilterField::Style, "blurred");
        assert_eq!(dlg.filters.style, "alternate,blurred");

        dlg.toggle_filter(&ArtworkFilterField::Style, "alternate");
        assert_eq!(dlg.filters.style, "blurred");

        dlg.toggle_filter(&ArtworkFilterField::Style, "blurred");
        assert!(dlg.filters.style.is_empty());
    }

    #[test]
    fn toggle_filter_image_type_single_value() {
        let mut dlg = ArtworkDialog::new("Test", 0, "", "", "", "", "");

        dlg.toggle_filter(&ArtworkFilterField::ImageType, "Static Only");
        assert_eq!(dlg.filters.image_type, "Static Only");

        dlg.toggle_filter(&ArtworkFilterField::ImageType, "Static Only");
        assert!(dlg.filters.image_type.is_empty());

        dlg.toggle_filter(&ArtworkFilterField::ImageType, "Animated Only");
        dlg.toggle_filter(&ArtworkFilterField::ImageType, "Static Only");
        assert_eq!(dlg.filters.image_type, "Static Only");
    }

    #[test]
    fn has_active_filters_defaults() {
        let dlg = ArtworkDialog::new("Test", 0, "", "", "", "", "");
        assert!(!dlg.has_active_filters());
    }

    #[test]
    fn has_active_filters_detects_changes() {
        let mut dlg = ArtworkDialog::new("Test", 0, "", "", "", "", "");

        dlg.toggle_filter(&ArtworkFilterField::Style, "alternate");
        assert!(dlg.has_active_filters());

        dlg.reset_filters();
        assert!(!dlg.has_active_filters());

        dlg.filters.show_nsfw = true;
        assert!(dlg.has_active_filters());

        dlg.reset_filters();
        dlg.filters.show_humor = false;
        assert!(dlg.has_active_filters());
    }

    #[test]
    fn reset_filters_restores_defaults() {
        let mut dlg = ArtworkDialog::new("Test", 0, "", "", "", "", "");
        dlg.filters.style = "alternate".into();
        dlg.filters.dimension = "600x900".into();
        dlg.filters.show_nsfw = true;
        dlg.filters.show_humor = false;

        dlg.reset_filters();
        assert!(dlg.filters.style.is_empty());
        assert!(dlg.filters.dimension.is_empty());
        assert!(!dlg.filters.show_nsfw);
        assert!(dlg.filters.show_humor);
    }
}
