use std::collections::HashMap;
use std::fs;

use capydeploy_protocol::{ShortcutConfig, ShortcutInfo};
use crc32fast::Hasher;

use crate::SteamError;
use crate::paths::{ArtworkType, Paths};

/// Handles Steam shortcut operations and artwork management.
pub struct ShortcutManager {
    paths: Paths,
}

impl ShortcutManager {
    /// Creates a new `ShortcutManager` with auto-detected Steam paths.
    pub fn new() -> Result<Self, SteamError> {
        Ok(Self {
            paths: Paths::new()?,
        })
    }

    /// Creates a `ShortcutManager` with custom paths.
    pub fn with_paths(paths: Paths) -> Self {
        Self { paths }
    }

    /// Returns the shortcuts.vdf path for a user.
    pub fn shortcuts_path(&self, user_id: &str) -> String {
        self.paths
            .shortcuts_path(user_id)
            .to_string_lossy()
            .into_owned()
    }

    /// Returns the grid artwork directory for a user.
    pub fn grid_dir(&self, user_id: &str) -> String {
        self.paths.grid_dir(user_id).to_string_lossy().into_owned()
    }

    /// Creates the grid directory if it doesn't exist.
    pub fn ensure_grid_dir(&self, user_id: &str) -> Result<(), SteamError> {
        self.paths.ensure_grid_dir(user_id)
    }

    /// Returns all artwork paths for a shortcut.
    pub fn artwork_paths(&self, user_id: &str, app_id: u32) -> HashMap<ArtworkType, String> {
        let mut map = HashMap::new();
        for &art_type in ArtworkType::all() {
            let path = self.paths.artwork_path(user_id, app_id, art_type, "png");
            map.insert(art_type, path.to_string_lossy().into_owned());
        }
        map
    }

    /// Finds existing artwork files for an app ID.
    pub fn find_existing_artwork(
        &self,
        user_id: &str,
        app_id: u32,
    ) -> Result<HashMap<ArtworkType, String>, SteamError> {
        let grid_dir = self.paths.grid_dir(user_id);
        let mut result = HashMap::new();

        let extensions = ["png", "jpg", "jpeg", "ico"];
        let art_types = [
            (ArtworkType::Grid, format!("{app_id}")),
            (ArtworkType::Hero, format!("{app_id}_hero")),
            (ArtworkType::Logo, format!("{app_id}_logo")),
            (ArtworkType::Icon, format!("{app_id}_icon")),
            (ArtworkType::Portrait, format!("{app_id}p")),
        ];

        for (art_type, base_name) in &art_types {
            for ext in &extensions {
                let path = grid_dir.join(format!("{base_name}.{ext}"));
                if path.exists() {
                    result.insert(*art_type, path.to_string_lossy().into_owned());
                    break;
                }
            }
        }

        Ok(result)
    }

    /// Saves artwork data to the appropriate path.
    pub fn save_artwork(
        &self,
        user_id: &str,
        app_id: u32,
        art_type: ArtworkType,
        data: &[u8],
        ext: &str,
    ) -> Result<(), SteamError> {
        self.ensure_grid_dir(user_id)?;

        let ext = ext.trim_start_matches('.');
        let ext = if ext.is_empty() { "png" } else { ext };

        let path = self.paths.artwork_path(user_id, app_id, art_type, ext);
        fs::write(&path, data)
            .map_err(|e| SteamError::Io(format!("failed to write artwork: {e}")))?;
        Ok(())
    }

    /// Removes all artwork for an app ID.
    pub fn delete_artwork(&self, user_id: &str, app_id: u32) -> Result<(), SteamError> {
        let existing = self.find_existing_artwork(user_id, app_id)?;
        for path in existing.values() {
            if let Err(e) = fs::remove_file(path)
                && e.kind() != std::io::ErrorKind::NotFound
            {
                return Err(SteamError::Io(format!("failed to remove {path}: {e}")));
            }
        }
        Ok(())
    }
}

/// Generates a Steam shortcut app ID from executable path and name.
///
/// Matches Steam's algorithm: `CRC32(exe + name) | 0x80000000 | 0x02000000`.
pub fn generate_app_id(exe: &str, name: &str) -> u32 {
    let key = format!("{exe}{name}");
    let mut hasher = Hasher::new();
    hasher.update(key.as_bytes());
    let crc = hasher.finalize();
    (crc | 0x80000000) | 0x02000000
}

/// Converts a `ShortcutConfig` to a `ShortcutInfo`.
pub fn convert_to_shortcut_info(cfg: &ShortcutConfig) -> ShortcutInfo {
    ShortcutInfo {
        app_id: generate_app_id(&cfg.exe, &cfg.name),
        name: cfg.name.clone(),
        exe: cfg.exe.clone(),
        start_dir: cfg.start_dir.clone(),
        launch_options: cfg.launch_options.clone(),
        tags: cfg.tags.clone(),
        last_played: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_app_id_deterministic() {
        let id1 = generate_app_id("/usr/bin/game", "My Game");
        let id2 = generate_app_id("/usr/bin/game", "My Game");
        assert_eq!(id1, id2);
    }

    #[test]
    fn generate_app_id_high_bits_set() {
        let id = generate_app_id("/bin/test", "Test");
        // Must have bits 0x80000000 and 0x02000000 set
        assert_ne!(id & 0x80000000, 0);
        assert_ne!(id & 0x02000000, 0);
    }

    #[test]
    fn generate_app_id_different_inputs() {
        let id1 = generate_app_id("/bin/a", "Game A");
        let id2 = generate_app_id("/bin/b", "Game B");
        assert_ne!(id1, id2);
    }

    #[test]
    fn convert_to_shortcut_info_basic() {
        let cfg = ShortcutConfig {
            name: "Test".into(),
            exe: "/bin/test".into(),
            start_dir: "/home".into(),
            launch_options: "--fullscreen".into(),
            tags: vec!["RPG".into()],
            artwork: None,
        };
        let info = convert_to_shortcut_info(&cfg);
        assert_eq!(info.name, "Test");
        assert_eq!(info.exe, "/bin/test");
        assert_eq!(info.app_id, generate_app_id("/bin/test", "Test"));
        assert_eq!(info.tags, vec!["RPG"]);
        assert_eq!(info.last_played, 0);
    }
}
