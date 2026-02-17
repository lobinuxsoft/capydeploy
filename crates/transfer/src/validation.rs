use std::path::{Component, Path};

use crate::TransferError;

/// Validates that a relative file path does not escape its base directory.
///
/// Rejects:
/// - Empty paths
/// - Absolute paths (Unix `/` or Windows `C:\`)
/// - Parent directory traversal (`..`)
/// - Windows prefix components (`C:`, `\\server`)
pub fn validate_upload_path(file_path: &str) -> Result<(), TransferError> {
    if file_path.is_empty() {
        return Err(TransferError::InvalidPath("empty path".into()));
    }

    let path = Path::new(file_path);

    if path.is_absolute() {
        return Err(TransferError::InvalidPath(format!(
            "absolute path not allowed: {file_path}"
        )));
    }

    for component in path.components() {
        match component {
            Component::ParentDir => {
                return Err(TransferError::InvalidPath(format!(
                    "parent directory traversal not allowed: {file_path}"
                )));
            }
            Component::Prefix(_) => {
                return Err(TransferError::InvalidPath(format!(
                    "path prefix not allowed: {file_path}"
                )));
            }
            Component::RootDir => {
                return Err(TransferError::InvalidPath(format!(
                    "absolute path not allowed: {file_path}"
                )));
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_path() {
        assert!(validate_upload_path("").is_err());
    }

    #[test]
    fn rejects_parent_dir_traversal() {
        assert!(validate_upload_path("../../../etc/passwd").is_err());
    }

    #[test]
    fn rejects_nested_parent_dir_traversal() {
        assert!(validate_upload_path("sub/../../../escape").is_err());
    }

    #[test]
    fn rejects_absolute_unix_path() {
        assert!(validate_upload_path("/tmp/malicious").is_err());
    }

    #[test]
    fn rejects_windows_absolute_path() {
        // On Unix, `C:\Windows\evil` is parsed as a normal relative component,
        // but `C:/Windows/evil` would start with `C:` which is not absolute on Unix.
        // The key protection is that `..` is blocked regardless.
        // On Windows, this would be caught by is_absolute() or Prefix component.
        let result = validate_upload_path("C:\\Windows\\evil");
        // On Unix this is a valid filename (weird but safe), on Windows it's blocked.
        #[cfg(windows)]
        assert!(result.is_err());
        #[cfg(not(windows))]
        assert!(result.is_ok());
    }

    #[test]
    fn accepts_simple_filename() {
        assert!(validate_upload_path("game.exe").is_ok());
    }

    #[test]
    fn accepts_subdirectory_path() {
        assert!(validate_upload_path("sub/dir/file.txt").is_ok());
    }

    #[test]
    fn accepts_dotfile() {
        assert!(validate_upload_path(".config/settings.json").is_ok());
    }

    #[test]
    fn accepts_current_dir_prefix() {
        assert!(validate_upload_path("./game.exe").is_ok());
    }

    #[test]
    fn rejects_single_parent_dir() {
        assert!(validate_upload_path("..").is_err());
    }

    #[test]
    fn rejects_parent_then_file() {
        assert!(validate_upload_path("../file.txt").is_err());
    }
}
