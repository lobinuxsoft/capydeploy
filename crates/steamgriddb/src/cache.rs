//! Optional disk cache for downloaded SteamGridDB images.
//!
//! Images are stored under `~/.config/capydeploy/cache/images/game_<ID>/`
//! with filenames derived from the SHA-256 hash of the source URL.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Errors from cache operations.
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("cache directory not available")]
    NoCacheDir,
}

/// Returns the base image cache directory.
///
/// Creates the directory if it doesn't exist.
pub fn image_cache_dir() -> Result<PathBuf, CacheError> {
    let base = config_dir().ok_or(CacheError::NoCacheDir)?;
    image_cache_dir_in(&base)
}

/// Returns the cache directory for a specific game.
///
/// Creates the directory if it doesn't exist.
pub fn game_cache_dir(game_id: i32) -> Result<PathBuf, CacheError> {
    let base = image_cache_dir()?;
    game_cache_dir_in(&base, game_id)
}

/// Returns the cached image data and content type, if available.
pub fn get_cached_image(game_id: i32, image_url: &str) -> Result<(Vec<u8>, String), CacheError> {
    let base = image_cache_dir()?;
    get_cached_image_in(&base, game_id, image_url)
}

/// Returns the file path of a cached image, if available.
pub fn get_cached_image_path(game_id: i32, image_url: &str) -> Result<PathBuf, CacheError> {
    let base = image_cache_dir()?;
    get_cached_image_path_in(&base, game_id, image_url)
}

/// Saves image data to the cache.
pub fn save_image_to_cache(
    game_id: i32,
    image_url: &str,
    data: &[u8],
    content_type: &str,
) -> Result<(), CacheError> {
    let base = image_cache_dir()?;
    save_image_to_cache_in(&base, game_id, image_url, data, content_type)
}

/// Clears all cached images.
pub fn clear_image_cache() -> Result<(), CacheError> {
    let base = image_cache_dir()?;
    clear_cache_in(&base)
}

/// Returns the total size of the image cache in bytes.
pub fn get_cache_size() -> Result<u64, CacheError> {
    let base = image_cache_dir()?;
    Ok(cache_size_in(&base))
}

// ---------------------------------------------------------------------------
// Internal functions accepting an explicit base directory (testable).
// ---------------------------------------------------------------------------

fn image_cache_dir_in(config_base: &Path) -> Result<PathBuf, CacheError> {
    let cache_dir = config_base.join("capydeploy").join("cache").join("images");
    std::fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

fn game_cache_dir_in(images_dir: &Path, game_id: i32) -> Result<PathBuf, CacheError> {
    let game_dir = images_dir.join(format!("game_{game_id}"));
    std::fs::create_dir_all(&game_dir)?;
    Ok(game_dir)
}

fn get_cached_image_in(
    images_dir: &Path,
    game_id: i32,
    image_url: &str,
) -> Result<(Vec<u8>, String), CacheError> {
    let game_dir = game_cache_dir_in(images_dir, game_id)?;
    let hash = hash_url(image_url);

    let entries = std::fs::read_dir(&game_dir)?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(&hash) {
            let data = std::fs::read(entry.path())?;
            let ext = entry
                .path()
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let content_type = ext_to_content_type(&ext);
            return Ok((data, content_type));
        }
    }

    Err(CacheError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "image not cached",
    )))
}

fn get_cached_image_path_in(
    images_dir: &Path,
    game_id: i32,
    image_url: &str,
) -> Result<PathBuf, CacheError> {
    let game_dir = game_cache_dir_in(images_dir, game_id)?;
    let hash = hash_url(image_url);

    let entries = std::fs::read_dir(&game_dir)?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(&hash) {
            return Ok(entry.path());
        }
    }

    Err(CacheError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "image not cached",
    )))
}

fn save_image_to_cache_in(
    images_dir: &Path,
    game_id: i32,
    image_url: &str,
    data: &[u8],
    content_type: &str,
) -> Result<(), CacheError> {
    let game_dir = game_cache_dir_in(images_dir, game_id)?;
    let hash = hash_url(image_url);
    let ext = content_type_to_ext(content_type);
    let path = game_dir.join(format!("{hash}{ext}"));
    std::fs::write(path, data)?;
    Ok(())
}

fn clear_cache_in(images_dir: &Path) -> Result<(), CacheError> {
    let entries = std::fs::read_dir(images_dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(&path);
        } else {
            let _ = std::fs::remove_file(&path);
        }
    }
    Ok(())
}

fn cache_size_in(images_dir: &Path) -> u64 {
    let mut size = 0u64;
    walk_dir(images_dir, &mut size);
    size
}

/// Recursively sums file sizes.
fn walk_dir(dir: &Path, size: &mut u64) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, size);
        } else if let Ok(meta) = entry.metadata() {
            *size += meta.len();
        }
    }
}

