use std::fmt;
use std::fs;
use std::path::PathBuf;

use crate::SteamError;

/// Type of Steam artwork.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtworkType {
    /// 460x215 horizontal banner.
    Grid,
    /// 1920x620 header.
    Hero,
    /// Transparent logo.
    Logo,
    /// Square icon.
    Icon,
    /// 600x900 vertical grid.
    Portrait,
}

impl ArtworkType {
    /// Returns all artwork types.
    pub fn all() -> &'static [ArtworkType] {
        &[
            ArtworkType::Grid,
            ArtworkType::Hero,
            ArtworkType::Logo,
            ArtworkType::Icon,
            ArtworkType::Portrait,
        ]
    }

    /// Returns the filename suffix for this artwork type.
    fn suffix(&self) -> &'static str {
        match self {
            ArtworkType::Grid => "",
            ArtworkType::Hero => "_hero",
            ArtworkType::Logo => "_logo",
            ArtworkType::Icon => "_icon",
            ArtworkType::Portrait => "p",
        }
    }
}

impl fmt::Display for ArtworkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtworkType::Grid => write!(f, "grid"),
            ArtworkType::Hero => write!(f, "hero"),
            ArtworkType::Logo => write!(f, "logo"),
            ArtworkType::Icon => write!(f, "icon"),
            ArtworkType::Portrait => write!(f, "portrait"),
        }
    }
}

/// Provides access to Steam directory paths.
pub struct Paths {
    base_dir: PathBuf,
}

impl Paths {
    /// Creates a new `Paths` instance with auto-detected Steam directory.
    pub fn new() -> Result<Self, SteamError> {
        let base_dir = get_base_dir()?;
        Ok(Self { base_dir })
    }

    /// Creates a new `Paths` instance with a custom base directory.
    pub fn with_base(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Returns the Steam base directory.
    pub fn base_dir(&self) -> &PathBuf {
        &self.base_dir
    }

    /// Returns the userdata directory.
    pub fn user_data_dir(&self) -> PathBuf {
        self.base_dir.join("userdata")
    }

    /// Returns the directory for a specific user.
    pub fn user_dir(&self, user_id: &str) -> PathBuf {
        self.user_data_dir().join(user_id)
    }

    /// Returns the config directory for a user.
    pub fn config_dir(&self, user_id: &str) -> PathBuf {
        self.user_dir(user_id).join("config")
    }

    /// Returns the path to shortcuts.vdf for a user.
    pub fn shortcuts_path(&self, user_id: &str) -> PathBuf {
        self.config_dir(user_id).join("shortcuts.vdf")
    }

    /// Returns the grid artwork directory for a user.
    pub fn grid_dir(&self, user_id: &str) -> PathBuf {
        self.config_dir(user_id).join("grid")
    }

    /// Returns true if the user has a shortcuts.vdf file.
    pub fn has_shortcuts(&self, user_id: &str) -> bool {
        self.shortcuts_path(user_id).exists()
    }

    /// Creates the grid directory if it doesn't exist.
    pub fn ensure_grid_dir(&self, user_id: &str) -> Result<(), SteamError> {
        fs::create_dir_all(self.grid_dir(user_id))
            .map_err(|e| SteamError::Io(format!("failed to create grid dir: {e}")))
    }

    /// Returns the path for a specific artwork type.
    pub fn artwork_path(
        &self,
        user_id: &str,
        app_id: u32,
        art_type: ArtworkType,
        ext: &str,
    ) -> PathBuf {
        self.grid_dir(user_id)
            .join(artwork_filename(app_id, art_type, ext))
    }
}

/// Generates the filename for artwork based on type.
fn artwork_filename(app_id: u32, art_type: ArtworkType, ext: &str) -> String {
    let ext = if ext.is_empty() { "png" } else { ext };
    format!("{}{}.{}", app_id, art_type.suffix(), ext)
}

// Platform-specific base directory detection.
#[cfg(target_os = "linux")]
fn get_base_dir() -> Result<PathBuf, SteamError> {
    crate::paths_linux::get_base_dir()
}

#[cfg(target_os = "windows")]
fn get_base_dir() -> Result<PathBuf, SteamError> {
    crate::paths_windows::get_base_dir()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn get_base_dir() -> Result<PathBuf, SteamError> {
    Err(SteamError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_with_base() {
        let paths = Paths::with_base("/tmp/steam");
        assert_eq!(paths.base_dir(), &PathBuf::from("/tmp/steam"));
        assert_eq!(paths.user_data_dir(), PathBuf::from("/tmp/steam/userdata"));
    }

    #[test]
    fn user_dir_structure() {
        let paths = Paths::with_base("/steam");
        assert_eq!(
            paths.user_dir("12345"),
            PathBuf::from("/steam/userdata/12345")
        );
        assert_eq!(
            paths.config_dir("12345"),
            PathBuf::from("/steam/userdata/12345/config")
        );
        assert_eq!(
            paths.shortcuts_path("12345"),
            PathBuf::from("/steam/userdata/12345/config/shortcuts.vdf")
        );
        assert_eq!(
            paths.grid_dir("12345"),
            PathBuf::from("/steam/userdata/12345/config/grid")
        );
    }

    #[test]
    fn artwork_filename_variants() {
        assert_eq!(artwork_filename(999, ArtworkType::Grid, "png"), "999.png");
        assert_eq!(
            artwork_filename(999, ArtworkType::Hero, "png"),
            "999_hero.png"
        );
        assert_eq!(
            artwork_filename(999, ArtworkType::Logo, "png"),
            "999_logo.png"
        );
        assert_eq!(
            artwork_filename(999, ArtworkType::Icon, "ico"),
            "999_icon.ico"
        );
        assert_eq!(
            artwork_filename(999, ArtworkType::Portrait, "png"),
            "999p.png"
        );
    }

    #[test]
    fn artwork_filename_default_extension() {
        assert_eq!(artwork_filename(123, ArtworkType::Grid, ""), "123.png");
    }

    #[test]
    fn artwork_type_display() {
        assert_eq!(ArtworkType::Grid.to_string(), "grid");
        assert_eq!(ArtworkType::Hero.to_string(), "hero");
        assert_eq!(ArtworkType::Portrait.to_string(), "portrait");
    }

    #[test]
    fn artwork_path_full() {
        let paths = Paths::with_base("/steam");
        let path = paths.artwork_path("12345", 999, ArtworkType::Hero, "png");
        assert_eq!(
            path,
            PathBuf::from("/steam/userdata/12345/config/grid/999_hero.png")
        );
    }
}
