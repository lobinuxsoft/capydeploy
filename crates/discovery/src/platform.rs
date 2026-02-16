/// Returns the current platform identifier.
///
/// Platform values match the Go implementation:
/// - `"windows"` — Windows OS
/// - `"steamdeck"` — Steam Deck (real SteamOS)
/// - `"chimeraos"` — ChimeraOS handheld
/// - `"legiongologo"` — Lenovo Legion Go
/// - `"rogally"` — ASUS ROG Ally
/// - `"linux"` — Generic Linux (including Bazzite)
pub fn detect_platform() -> &'static str {
    detect_platform_inner()
}

#[cfg(target_os = "windows")]
fn detect_platform_inner() -> &'static str {
    "windows"
}

#[cfg(target_os = "linux")]
fn detect_platform_inner() -> &'static str {
    detect_linux_platform()
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn detect_platform_inner() -> &'static str {
    "linux"
}

#[cfg(target_os = "linux")]
fn detect_linux_platform() -> &'static str {
    // Check OS release first (most reliable method)
    if let Ok(data) = std::fs::read_to_string("/etc/os-release") {
        let content = data.to_lowercase();

        // SteamOS is the real Steam Deck
        if content.contains("steamos") {
            return "steamdeck";
        }

        // ChimeraOS
        if content.contains("chimeraos") {
            return "chimeraos";
        }

        // Bazzite (Fedora-based gaming distro, NOT a Steam Deck)
        if content.contains("bazzite") {
            return "linux";
        }
    }

    // Check for handheld-specific files (fallback)
    if std::path::Path::new("/usr/share/plymouth/themes/legion-go").exists() {
        return "legiongologo";
    }
    if std::path::Path::new("/usr/share/plymouth/themes/rogally").exists() {
        return "rogally";
    }

    // Only check /home/deck if it's a real directory (not a symlink).
    // This avoids false positives on Bazzite which symlinks /home/deck.
    if let Ok(meta) = std::fs::symlink_metadata("/home/deck")
        && !meta.file_type().is_symlink()
        && meta.is_dir()
    {
        return "steamdeck";
    }

    "linux"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_platform_returns_valid_value() {
        let platform = detect_platform();
        let valid = [
            "windows",
            "steamdeck",
            "chimeraos",
            "legiongologo",
            "rogally",
            "linux",
        ];
        assert!(valid.contains(&platform), "unexpected platform: {platform}");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn detect_platform_windows() {
        assert_eq!(detect_platform(), "windows");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn detect_platform_linux() {
        // On a standard Linux CI, should return "linux" or a recognized distro
        let platform = detect_platform();
        assert!(!platform.is_empty());
    }
}
