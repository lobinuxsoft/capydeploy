"""
Artwork download and shortcut icon management.
Includes retry logic for the VDF race condition when Steam hasn't flushed yet.
"""

import asyncio
import base64
import os
import ssl
import urllib.request
from typing import Optional

import decky  # type: ignore

from steam_utils import get_steam_dir, get_steam_users

MAX_ICON_RETRIES = 5
ICON_RETRY_BASE_DELAY = 1.0  # seconds


async def download_artwork(artwork: dict) -> dict:
    """Download artwork URLs and return base64-encoded data with format info."""
    # Decky runs as root with its own Python — SSL certs may not be configured
    ssl_ctx = ssl.create_default_context()
    try:
        ssl_ctx.load_default_certs()
    except Exception:
        pass
    # Fallback: if cert store is empty, disable verification for CDN downloads
    if not ssl_ctx.get_ca_certs():
        ssl_ctx.check_hostname = False
        ssl_ctx.verify_mode = ssl.CERT_NONE

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
    try:
        from vdf import binary_load, binary_dump
    except ImportError:
        decky.logger.error("vdf package not found, cannot set shortcut icon")
        return False

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

    # All retries exhausted — icon file is saved in grid/, Steam restart will pick it up
    decky.logger.warning(
        f"Could not update shortcuts.vdf for appId={app_id} after {MAX_ICON_RETRIES} retries. "
        f"Icon file saved at {icon_path}, will apply on Steam restart."
    )
    return False


async def set_shortcut_icon_from_url(app_id: int, icon_url: str) -> bool:
    """Download icon from URL and apply it to a shortcut.

    Downloads directly to the grid directory (no base64 round-trip) and
    updates shortcuts.vdf. Preserves the original file extension from the URL.
    """
    from urllib.parse import urlparse

    try:
        from vdf import binary_load, binary_dump
    except ImportError:
        decky.logger.error("vdf package not found, cannot set shortcut icon")
        return False

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
    ssl_ctx = ssl.create_default_context()
    try:
        ssl_ctx.load_default_certs()
    except Exception:
        pass
    if not ssl_ctx.get_ca_certs():
        ssl_ctx.check_hostname = False
        ssl_ctx.verify_mode = ssl.CERT_NONE

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
                vdf_data = binary_load(f)

            found = False
            for shortcut in vdf_data.get("shortcuts", {}).values():
                vdf_appid = (shortcut.get("appid", 0) & 0xFFFFFFFF) | 0x80000000
                if vdf_appid == app_id:
                    shortcut["icon"] = icon_path
                    found = True
                    break

            if found:
                with open(vdf_path, "wb") as f:
                    binary_dump(vdf_data, f)
                decky.logger.info(f"Updated shortcuts.vdf icon for appId={app_id}")
                return True

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
