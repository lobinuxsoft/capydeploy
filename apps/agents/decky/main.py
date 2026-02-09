"""
CapyDeploy Decky Plugin - Backend
Thin entry point: all logic lives in dedicated modules.
"""

import json
import os
import sys
import time
from typing import Optional

# Decky's sandbox doesn't add the plugin dir to sys.path — needed for local modules
PLUGIN_DIR = os.path.dirname(os.path.abspath(__file__))
if PLUGIN_DIR not in sys.path:
    sys.path.insert(0, PLUGIN_DIR)
# Also add py_modules for third-party packages (websockets, zeroconf, vdf)
PY_MODULES_DIR = os.path.join(PLUGIN_DIR, "py_modules")
if os.path.isdir(PY_MODULES_DIR) and PY_MODULES_DIR not in sys.path:
    sys.path.insert(0, PY_MODULES_DIR)

import decky  # type: ignore
from settings import SettingsManager  # type: ignore

from steam_utils import (
    get_local_ip,
    detect_platform,
    get_user_home,
    expand_path,
    get_steam_dir,
    get_steam_users,
)
from mdns_service import MDNSService
from pairing import PairingManager
from upload import UploadSession
from artwork import download_artwork, set_shortcut_icon, set_shortcut_icon_from_url
from ws_server import WebSocketServer

def _read_version() -> str:
    """Read version from package.json at plugin directory."""
    try:
        pkg_path = os.path.join(PLUGIN_DIR, "package.json")
        with open(pkg_path, "r") as f:
            return json.load(f).get("version", "0.0.0")
    except Exception:
        return "0.0.0"


PLUGIN_VERSION = _read_version()


