//! File scanning for upload.
//!
//! Recursively walks a directory and produces a list of file entries
//! with relative paths normalized to forward slashes.

use std::path::Path;

use capydeploy_protocol::messages::FileEntry;

use crate::error::DeployError;

/// Scans a directory recursively and returns file entries for upload.
///
/// Relative paths use `/` as separator (even on Windows) to match
/// the agent's expectations. Returns the file list and total size in bytes.
pub fn scan_files_for_upload(root_path: &Path) -> Result<(Vec<FileEntry>, i64), DeployError> {
    let mut files = Vec::new();
    let mut total_size: i64 = 0;

    walk_dir(root_path, root_path, &mut files, &mut total_size)?;

    Ok((files, total_size))
}

fn walk_dir(
    root: &Path,
    current: &Path,
    files: &mut Vec<FileEntry>,
    total_size: &mut i64,
) -> Result<(), DeployError> {
    let entries = std::fs::read_dir(current)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;

        if metadata.is_dir() {
            walk_dir(root, &path, files, total_size)?;
        } else if metadata.is_file() {
            let rel_path = path.strip_prefix(root).map_err(std::io::Error::other)?;

            // Normalize to forward slashes.
            let rel_str = rel_path.to_string_lossy().replace('\\', "/");
            let size = metadata.len() as i64;

            files.push(FileEntry {
                relative_path: rel_str,
                size,
            });
            *total_size += size;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_tree() -> TempDir {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        fs::write(root.join("game.exe"), b"EXE_CONTENT").unwrap();
        fs::write(root.join("readme.txt"), b"READ").unwrap();

        fs::create_dir_all(root.join("data").join("levels")).unwrap();
        fs::write(root.join("data").join("config.ini"), b"CFG").unwrap();
        fs::write(
            root.join("data").join("levels").join("level1.dat"),
            b"LEVEL_DATA_HERE",
        )
        .unwrap();

        dir
    }

    #[test]
    fn scan_finds_all_files() {
        let dir = create_test_tree();
        let (files, total_size) = scan_files_for_upload(dir.path()).unwrap();

        assert_eq!(files.len(), 4);

        let paths: Vec<&str> = files.iter().map(|f| f.relative_path.as_str()).collect();
        assert!(paths.contains(&"game.exe"));
        assert!(paths.contains(&"readme.txt"));
        assert!(paths.contains(&"data/config.ini"));
        assert!(paths.contains(&"data/levels/level1.dat"));

        let expected_size =
            b"EXE_CONTENT".len() + b"READ".len() + b"CFG".len() + b"LEVEL_DATA_HERE".len();
        assert_eq!(total_size, expected_size as i64);
    }

    #[test]
    fn scan_empty_dir() {
        let dir = TempDir::new().unwrap();
        let (files, total_size) = scan_files_for_upload(dir.path()).unwrap();
        assert!(files.is_empty());
        assert_eq!(total_size, 0);
    }

    #[test]
    fn scan_nonexistent_dir() {
        let result = scan_files_for_upload(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(result.is_err());
    }

    #[test]
    fn scan_file_sizes_are_correct() {
        let dir = TempDir::new().unwrap();
        let data = vec![0u8; 1234];
        fs::write(dir.path().join("test.bin"), &data).unwrap();

        let (files, total_size) = scan_files_for_upload(dir.path()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].size, 1234);
        assert_eq!(total_size, 1234);
    }
}
