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
    for user_home in ["/home/deck", "/home/lobinux"]:
        if os.path.exists(user_home):
            return user_home

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


def read_shortcuts_vdf(user_id: str) -> list:
    """Read shortcuts.vdf for a Steam user (binary VDF format)."""
    steam_dir = get_steam_dir()
    if not steam_dir:
        return []
    vdf_path = os.path.join(steam_dir, "userdata", user_id, "config", "shortcuts.vdf")
    if not os.path.exists(vdf_path):
        return []
    try:
        with open(vdf_path, "rb") as f:
            data = f.read()
        return parse_binary_vdf(data)
    except Exception as e:
        decky.logger.error(f"Failed to read shortcuts.vdf: {e}")
        return []


def parse_binary_vdf(data: bytes) -> list:
    """Parse Valve binary VDF shortcuts format."""
    shortcuts = []
    pos = 0
    length = len(data)

    # Skip header: \x00shortcuts\x00
    header_end = data.find(b'\x00', 1)
    if header_end == -1:
        return []
    pos = header_end + 1

    while pos < length:
        # Each shortcut starts with \x00 + index + \x00
        if data[pos:pos + 1] == b'\x08':  # End of shortcuts
            break
        if data[pos:pos + 1] != b'\x00':
            pos += 1
            continue

        pos += 1  # Skip \x00
        # Skip index string
        idx_end = data.find(b'\x00', pos)
        if idx_end == -1:
            break
        pos = idx_end + 1

        # Read key-value pairs for this shortcut
        shortcut = {}
        while pos < length:
            type_byte = data[pos:pos + 1]
            if type_byte == b'\x08':  # End of shortcut
                pos += 1
                break

            pos += 1
            # Read key name
            key_end = data.find(b'\x00', pos)
            if key_end == -1:
                break
            key = data[pos:key_end].decode('utf-8', errors='ignore').lower()
            pos = key_end + 1

            if type_byte == b'\x01':  # String
                val_end = data.find(b'\x00', pos)
                if val_end == -1:
                    break
                shortcut[key] = data[pos:val_end].decode('utf-8', errors='ignore')
                pos = val_end + 1
            elif type_byte == b'\x02':  # int32
                if pos + 4 > length:
                    break
                shortcut[key] = int.from_bytes(data[pos:pos + 4], 'little', signed=True)
                pos += 4
            elif type_byte == b'\x00':  # Nested (tags, etc) - skip
                depth = 1
                while pos < length and depth > 0:
                    if data[pos:pos + 1] == b'\x00':
                        depth += 1
                    elif data[pos:pos + 1] == b'\x08':
                        depth -= 1
                    pos += 1
            else:
                break

        if shortcut:
            shortcuts.append({
                "appId": shortcut.get("appid", 0) & 0xFFFFFFFF,
                "name": shortcut.get("appname", shortcut.get("name", "")),
                "exe": shortcut.get("exe", ""),
                "startDir": shortcut.get("startdir", ""),
                "launchOptions": shortcut.get("launchoptions", ""),
                "lastPlayed": shortcut.get("lastplaytime", 0),
            })

    return shortcuts
