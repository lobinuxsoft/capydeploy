"""
Shared Steam/system utility functions for CapyDeploy Decky Plugin.
All functions are module-level (no class required).
"""

import os
from pathlib import Path
from typing import Optional

import decky  # type: ignore


def get_local_ip() -> str:
    """Get the local non-loopback IP address."""
    import socket

    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.connect(("8.8.8.8", 80))
        ip = s.getsockname()[0]
        s.close()
        return ip
    except Exception:
        return "127.0.0.1"


def detect_platform() -> str:
    """Detect the handheld platform."""
    # Check OS release first (most reliable method)
    try:
        with open("/etc/os-release", "r") as f:
            content = f.read().lower()
            # SteamOS is the real Steam Deck
            if "steamos" in content:
                return "steamdeck"
            if "chimeraos" in content:
                return "chimeraos"
            # Bazzite is NOT a Steam Deck, return linux
            if "bazzite" in content:
                return "linux"
    except Exception:
        pass

    # Check for handheld-specific files (fallback)
    if os.path.exists("/usr/share/plymouth/themes/legion-go"):
        return "legiongologo"
    if os.path.exists("/usr/share/plymouth/themes/rogally"):
        return "rogally"

    # Only check /home/deck if it's a real directory (not a symlink)
    # This avoids false positives on Bazzite which symlinks /home/deck
    try:
        info = os.lstat("/home/deck")
        import stat
        if not stat.S_ISLNK(info.st_mode) and stat.S_ISDIR(info.st_mode):
            return "steamdeck"
    except Exception:
        pass

    return "linux"


def get_user_home() -> str:
    """Get the real user home directory (not /root when running as service)."""
    # Check the standard Steam Deck user first
    if os.path.exists("/home/deck"):
        return "/home/deck"

    try:
        for entry in os.listdir("/home"):
            home_path = f"/home/{entry}"
            if os.path.isdir(home_path) and os.path.exists(f"{home_path}/.steam"):
                return home_path
    except Exception:
        pass

    return str(Path.home())


def expand_path(path: str) -> str:
    """Expand ~ to actual home directory."""
    if path.startswith("~/"):
        return os.path.join(get_user_home(), path[2:])
    return path


def fix_permissions(path: str) -> None:
    """Recursively fix ownership and permissions so the real user can access files.

    Decky runs as root but Steam runs as the regular user (e.g. deck).
    Resolves the real owner from the user home directory.
    """
    home = get_user_home()
    try:
        stat = os.stat(home)
        uid, gid = stat.st_uid, stat.st_gid
    except Exception:
        return

    for dirpath, dirnames, filenames in os.walk(path):
        try:
            os.chown(dirpath, uid, gid)
            os.chmod(dirpath, 0o755)
        except Exception:
            pass
        for fname in filenames:
            fpath = os.path.join(dirpath, fname)
            try:
                os.chown(fpath, uid, gid)
                os.chmod(fpath, 0o644)
            except Exception:
                pass


def get_steam_dir() -> Optional[str]:
    """Find Steam installation directory."""
    home = get_user_home()
    candidates = [
        os.path.join(home, ".steam", "steam"),
        os.path.join(home, ".local", "share", "Steam"),
        os.path.join(home, ".var", "app", "com.valvesoftware.Steam", ".steam", "steam"),
    ]
    for path in candidates:
        if os.path.isdir(path):
            return path
    return None


def get_steam_users() -> list:
    """Get Steam users from userdata directory."""
    steam_dir = get_steam_dir()
    if not steam_dir:
        return []
    userdata_dir = os.path.join(steam_dir, "userdata")
    if not os.path.isdir(userdata_dir):
        return []
    users = []
    for entry in os.listdir(userdata_dir):
        entry_path = os.path.join(userdata_dir, entry)
        if not os.path.isdir(entry_path):
            continue
        if not entry.isdigit() or entry == "0":
            continue
        has_shortcuts = os.path.exists(
            os.path.join(entry_path, "config", "shortcuts.vdf")
        )
        users.append({"id": entry, "hasShortcuts": has_shortcuts})
    return users


