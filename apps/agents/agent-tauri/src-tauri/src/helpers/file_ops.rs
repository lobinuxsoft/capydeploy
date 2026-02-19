/// Safely deletes a game directory.
///
/// Safety checks (matching Go agent):
/// - Must be an absolute path
/// - Must be within the user's home directory
/// - Must be at least 2 levels deep from home (e.g. ~/Games/MyGame, not ~/Games)
pub(crate) fn delete_game_directory(path: &str) -> Result<(), String> {
    let abs_path = std::path::Path::new(path);
    if !abs_path.is_absolute() {
        return Err(format!("refusing to delete relative path: {path}"));
    }

    let home = std::env::var("HOME").map_err(|_| "cannot determine home directory".to_string())?;
    let home_path = std::path::Path::new(&home);

    if !abs_path.starts_with(home_path) {
        return Err(format!(
            "refusing to delete path outside home directory: {}",
            abs_path.display()
        ));
    }

    let rel_path = abs_path
        .strip_prefix(home_path)
        .map_err(|_| "cannot determine relative path".to_string())?;

    // Must be at least 2 levels deep (e.g., Games/MyGame, not just Games).
    let depth = rel_path.components().count();
    if depth < 2 {
        return Err(format!(
            "refusing to delete top-level directory: {}",
            abs_path.display()
        ));
    }

    match std::fs::metadata(abs_path) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => {
            return Err(format!("path is not a directory: {}", abs_path.display()));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(format!("cannot stat path: {e}"));
        }
    }

    std::fs::remove_dir_all(abs_path).map_err(|e| format!("failed to remove directory: {e}"))
}
