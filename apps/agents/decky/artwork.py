"""
Artwork download and shortcut icon management.
Includes retry logic for the VDF race condition when Steam hasn't flushed yet.
"""

import asyncio
import base64
import os
import ssl
import urllib.request
import decky  # type: ignore

from steam_utils import get_steam_dir, get_steam_users

MAX_ICON_RETRIES = 5
ICON_RETRY_BASE_DELAY = 1.0  # seconds

# Artwork suffix map — matches Steam grid naming convention
_ARTWORK_SUFFIX = {
    "grid": "p",
    "banner": "",
    "hero": "_hero",
    "logo": "_logo",
    "icon": "_icon",
}

# Content-type to file extension
_EXT_MAP = {
    "image/png": "png",
    "image/jpeg": "jpg",
    "image/webp": "webp",
}


def _make_ssl_context() -> ssl.SSLContext:
    """Create an SSL context with best-effort certificate loading.

    Decky Loader runs as root with its own Python — the system cert store
    may not be configured.  We try to load certs, and if the store is empty,
    we disable verification so CDN downloads still work.
    """
    ctx = ssl.create_default_context()
    try:
        ctx.load_default_certs()
    except Exception:
        pass
    if not ctx.get_ca_certs():
        ctx.check_hostname = False
        ctx.verify_mode = ssl.CERT_NONE
    return ctx


async def _update_vdf_icon(vdf_path: str, app_id: int, icon_path: str) -> bool:
    """Update shortcuts.vdf to set the icon path for a shortcut.

    Retries with exponential backoff because Steam may not have flushed
    the new shortcut to shortcuts.vdf yet after AddShortcut().
    Returns True if the VDF was successfully updated.
    """
    try:
        from vdf import binary_load, binary_dump
    except ImportError:
        decky.logger.error("vdf package not found, cannot set shortcut icon")
        return False

    for attempt in range(MAX_ICON_RETRIES):
        if not os.path.exists(vdf_path):
            delay = ICON_RETRY_BASE_DELAY * (2 ** attempt)
            decky.logger.info(
                f"shortcuts.vdf not found yet, retry {attempt + 1}/{MAX_ICON_RETRIES} in {delay}s"
            )
            await asyncio.sleep(delay)
            continue

        try:
            with open(vdf_path, "rb") as f:
                data = binary_load(f)

            found = False
            for shortcut in data.get("shortcuts", {}).values():
                vdf_appid = (shortcut.get("appid", 0) & 0xFFFFFFFF) | 0x80000000
                if vdf_appid == app_id:
                    shortcut["icon"] = icon_path
                    found = True
                    break

            if found:
                with open(vdf_path, "wb") as f:
                    binary_dump(data, f)
                decky.logger.info(f"Updated shortcuts.vdf icon for appId={app_id}")
                return True

            # VDF exists but shortcut not in it yet — Steam hasn't flushed
            delay = ICON_RETRY_BASE_DELAY * (2 ** attempt)
            decky.logger.info(
                f"Shortcut appId={app_id} not in VDF yet, retry {attempt + 1}/{MAX_ICON_RETRIES} in {delay}s"
            )
            await asyncio.sleep(delay)

        except Exception as e:
            decky.logger.error(f"Failed to update shortcuts.vdf (attempt {attempt + 1}): {e}")
            delay = ICON_RETRY_BASE_DELAY * (2 ** attempt)
            await asyncio.sleep(delay)

    decky.logger.warning(
        f"Could not update shortcuts.vdf for appId={app_id} after {MAX_ICON_RETRIES} retries. "
        f"Icon file saved at {icon_path}, will apply on Steam restart."
    )
    return False


def apply_from_data(app_id: int, artwork_type: str, data: bytes, content_type: str) -> None:
    """Write raw artwork bytes to the Steam grid directory.

    Mirrors the Go agent's artwork.ApplyFromData — same naming convention,
    same binary WS message origin.

    Raises ValueError on invalid artwork_type or content_type.
    Raises RuntimeError if Steam directory or users are not found.
    """
    ext = _EXT_MAP.get(content_type)
    if not ext:
        raise ValueError(f"unsupported content type: {content_type}")

    suffix = _ARTWORK_SUFFIX.get(artwork_type)
    if suffix is None:
        raise ValueError(f"unknown artwork type: {artwork_type}")

    steam_dir = get_steam_dir()
    if not steam_dir:
        raise RuntimeError("Steam directory not found")

    users = get_steam_users()
    if not users:
        raise RuntimeError("no Steam users found")

    user_id = users[0]["id"]
    grid_dir = os.path.join(steam_dir, "userdata", user_id, "config", "grid")
    os.makedirs(grid_dir, exist_ok=True)

    # Remove existing artwork with different extensions to avoid stale files
    base = f"{app_id}{suffix}"
    for old_ext in ("png", "jpg", "jpeg", "webp", "ico"):
        old_path = os.path.join(grid_dir, f"{base}.{old_ext}")
        try:
            os.remove(old_path)
        except FileNotFoundError:
            pass

    filename = f"{base}.{ext}"
    dest_path = os.path.join(grid_dir, filename)

    with open(dest_path, "wb") as f:
        f.write(data)

    decky.logger.info(f"Applied {artwork_type} artwork: {dest_path} ({len(data)} bytes)")


