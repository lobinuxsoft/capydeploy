//! Game installation path resolution and preparation.

use std::path::{Path, PathBuf};

/// Resolves the full installation path for a game.
///
/// Priority: `custom_path` > `base_path`. If neither is provided,
/// falls back to [`default_install_path`](crate::default_install_path).
pub fn resolve_install_path(
    game_name: &str,
    custom_path: Option<&str>,
    base_path: Option<&str>,
) -> PathBuf {
    let base = match (custom_path, base_path) {
        (Some(custom), _) if !custom.is_empty() => expand_home(custom),
        (_, Some(base)) if !base.is_empty() => expand_home(base),
        _ => crate::default_install_path(),
    };

    base.join(game_name)
}

/// Ensures the install directory exists, creating it if necessary.
///
/// Returns the canonicalized path.
pub fn ensure_install_dir(path: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(path)
        .map_err(|e| format!("failed to create install directory {}: {e}", path.display()))?;

    // Set directory permissions on Unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("failed to set directory permissions: {e}"))?;
    }

    std::fs::canonicalize(path)
        .map_err(|e| format!("failed to canonicalize path {}: {e}", path.display()))
}

/// Sets a file as executable (Unix only, 755).
pub fn set_executable(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("file not found: {}", path.display()));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("failed to set executable permissions: {e}"))?;
    }

    Ok(())
}

/// Expands `~` prefix to the user's home directory.
fn expand_home(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        crate::home_dir().join(rest)
    } else if path == "~" {
        crate::home_dir()
    } else {
        PathBuf::from(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_with_custom_path() {
        let path = resolve_install_path("MyGame", Some("/opt/games"), None);
        assert_eq!(path, PathBuf::from("/opt/games/MyGame"));
    }

    #[test]
    fn resolve_with_base_path() {
        let path = resolve_install_path("MyGame", None, Some("/opt/games"));
        assert_eq!(path, PathBuf::from("/opt/games/MyGame"));
    }

    #[test]
    fn resolve_custom_overrides_base() {
        let path = resolve_install_path("MyGame", Some("/custom"), Some("/base"));
        assert_eq!(path, PathBuf::from("/custom/MyGame"));
    }

    #[test]
    fn resolve_fallback_to_default() {
        // When no custom or base path, should use default (HOME/Games).
        let path = resolve_install_path("MyGame", None, None);
        assert!(
            path.to_string_lossy().ends_with("Games/MyGame"),
            "expected path ending with Games/MyGame, got {path:?}"
        );
    }

    #[test]
    fn expand_home_tilde() {
        // Absolute paths pass through unchanged.
        assert_eq!(
            expand_home("/absolute/path"),
            PathBuf::from("/absolute/path")
        );
        // Tilde paths expand to something containing the rest.
        let expanded = expand_home("~/Games");
        assert!(
            expanded.to_string_lossy().ends_with("/Games"),
            "expected path ending with /Games, got {expanded:?}"
        );
        let home_expanded = expand_home("~");
        assert!(!home_expanded.to_string_lossy().contains('~'));
    }

    #[test]
    fn ensure_install_dir_creates_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let game_dir = tmp.path().join("games").join("TestGame");
        let result = ensure_install_dir(&game_dir);
        assert!(result.is_ok());
        assert!(game_dir.exists());
    }

    #[test]
    fn set_executable_nonexistent() {
        let result = set_executable(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn set_executable_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::NamedTempFile::new().unwrap();
        set_executable(tmp.path()).unwrap();
        let mode = std::fs::metadata(tmp.path()).unwrap().permissions().mode();
        assert_eq!(mode & 0o755, 0o755);
    }

    #[test]
    fn resolve_empty_strings_fallback() {
        let path = resolve_install_path("MyGame", Some(""), Some(""));
        // Should fall back to default.
        assert!(path.to_string_lossy().ends_with("Games/MyGame"));
    }
}
