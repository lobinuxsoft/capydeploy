//! SteamGridDB artwork selector dialog.
//!
//! Split into submodules:
//! - [`dialog`]: Dialog state and logic (filters, selections, thumbnail cache).
//! - [`view`]: All rendering functions (panels, grids, cards, styles).

mod dialog;
mod view;

pub use dialog::ArtworkDialog;
pub use view::view;

/// Artwork category tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtworkTab {
    Capsule,
    Wide,
    Hero,
    Logo,
    Icon,
}

impl ArtworkTab {
    pub const ALL: [ArtworkTab; 5] = [
        Self::Capsule,
        Self::Wide,
        Self::Hero,
        Self::Logo,
        Self::Icon,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Capsule => "Capsule",
            Self::Wide => "Wide",
            Self::Hero => "Hero",
            Self::Logo => "Logo",
            Self::Icon => "Icon",
        }
    }

    /// Card width in pixels for the thumbnail grid layout.
    pub(crate) fn card_width(&self) -> f32 {
        match self {
            Self::Capsule => 120.0,
            Self::Wide => 180.0,
            Self::Hero => 220.0,
            Self::Logo => 110.0,
            Self::Icon => 80.0,
        }
    }

    /// Thumbnail height in pixels for the grid layout.
    pub(crate) fn thumb_height(&self) -> f32 {
        match self {
            Self::Capsule => 180.0, // 2:3 portrait
            Self::Wide => 84.0,     // ~2.14:1
            Self::Hero => 71.0,     // ~3.1:1
            Self::Logo => 110.0,    // square
            Self::Icon => 80.0,     // square
        }
    }

    /// Available style options for this asset type.
    pub fn style_options(&self) -> &'static [&'static str] {
        match self {
            Self::Capsule | Self::Wide => {
                &["alternate", "white_logo", "no_logo", "blurred", "material"]
            }
            Self::Hero => &["alternate", "blurred", "material"],
            Self::Logo => &["official", "white", "black", "custom"],
            Self::Icon => &["official", "custom"],
        }
    }

    /// Available dimension options for this asset type.
    pub fn dimension_options(&self) -> &'static [&'static str] {
        match self {
            Self::Capsule => &["600x900", "342x482", "660x930", "512x512", "1024x1024"],
            Self::Wide => &["460x215", "920x430"],
            Self::Hero => &["1920x620", "3840x1240", "1600x650"],
            Self::Logo => &[],
            Self::Icon => &["512x512", "256x256", "128x128", "64x64", "32x32"],
        }
    }

    /// Available MIME type options for this asset type.
    pub fn mime_options(&self) -> &'static [&'static str] {
        match self {
            Self::Capsule | Self::Wide | Self::Hero => &["image/png", "image/jpeg", "image/webp"],
            Self::Logo => &["image/png", "image/webp"],
            Self::Icon => &["image/png", "image/vnd.microsoft.icon"],
        }
    }
}

/// The artwork selections resulting from the dialog.
#[derive(Debug, Clone, Default)]
pub struct ArtworkSelection {
    pub griddb_game_id: i32,
    pub grid_portrait: String,
    pub grid_landscape: String,
    pub hero_image: String,
    pub logo_image: String,
    pub icon_image: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artwork_tab_all_contains_every_variant() {
        assert_eq!(ArtworkTab::ALL.len(), 5);
        assert_eq!(ArtworkTab::ALL[0], ArtworkTab::Capsule);
        assert_eq!(ArtworkTab::ALL[1], ArtworkTab::Wide);
        assert_eq!(ArtworkTab::ALL[2], ArtworkTab::Hero);
        assert_eq!(ArtworkTab::ALL[3], ArtworkTab::Logo);
        assert_eq!(ArtworkTab::ALL[4], ArtworkTab::Icon);
    }

    #[test]
    fn artwork_tab_labels() {
        assert_eq!(ArtworkTab::Capsule.label(), "Capsule");
        assert_eq!(ArtworkTab::Wide.label(), "Wide");
        assert_eq!(ArtworkTab::Hero.label(), "Hero");
        assert_eq!(ArtworkTab::Logo.label(), "Logo");
        assert_eq!(ArtworkTab::Icon.label(), "Icon");
    }

    #[test]
    fn artwork_selection_default_empty() {
        let sel = ArtworkSelection::default();
        assert_eq!(sel.griddb_game_id, 0);
        assert!(sel.grid_portrait.is_empty());
        assert!(sel.grid_landscape.is_empty());
        assert!(sel.hero_image.is_empty());
        assert!(sel.logo_image.is_empty());
        assert!(sel.icon_image.is_empty());
    }

    #[test]
    fn tab_style_options_per_type() {
        assert_eq!(ArtworkTab::Capsule.style_options().len(), 5);
        assert_eq!(ArtworkTab::Hero.style_options().len(), 3);
        assert_eq!(ArtworkTab::Logo.style_options().len(), 4);
        assert_eq!(ArtworkTab::Icon.style_options().len(), 2);
    }

    #[test]
    fn tab_dimension_options_per_type() {
        assert_eq!(ArtworkTab::Capsule.dimension_options().len(), 5);
        assert_eq!(ArtworkTab::Wide.dimension_options().len(), 2);
        assert_eq!(ArtworkTab::Hero.dimension_options().len(), 3);
        assert!(ArtworkTab::Logo.dimension_options().is_empty());
        assert_eq!(ArtworkTab::Icon.dimension_options().len(), 5);
    }
}