class Plugin:
    settings: SettingsManager
    pairing: PairingManager
    ws_server: WebSocketServer
    mdns_service: Optional[MDNSService]
    agent_id: str
    agent_name: str
    accept_connections: bool
    install_path: str
    _frontend_ws = None

    # Events that MUST NOT be lost — use append queue instead of overwrite
    QUEUED_EVENTS = {
        "operation_event", "create_shortcut", "remove_shortcut",
        "update_artwork", "pairing_code", "pairing_success",
        "hub_connected", "hub_disconnected", "server_error",
    }

    async def _main(self):
        """Called when the plugin is loaded."""
        self.settings = SettingsManager(
            name="capydeploy",
            settings_directory=decky.DECKY_PLUGIN_SETTINGS_DIR
        )
        self.version = PLUGIN_VERSION
        self.pairing = PairingManager(self.settings)
        self.ws_server = WebSocketServer(self)
        self.mdns_service = None

        # Clean stale event queues from previous sessions
        for key in list(self.settings.settings.keys()):
            if key.startswith("_queue_") or key.startswith("_event_"):
                del self.settings.settings[key]
        self.settings.commit()

        # Load settings
        self.agent_name = self.settings.getSetting("agent_name", "Steam Deck")
        self.accept_connections = self.settings.getSetting("accept_connections", True)
        self.install_path = self.settings.getSetting("install_path", "~/Games")

        # Get or generate agent ID
        stored_id = self.settings.getSetting("agent_id", None)
        if stored_id:
            self.agent_id = stored_id
        else:
            import hashlib
            data = f"{self.agent_name}-linux-{time.time()}"
            self.agent_id = hashlib.sha256(data.encode()).hexdigest()[:8]
            self.settings.setSetting("agent_id", self.agent_id)

        # Ensure install path exists
        os.makedirs(expand_path(self.install_path), exist_ok=True)

        # Start server if enabled
        if self.settings.getSetting("enabled", False):
            success = await self.ws_server.start()
            if success:
                # Use the actual port assigned by the OS
                self.mdns_service = MDNSService(
                    self.agent_id, self.agent_name, self.ws_server.actual_port, PLUGIN_VERSION
                )
                self.mdns_service.start()
            else:
                decky.logger.error("Server failed to start - deps missing?")

        decky.logger.info("CapyDeploy plugin loaded")

    async def _unload(self):
        """Called when the plugin is unloaded."""
        if self.mdns_service:
            self.mdns_service.stop()
            self.mdns_service = None
        await self.ws_server.stop()
        decky.logger.info("CapyDeploy plugin unloaded")

    # Maximum events to keep in queue to prevent memory/disk bloat
    MAX_QUEUE_SIZE = 50

    async def notify_frontend(self, event: str, data: dict):
        """Send event to frontend via queue (critical) or overwrite (progress)."""
        decky.logger.info(f"Frontend event: {event} - {data}")
        if event in self.QUEUED_EVENTS:
            queue = self.settings.getSetting(f"_queue_{event}", []) or []
            queue.append({"timestamp": time.time(), "data": data})
            # Limit queue size to prevent unbounded growth
            if len(queue) > self.MAX_QUEUE_SIZE:
                queue = queue[-self.MAX_QUEUE_SIZE:]
            self.settings.setSetting(f"_queue_{event}", queue)
        else:
            self.settings.setSetting(f"_event_{event}", {
                "timestamp": time.time(),
                "data": data,
            })

    # ── Frontend API methods ─────────────────────────────────────────────────

    async def get_setting(self, key: str, default):
        """Get a setting value."""
        return self.settings.getSetting(key, default)

    async def set_setting(self, key: str, value):
        """Set a setting value."""
        self.settings.setSetting(key, value)

    async def set_enabled(self, enabled=False):
        """Enable or disable the server."""
        decky.logger.info(f"set_enabled called with: {enabled}")
        self.settings.setSetting("enabled", enabled)
        if enabled:
            success = await self.ws_server.start()
            if success:
                if not self.mdns_service:
                    # Use the actual port assigned by the OS
                    self.mdns_service = MDNSService(
                        self.agent_id, self.agent_name, self.ws_server.actual_port, PLUGIN_VERSION
                    )
                    self.mdns_service.start()
            else:
                await self.notify_frontend("server_error", {
                    "message": "Failed to start server. Missing dependencies?",
                })
        else:
            if self.mdns_service:
                self.mdns_service.stop()
                self.mdns_service = None
            await self.ws_server.stop()

    async def get_status(self):
        """Get current connection status."""
        decky.logger.info("get_status called")
        return {
            "enabled": self.settings.getSetting("enabled", False),
            "connected": self.ws_server.connected_hub is not None,
            "hubName": self.ws_server.connected_hub.get("name") if self.ws_server.connected_hub else None,
            "agentName": self.agent_name,
            "installPath": self.install_path,
            "platform": detect_platform(),
            "version": PLUGIN_VERSION,
            "port": self.ws_server.actual_port,
            "ip": get_local_ip(),
        }

    async def get_event(self, event_name: str) -> Optional[dict]:
        """Get and clear a frontend event. Pops from queue for critical events."""
        queue_key = f"_queue_{event_name}"
        queue = self.settings.getSetting(queue_key, [])
        if queue:
            event = queue.pop(0)
            self.settings.setSetting(queue_key, queue)
            return event
        key = f"_event_{event_name}"
        event = self.settings.getSetting(key, None)
        if event:
            self.settings.setSetting(key, None)
        return event

    async def set_agent_name(self, name: str):
        """Set the agent name."""
        self.agent_name = name
        self.settings.setSetting("agent_name", name)

    async def set_install_path(self, path: str):
        """Set the install path."""
        self.install_path = path
        self.settings.setSetting("install_path", path)
        os.makedirs(expand_path(path), exist_ok=True)

    async def log_info(self, message: str):
        """Log an info message."""
        decky.logger.info(f"[CapyDeploy] {message}")

    async def log_error(self, message: str):
        """Log an error message."""
        decky.logger.error(f"[CapyDeploy] {message}")

    async def register_shortcut(self, game_name: str, app_id: int):
        """Register a shortcut's appId after frontend creates it via SteamClient."""
        tracked = self.settings.getSetting("tracked_shortcuts", [])
        for sc in tracked:
            if sc.get("appId") == 0 and (sc.get("gameName") == game_name or sc.get("name") == game_name):
                sc["appId"] = app_id
                decky.logger.info(f"Registered shortcut: {game_name} -> appId={app_id}")
                break
        self.settings.setSetting("tracked_shortcuts", tracked)

    async def set_shortcut_icon(self, app_id: int, icon_b64: str, icon_format: str) -> bool:
        """Save icon file and update shortcuts.vdf for a shortcut."""
        return await set_shortcut_icon(app_id, icon_b64, icon_format)

    async def set_shortcut_icon_from_url(self, app_id: int, icon_url: str) -> bool:
        """Download icon from URL and apply it to a shortcut."""
        return await set_shortcut_icon_from_url(app_id, icon_url)

    async def get_authorized_hubs(self):
        """Get list of authorized hubs."""
        authorized = self.settings.getSetting("authorized_hubs", {})
        hubs = []
        for hub_id, hub_data in authorized.items():
            hubs.append({
                "id": hub_id,
                "name": hub_data.get("name", "Unknown"),
                "platform": hub_data.get("platform", ""),
                "pairedAt": hub_data.get("paired_at", 0),
            })
        return hubs

    async def revoke_hub(self, hub_id: str):
        """Revoke authorization for a hub."""
        authorized = self.settings.getSetting("authorized_hubs", {})
        if hub_id in authorized:
            del authorized[hub_id]
            self.settings.setSetting("authorized_hubs", authorized)
            if self.ws_server.connected_hub and self.ws_server.connected_hub.get("id") == hub_id:
                await self.ws_server.close_connection()
            decky.logger.info(f"Revoked hub: {hub_id}")
            return True
        return False

    async def get_installed_games(self):
        """Get list of games installed in the install path."""
        games = []
        expanded_path = expand_path(self.install_path)
        try:
            if os.path.exists(expanded_path):
                for name in os.listdir(expanded_path):
                    game_path = os.path.join(expanded_path, name)
                    if os.path.isdir(game_path):
                        total_size = 0
                        for dirpath, dirnames, filenames in os.walk(game_path):
                            for f in filenames:
                                fp = os.path.join(dirpath, f)
                                try:
                                    total_size += os.path.getsize(fp)
                                except OSError:
                                    pass
                        games.append({
                            "name": name,
                            "path": game_path,
                            "size": total_size,
                        })
        except Exception as e:
            decky.logger.error(f"Error listing games: {e}")
        return games

    async def uninstall_game(self, game_name: str):
        """Remove a game folder and return its appId for shortcut removal."""
        import shutil
        expanded_path = expand_path(self.install_path)
        game_path = os.path.join(expanded_path, game_name)
        try:
            if os.path.exists(game_path) and os.path.isdir(game_path):
                shutil.rmtree(game_path)
                decky.logger.info(f"Uninstalled game: {game_name}")

                tracked = self.settings.getSetting("tracked_shortcuts", [])
                app_id = 0
                for sc in tracked:
                    if sc.get("gameName") == game_name or sc.get("name") == game_name:
                        app_id = sc.get("appId", 0)
                        break
                tracked = [sc for sc in tracked if not (sc.get("gameName") == game_name or sc.get("name") == game_name)]
                self.settings.setSetting("tracked_shortcuts", tracked)

                return app_id if app_id else True
        except Exception as e:
            decky.logger.error(f"Error uninstalling game: {e}")
        return False