async def download_artwork(artwork: dict) -> dict:
    """Download artwork URLs and return base64-encoded data with format info."""
    ssl_ctx = _make_ssl_context()

    result = {}
    for key in ("grid", "hero", "logo", "banner"):
        url = artwork.get(key, "")
        if not url:
            continue
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "CapyDeploy/0.1"})
            with urllib.request.urlopen(req, timeout=15, context=ssl_ctx) as resp:
                data = resp.read()
                content_type = resp.headers.get("Content-Type", "")

            # Detect format from content-type or URL
            fmt = "png"
            if "jpeg" in content_type or "jpg" in content_type or url.endswith(".jpg"):
                fmt = "jpg"
            elif "webp" in content_type or url.endswith(".webp"):
                fmt = "png"  # Steam doesn't support webp, but SetCustomArtwork might handle it

            b64 = base64.b64encode(data).decode("ascii")
            result[key] = {"data": b64, "format": fmt}
            decky.logger.info(f"Downloaded artwork '{key}': {len(data)} bytes ({fmt})")
        except Exception as e:
            decky.logger.error(f"Failed to download artwork '{key}' from {url}: {e}")
    return result


async def set_shortcut_icon(app_id: int, icon_b64: str, icon_format: str) -> bool:
    """Save icon file and update shortcuts.vdf for a shortcut.

    Uses retry with exponential backoff because Steam may not have flushed
    the new shortcut to shortcuts.vdf yet when this is called right after
    AddShortcut().
    """
    steam_dir = get_steam_dir()
    if not steam_dir:
        decky.logger.error("Steam directory not found")
        return False

    users = get_steam_users()
    if not users:
        decky.logger.error("No Steam users found")
        return False
    user_id = users[0]["id"]

    # 1. Save icon file to grid directory (doesn't depend on VDF)
    ext = "jpg" if icon_format == "jpg" else "png"
    grid_dir = os.path.join(steam_dir, "userdata", user_id, "config", "grid")
    os.makedirs(grid_dir, exist_ok=True)
    icon_filename = f"{app_id}_icon.{ext}"
    icon_path = os.path.join(grid_dir, icon_filename)

    try:
        icon_data = base64.b64decode(icon_b64)
        with open(icon_path, "wb") as f:
            f.write(icon_data)
        decky.logger.info(f"Saved icon: {icon_path} ({len(icon_data)} bytes)")
    except Exception as e:
        decky.logger.error(f"Failed to save icon file: {e}")
        return False

    # 2. Update shortcuts.vdf with retry (race condition: Steam hasn't flushed yet)
    vdf_path = os.path.join(steam_dir, "userdata", user_id, "config", "shortcuts.vdf")
    return await _update_vdf_icon(vdf_path, app_id, icon_path)


async def set_shortcut_icon_from_url(app_id: int, icon_url: str) -> bool:
    """Download icon from URL and apply it to a shortcut.

    Downloads directly to the grid directory (no base64 round-trip) and
    updates shortcuts.vdf. Preserves the original file extension from the URL.
    """
    from urllib.parse import urlparse

    steam_dir = get_steam_dir()
    if not steam_dir:
        decky.logger.error("Steam directory not found")
        return False

    users = get_steam_users()
    if not users:
        decky.logger.error("No Steam users found")
        return False
    user_id = users[0]["id"]

    # Determine extension from URL path
    url_path = urlparse(icon_url).path
    ext = os.path.splitext(url_path)[1] or ".png"

    grid_dir = os.path.join(steam_dir, "userdata", user_id, "config", "grid")
    os.makedirs(grid_dir, exist_ok=True)
    icon_filename = f"{app_id}_icon{ext}"
    icon_path = os.path.join(grid_dir, icon_filename)

    # Download directly to file
    ssl_ctx = _make_ssl_context()

    try:
        req = urllib.request.Request(icon_url, headers={"User-Agent": "CapyDeploy/0.1"})
        with urllib.request.urlopen(req, timeout=15, context=ssl_ctx) as resp:
            data = resp.read()
        with open(icon_path, "wb") as f:
            f.write(data)
        decky.logger.info(f"Downloaded icon to {icon_path} ({len(data)} bytes)")
    except Exception as e:
        decky.logger.error(f"Failed to download icon from {icon_url}: {e}")
        return False

    # Update shortcuts.vdf with retry
    vdf_path = os.path.join(steam_dir, "userdata", user_id, "config", "shortcuts.vdf")
    return await _update_vdf_icon(vdf_path, app_id, icon_path)
