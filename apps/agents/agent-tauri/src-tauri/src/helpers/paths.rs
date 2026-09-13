/// Expands `~` in paths to the `$HOME` directory.
///
/// Canonicalizes `$HOME` first: on OSTree/atomic distros (Bazzite, Anatase)
/// `/home` is a symlink to `/var/home`, and Steam's sandboxed runtime
/// (pressure-vessel) does not resolve it — a shortcut written as
/// `/home/user/...` fails to launch (`chdir` ENOENT) even though the path
/// is valid outside the sandbox.
pub(crate) fn expand_path(path: &str) -> String {
    if path.starts_with("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        let home = std::fs::canonicalize(&home)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or(home);
        return format!("{}{}", home, &path[1..]);
    }
    path.to_string()
}
