//! Filesystem browser for install path selection.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A directory entry for filesystem browsing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirEntry {
    /// Entry name (not full path).
    pub name: String,
    /// Full absolute path.
    pub path: String,
    /// Whether this entry is a directory.
    pub is_dir: bool,
}

/// Lists the contents of a directory for filesystem browsing.
///
/// Only returns directories (not files), sorted alphabetically.
/// Hidden directories (starting with `.`) are excluded.
pub fn list_directory(path: &Path) -> Result<Vec<DirEntry>, String> {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::fs::canonicalize(path)
            .map_err(|e| format!("failed to resolve path {}: {e}", path.display()))?
    };

    if !abs.is_dir() {
        return Err(format!("not a directory: {}", abs.display()));
    }

    let entries = std::fs::read_dir(&abs)
        .map_err(|e| format!("failed to read directory {}: {e}", abs.display()))?;

    let mut result: Vec<DirEntry> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            if !metadata.is_dir() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip hidden directories.
            if name.starts_with('.') {
                return None;
            }
            let full_path = abs.join(&name);
            Some(DirEntry {
                name,
                path: full_path.to_string_lossy().to_string(),
                is_dir: true,
            })
        })
        .collect();

    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(result)
}

/// Lists common root paths for the platform.
pub fn platform_roots() -> Vec<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        let mut roots = vec![
            PathBuf::from("/"),
            PathBuf::from("/home"),
            PathBuf::from("/mnt"),
            PathBuf::from("/media"),
        ];
        if let Ok(home) = std::env::var("HOME") {
            roots.insert(0, PathBuf::from(home));
        }
        roots
    }

    #[cfg(target_os = "windows")]
    {
        // List available drive letters.
        let mut roots = Vec::new();
        for letter in b'A'..=b'Z' {
            let drive = format!("{}:\\", letter as char);
            let path = PathBuf::from(&drive);
            if path.exists() {
                roots.push(path);
            }
        }
        roots
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        vec![PathBuf::from("/")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_directory_returns_dirs_only() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();

        std::fs::create_dir(base.join("alpha")).unwrap();
        std::fs::create_dir(base.join("beta")).unwrap();
        std::fs::write(base.join("file.txt"), "data").unwrap();
        std::fs::create_dir(base.join(".hidden")).unwrap();

        let entries = list_directory(base).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "alpha");
        assert_eq!(entries[1].name, "beta");
        assert!(entries[0].is_dir);
    }

    #[test]
    fn list_directory_sorted_case_insensitive() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();

        std::fs::create_dir(base.join("Zebra")).unwrap();
        std::fs::create_dir(base.join("alpha")).unwrap();
        std::fs::create_dir(base.join("Beta")).unwrap();

        let entries = list_directory(base).unwrap();

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].name, "alpha");
        assert_eq!(entries[1].name, "Beta");
        assert_eq!(entries[2].name, "Zebra");
    }

    #[test]
    fn list_directory_nonexistent() {
        let result = list_directory(Path::new("/definitely/not/real"));
        assert!(result.is_err());
    }

    #[test]
    fn list_directory_file_not_dir() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let result = list_directory(tmp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a directory"));
    }

    #[test]
    fn list_directory_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let entries = list_directory(tmp.path()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn dir_entry_serialization() {
        let entry = DirEntry {
            name: "Games".into(),
            path: "/home/user/Games".into(),
            is_dir: true,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"name\":\"Games\""));
        assert!(json.contains("\"isDir\":true"));
    }

    #[test]
    fn platform_roots_not_empty() {
        let roots = platform_roots();
        assert!(!roots.is_empty());
    }
}
