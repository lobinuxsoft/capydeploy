//! Safe game directory and artwork deletion.

use std::path::{Path, PathBuf};

/// Safely deletes a game directory.
///
/// Safety checks:
/// 1. Path must resolve to an existing directory.
/// 2. Path must be within the user's home directory.
/// 3. Path must be at least 2 levels deep from home (e.g., `~/Games/MyGame`).
pub fn delete_game_directory(path: &Path) -> Result<(), String> {
    let home = crate::home_dir();
    delete_game_directory_with_home(path, &home)
}

/// Inner implementation that accepts a home directory for testability.
fn delete_game_directory_with_home(path: &Path, home: &Path) -> Result<(), String> {
    let abs = std::fs::canonicalize(path)
        .map_err(|e| format!("failed to resolve path {}: {e}", path.display()))?;

    // Must be a directory.
    if !abs.is_dir() {
        return Err(format!("not a directory: {}", abs.display()));
    }

    let home_canon = std::fs::canonicalize(home).unwrap_or_else(|_| home.to_path_buf());

    // Must be within home directory.
    if !abs.starts_with(&home_canon) {
        return Err(format!(
            "refusing to delete path outside home directory: {}",
            abs.display()
        ));
    }

    // Must be at least 2 levels deep from home.
    let rel = abs
        .strip_prefix(&home_canon)
        .map_err(|_| "failed to compute relative path".to_string())?;

    let depth = rel.components().count();
    if depth < 2 {
        return Err(format!(
            "refusing to delete top-level directory: {} (depth {depth}, need >= 2)",
            abs.display()
        ));
    }

    std::fs::remove_dir_all(&abs)
        .map_err(|e| format!("failed to delete directory {}: {e}", abs.display()))?;

    tracing::info!(path = %abs.display(), "deleted game directory");
    Ok(())
}

/// Deletes artwork files associated with a Steam appID from the grid directory.
///
/// Removes files matching patterns like `{appId}.*`, `{appId}p.*`,
/// `{appId}_hero.*`, `{appId}_logo.*`, `{appId}_icon.*`.
pub fn delete_artwork(grid_dir: &Path, app_id: u32) -> Result<u32, String> {
    if !grid_dir.exists() {
        return Ok(0);
    }

    let patterns = [
        format!("{app_id}."),
        format!("{app_id}p."),
        format!("{app_id}_hero."),
        format!("{app_id}_logo."),
        format!("{app_id}_icon."),
    ];

    let entries =
        std::fs::read_dir(grid_dir).map_err(|e| format!("failed to read grid directory: {e}"))?;

    let mut deleted = 0u32;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if patterns.iter().any(|p| name_str.starts_with(p.as_str())) {
            if let Err(e) = std::fs::remove_file(entry.path()) {
                tracing::warn!(file = %entry.path().display(), error = %e, "failed to delete artwork");
            } else {
                deleted += 1;
            }
        }
    }

    if deleted > 0 {
        tracing::info!(app_id, deleted, "cleaned up artwork files");
    }

    Ok(deleted)
}

/// Returns the grid directory path for a Steam user.
pub fn grid_dir(steam_base: &Path, user_id: &str) -> PathBuf {
    steam_base
        .join("userdata")
        .join(user_id)
        .join("config")
        .join("grid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delete_game_directory_safety_checks() {
        // Non-existent path.
        let result = delete_game_directory(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn delete_refuses_top_level() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();

        // Create a top-level dir (depth=1, needs >= 2).
        let top_level = home.join("Games");
        std::fs::create_dir_all(&top_level).unwrap();

        let result = delete_game_directory_with_home(&top_level, home);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("top-level"));
    }

    #[test]
    fn delete_allows_deep_path() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();

        let game_dir = home.join("Games").join("TestGame");
        std::fs::create_dir_all(&game_dir).unwrap();

        let result = delete_game_directory_with_home(&game_dir, home);
        assert!(result.is_ok());
        assert!(!game_dir.exists());
    }

    #[test]
    fn delete_refuses_outside_home() {
        let home_tmp = tempfile::tempdir().unwrap();
        let outside_tmp = tempfile::tempdir().unwrap();

        let result = delete_game_directory_with_home(outside_tmp.path(), home_tmp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("outside home"));
    }

    #[test]
    fn delete_refuses_not_a_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();
        let file = home.join("Games").join("not_a_dir");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "data").unwrap();

        let result = delete_game_directory_with_home(&file, home);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a directory"));
    }

    #[test]
    fn delete_artwork_removes_matching_files() {
        let tmp = tempfile::tempdir().unwrap();
        let grid = tmp.path();

        // Create artwork files for app 12345.
        std::fs::write(grid.join("12345.png"), "grid").unwrap();
        std::fs::write(grid.join("12345p.png"), "portrait").unwrap();
        std::fs::write(grid.join("12345_hero.jpg"), "hero").unwrap();
        std::fs::write(grid.join("12345_logo.png"), "logo").unwrap();
        std::fs::write(grid.join("12345_icon.ico"), "icon").unwrap();
        // Files for a different app — should NOT be deleted.
        std::fs::write(grid.join("99999.png"), "other").unwrap();

        let deleted = delete_artwork(grid, 12345).unwrap();
        assert_eq!(deleted, 5);
        assert!(!grid.join("12345.png").exists());
        assert!(!grid.join("12345_hero.jpg").exists());
        // Other app's files should remain.
        assert!(grid.join("99999.png").exists());
    }

    #[test]
    fn delete_artwork_nonexistent_dir() {
        let result = delete_artwork(Path::new("/nonexistent"), 123);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn grid_dir_path() {
        let base = Path::new("/home/user/.steam");
        let dir = grid_dir(base, "12345678");
        assert_eq!(
            dir,
            PathBuf::from("/home/user/.steam/userdata/12345678/config/grid")
        );
    }
}