/// Creates a deterministic filename hash from a URL.
///
/// Uses first 16 bytes of SHA-256 (32 hex characters).
pub fn hash_url(url: &str) -> String {
    let hash = Sha256::digest(url.as_bytes());
    hex::encode(&hash[..16])
}

/// Maps a content type to a file extension.
fn content_type_to_ext(content_type: &str) -> &'static str {
    match content_type {
        "image/png" => ".png",
        "image/webp" => ".webp",
        "image/gif" => ".gif",
        _ => ".jpg",
    }
}

/// Maps a file extension to a content type.
fn ext_to_content_type(ext: &str) -> String {
    match ext {
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => "image/jpeg",
    }
    .to_string()
}

/// Returns the platform-specific config directory.
fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".config"))
            })
    }

    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(PathBuf::from)
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join(".config"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a temporary images cache dir for tests (no env var needed).
    fn test_images_dir() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let images = image_cache_dir_in(tmp.path()).unwrap();
        (tmp, images)
    }

    #[test]
    fn hash_url_deterministic() {
        let h1 = hash_url("https://example.com/image.png");
        let h2 = hash_url("https://example.com/image.png");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_url_different_inputs() {
        let h1 = hash_url("https://example.com/image.png");
        let h2 = hash_url("https://example.com/other.png");
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash_url_length() {
        let h = hash_url("https://example.com/image.png");
        assert_eq!(h.len(), 32, "hash should be 32 hex chars (16 bytes)");
    }

    #[test]
    fn content_type_ext_roundtrip() {
        let cases = [
            ("image/png", ".png", "png"),
            ("image/jpeg", ".jpg", "jpg"),
            ("image/webp", ".webp", "webp"),
            ("image/gif", ".gif", "gif"),
            ("image/unknown", ".jpg", "jpg"),
        ];
        for (ct, ext, ext_str) in cases {
            assert_eq!(content_type_to_ext(ct), ext, "content_type_to_ext({ct})");
            let round = ext_to_content_type(ext_str);
            if ct != "image/unknown" {
                assert_eq!(round, ct, "ext_to_content_type({ext_str})");
            }
        }
    }

    #[test]
    fn save_and_get_cached_image() {
        let (_tmp, images) = test_images_dir();

        let game_id = 12345;
        let image_url = "https://example.com/image.png";
        let test_data = b"fake-png-data";

        save_image_to_cache_in(&images, game_id, image_url, test_data, "image/png").unwrap();
        let (data, ct) = get_cached_image_in(&images, game_id, image_url).unwrap();
        assert_eq!(data, test_data);
        assert_eq!(ct, "image/png");
    }

    #[test]
    fn get_cached_image_not_found() {
        let (_tmp, images) = test_images_dir();
        let result = get_cached_image_in(&images, 99999, "https://example.com/nope.png");
        assert!(result.is_err());
    }

    #[test]
    fn save_image_content_types() {
        let (_tmp, images) = test_images_dir();

        let cases = [
            ("image/png", ".png"),
            ("image/jpeg", ".jpg"),
            ("image/webp", ".webp"),
            ("image/gif", ".gif"),
        ];

        for (ct, expected_ext) in cases {
            let url = format!("https://example.com/{ct}");
            save_image_to_cache_in(&images, 1, &url, b"test", ct).unwrap();
            let path = get_cached_image_path_in(&images, 1, &url).unwrap();
            let ext = path.extension().unwrap().to_str().unwrap();
            assert_eq!(
                format!(".{ext}"),
                expected_ext,
                "content type {ct} should produce ext {expected_ext}"
            );
        }
    }

    #[test]
    fn clear_image_cache_works() {
        let (_tmp, images) = test_images_dir();

        save_image_to_cache_in(&images, 1, "https://a.com/a.png", b"aaa", "image/png").unwrap();
        save_image_to_cache_in(&images, 2, "https://b.com/b.jpg", b"bbb", "image/jpeg").unwrap();

        clear_cache_in(&images).unwrap();

        let size = cache_size_in(&images);
        assert_eq!(size, 0);
    }

    #[test]
    fn get_cache_size_counts_bytes() {
        let (_tmp, images) = test_images_dir();

        let size_before = cache_size_in(&images);
        assert_eq!(size_before, 0);

        let data = b"some-image-data-here";
        save_image_to_cache_in(
            &images,
            1,
            "https://example.com/test.png",
            data,
            "image/png",
        )
        .unwrap();

        let size_after = cache_size_in(&images);
        assert_eq!(size_after, data.len() as u64);
    }

    #[test]
    fn game_cache_dir_contains_game_id() {
        let (_tmp, images) = test_images_dir();
        let dir = game_cache_dir_in(&images, 42).unwrap();
        assert!(dir.to_string_lossy().contains("game_42"));
        assert!(dir.exists());
    }
}
