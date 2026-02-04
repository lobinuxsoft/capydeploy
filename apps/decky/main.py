"""
CapyDeploy Decky Plugin - Backend
Full WebSocket server that receives games from the Hub.
"""

import asyncio
import json
import os
import random
import string
import time
from pathlib import Path
from typing import Optional

import decky  # type: ignore
from settings import SettingsManager  # type: ignore

# Constants
WS_PORT = 9999
CHUNK_SIZE = 1024 * 1024  # 1MB
PAIRING_CODE_LENGTH = 6
PAIRING_CODE_EXPIRY = 60  # seconds
MDNS_SERVICE_TYPE = "_capydeploy._tcp.local."
PLUGIN_VERSION = "0.1.0"


class MDNSService:
    """Advertises the agent via mDNS/DNS-SD for Hub discovery."""

    def __init__(self, agent_id: str, agent_name: str, port: int):
        self.agent_id = agent_id
        self.agent_name = agent_name
        self.port = port
        self.zeroconf = None
        self.service_info = None
        self._thread = None

    def _get_local_ip(self) -> str:
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

    def _detect_platform(self) -> str:
        """Detect the handheld platform."""
        if os.path.exists("/home/deck"):
            return "steamdeck"
        if os.path.exists("/usr/share/plymouth/themes/legion-go"):
            return "legiongologo"
        if os.path.exists("/usr/share/plymouth/themes/rogally"):
            return "rogally"
        try:
            with open("/etc/os-release", "r") as f:
                content = f.read().lower()
                if "steamos" in content:
                    return "steamdeck"
                if "chimeraos" in content:
                    return "chimeraos"
                if "bazzite" in content:
                    return "bazzite"
        except Exception:
            pass
        return "linux"

    def _register_in_thread(self):
        """Register mDNS service in a separate thread to avoid asyncio conflicts."""
        import threading
        import socket
        try:
            from zeroconf import Zeroconf, ServiceInfo

            hostname = socket.gethostname()
            local_ip = self._get_local_ip()
            platform = self._detect_platform()

            properties = {
                b"id": self.agent_id.encode(),
                b"name": self.agent_name.encode(),
                b"platform": platform.encode(),
                b"version": PLUGIN_VERSION.encode(),
            }

            self.service_info = ServiceInfo(
                MDNS_SERVICE_TYPE,
                f"{self.agent_id}.{MDNS_SERVICE_TYPE}",
                addresses=[socket.inet_aton(local_ip)],
                port=self.port,
                properties=properties,
                server=f"{hostname}.local.",
            )

            self.zeroconf = Zeroconf()
            self.zeroconf.register_service(self.service_info)
            decky.logger.info(f"mDNS service registered: {self.agent_id}._capydeploy._tcp.local on {local_ip}:{self.port}")

        except Exception as e:
            import traceback
            decky.logger.error(f"Failed to start mDNS in thread: {e} - {traceback.format_exc()}")

    def start(self):
        """Start advertising via mDNS."""
        import threading
        decky.logger.info(f"mDNS will advertise on {self._get_local_ip()}:{self.port}")
        self._thread = threading.Thread(target=self._register_in_thread, daemon=True)
        self._thread.start()

    def stop(self):
        """Stop advertising."""
        try:
            if self.zeroconf and self.service_info:
                self.zeroconf.unregister_service(self.service_info)
                self.zeroconf.close()
                self.zeroconf = None
                self.service_info = None
                decky.logger.info("mDNS service stopped")
        except Exception as e:
            decky.logger.error(f"Failed to stop mDNS: {e}")


class PairingManager:
    """Manages pairing codes and tokens for Hub authentication."""

    def __init__(self, settings: SettingsManager):
        self.settings = settings
        self.pending_code: Optional[str] = None
        self.pending_hub_id: Optional[str] = None
        self.pending_hub_name: Optional[str] = None
        self.code_expires_at: float = 0

    def generate_code(self, hub_id: str, hub_name: str) -> str:
        """Generate a new pairing code."""
        self.pending_code = "".join(random.choices(string.digits, k=PAIRING_CODE_LENGTH))
        self.pending_hub_id = hub_id
        self.pending_hub_name = hub_name
        self.code_expires_at = time.time() + PAIRING_CODE_EXPIRY
        return self.pending_code

    def validate_code(self, hub_id: str, code: str) -> Optional[str]:
        """Validate a pairing code and return a token if valid."""
        if not self.pending_code or time.time() > self.code_expires_at:
            return None
        if self.pending_hub_id != hub_id or self.pending_code != code:
            return None

        # Generate token
        token = "".join(random.choices(string.ascii_letters + string.digits, k=32))

        # Save authorized hub
        authorized = self.settings.getSetting("authorized_hubs", {})
        authorized[hub_id] = {
            "name": self.pending_hub_name,
            "token": token,
            "paired_at": time.time(),
        }
        self.settings.setSetting("authorized_hubs", authorized)

        # Clear pending
        self.pending_code = None
        self.pending_hub_id = None
        self.pending_hub_name = None

        return token

    def validate_token(self, hub_id: str, token: str) -> bool:
        """Check if a token is valid for a hub."""
        authorized = self.settings.getSetting("authorized_hubs", {})
        hub = authorized.get(hub_id)
        return hub is not None and hub.get("token") == token


class UploadSession:
    """Manages a file upload session."""

    def __init__(self, upload_id: str, game_name: str, total_size: int, files: list):
        self.id = upload_id
        self.game_name = game_name
        self.total_size = total_size
        self.files = files
        self.transferred = 0
        self.current_file: Optional[str] = None
        self.status = "active"
        self.install_path: Optional[str] = None

    def progress(self) -> float:
        if self.total_size == 0:
            return 100.0
        return (self.transferred / self.total_size) * 100


class WebSocketServer:
    """WebSocket server for Hub connections."""

    def __init__(self, plugin: "Plugin"):
        self.plugin = plugin
        self.server = None
        self.connected_hub: Optional[dict] = None
        self.uploads: dict[str, UploadSession] = {}

    async def start(self):
        """Start the WebSocket server."""
        try:
            # Import here to avoid issues if not available
            import websockets

            self.server = await websockets.serve(
                self.handle_connection,
                "0.0.0.0",
                WS_PORT,
                max_size=10 * 1024 * 1024,  # 10MB max message
            )
            decky.logger.info(f"WebSocket server started on port {WS_PORT}")
        except Exception as e:
            decky.logger.error(f"Failed to start WebSocket server: {e}")

    async def stop(self):
        """Stop the WebSocket server."""
        if self.server:
            self.server.close()
            await self.server.wait_closed()
            decky.logger.info("WebSocket server stopped")

    async def handle_connection(self, websocket):
        """Handle a new WebSocket connection."""
        decky.logger.info(f"New connection from {websocket.remote_address}")
        authorized = False
        hub_id = None

        try:
            async for message in websocket:
                try:
                    if isinstance(message, bytes):
                        # Binary message - chunk data
                        await self.handle_binary(websocket, message)
                        continue

                    msg = json.loads(message)
                    msg_type = msg.get("type")
                    msg_id = msg.get("id", "")
                    payload = msg.get("payload", {})

                    if msg_type == "hub_connected":
                        hub_id, authorized = await self.handle_hub_connected(
                            websocket, msg_id, payload
                        )
                    elif msg_type == "pair_confirm":
                        authorized = await self.handle_pair_confirm(
                            websocket, msg_id, payload, hub_id
                        )
                    elif not authorized:
                        await self.send_error(websocket, msg_id, 401, "Not authorized")
                    elif msg_type == "ping":
                        await self.send(websocket, msg_id, "pong", None)
                    elif msg_type == "get_info":
                        await self.handle_get_info(websocket, msg_id)
                    elif msg_type == "get_config":
                        await self.handle_get_config(websocket, msg_id)
                    elif msg_type == "init_upload":
                        await self.handle_init_upload(websocket, msg_id, payload)
                    elif msg_type == "upload_chunk":
                        await self.handle_upload_chunk(websocket, msg_id, payload)
                    elif msg_type == "complete_upload":
                        await self.handle_complete_upload(websocket, msg_id, payload)
                    else:
                        decky.logger.warning(f"Unknown message type: {msg_type}")

                except json.JSONDecodeError:
                    decky.logger.error("Failed to parse JSON message")
                except Exception as e:
                    decky.logger.error(f"Error handling message: {e}")

        except Exception as e:
            decky.logger.error(f"Connection error: {e}")
        finally:
            if self.connected_hub and self.connected_hub.get("id") == hub_id:
                self.connected_hub = None
                await self.plugin.notify_frontend("hub_disconnected", {})
            decky.logger.info(f"Connection closed: {websocket.remote_address}")

    async def handle_hub_connected(self, websocket, msg_id: str, payload: dict):
        """Handle hub_connected handshake."""
        hub_id = payload.get("hubId", "")
        hub_name = payload.get("name", "Unknown Hub")
        hub_version = payload.get("version", "")
        token = payload.get("token", "")

        decky.logger.info(f"Hub connected: {hub_name} v{hub_version}")

        # Check if token is valid
        if token and hub_id and self.plugin.pairing.validate_token(hub_id, token):
            self.connected_hub = {"id": hub_id, "name": hub_name, "version": hub_version}
            await self.send(websocket, msg_id, "agent_status", {
                "name": self.plugin.agent_name,
                "version": "0.1.0",
                "platform": "linux",
                "acceptConnections": self.plugin.accept_connections,
            })
            await self.plugin.notify_frontend("hub_connected", {
                "name": hub_name,
                "version": hub_version,
            })
            return hub_id, True

        # Need pairing
        if not hub_id:
            await self.send_error(websocket, msg_id, 401, "hub_id required")
            return None, False

        code = self.plugin.pairing.generate_code(hub_id, hub_name)
        await self.send(websocket, msg_id, "pairing_required", {
            "code": code,
            "expiresIn": PAIRING_CODE_EXPIRY,
        })
        await self.plugin.notify_frontend("pairing_code", {"code": code})
        return hub_id, False

    async def handle_pair_confirm(self, websocket, msg_id: str, payload: dict, hub_id: str):
        """Handle pairing confirmation."""
        code = payload.get("code", "")
        token = self.plugin.pairing.validate_code(hub_id, code)

        if token:
            self.connected_hub = {"id": hub_id, "name": self.plugin.pairing.pending_hub_name}
            await self.send(websocket, msg_id, "pair_success", {"token": token})
            await self.plugin.notify_frontend("pairing_success", {})
            return True
        else:
            await self.send(websocket, msg_id, "pair_failed", {"reason": "Invalid code"})
            return False

    async def handle_get_info(self, websocket, msg_id: str):
        """Return agent info."""
        await self.send(websocket, msg_id, "info_response", {
            "agent": {
                "id": self.plugin.agent_id,
                "name": self.plugin.agent_name,
                "platform": "linux",
                "version": "0.1.0",
                "acceptConnections": self.plugin.accept_connections,
                "capabilities": ["file_upload", "steam_shortcuts", "steam_artwork"],
            }
        })

    async def handle_get_config(self, websocket, msg_id: str):
        """Return agent config."""
        await self.send(websocket, msg_id, "config_response", {
            "installPath": self.plugin.install_path,
        })

    async def handle_init_upload(self, websocket, msg_id: str, payload: dict):
        """Initialize an upload session."""
        config = payload.get("config", {})
        game_name = config.get("gameName", "Unknown")
        total_size = payload.get("totalSize", 0)
        files = payload.get("files", [])

        upload_id = f"upload-{int(time.time())}-{random.randint(1000, 9999)}"
        session = UploadSession(upload_id, game_name, total_size, files)
        session.install_path = os.path.join(self.plugin.install_path, game_name)
        self.uploads[upload_id] = session

        # Create install directory
        os.makedirs(session.install_path, exist_ok=True)

        decky.logger.info(f"Upload started: {game_name} ({total_size} bytes)")
        await self.plugin.notify_frontend("operation_event", {
            "type": "install",
            "status": "start",
            "gameName": game_name,
            "progress": 0,
        })

        await self.send(websocket, msg_id, "upload_init_response", {
            "uploadId": upload_id,
            "chunkSize": CHUNK_SIZE,
        })

    async def handle_upload_chunk(self, websocket, msg_id: str, payload: dict):
        """Handle a chunk upload."""
        upload_id = payload.get("uploadId", "")
        file_path = payload.get("filePath", "")
        offset = payload.get("offset", 0)
        data = payload.get("data", b"")

        session = self.uploads.get(upload_id)
        if not session:
            await self.send_error(websocket, msg_id, 404, "Upload not found")
            return

        # Write chunk to file
        full_path = os.path.join(session.install_path, file_path)
        os.makedirs(os.path.dirname(full_path), exist_ok=True)

        # Decode base64 if string
        if isinstance(data, str):
            import base64
            data = base64.b64decode(data)

        with open(full_path, "ab" if offset > 0 else "wb") as f:
            f.seek(offset)
            f.write(data)

        session.transferred += len(data)
        session.current_file = file_path

        # Notify progress
        progress = session.progress()
        await self.plugin.notify_frontend("upload_progress", {
            "uploadId": upload_id,
            "transferredBytes": session.transferred,
            "totalBytes": session.total_size,
            "currentFile": file_path,
            "percentage": progress,
        })

        await self.send(websocket, msg_id, "upload_chunk_response", {
            "uploadId": upload_id,
            "bytesWritten": len(data),
            "totalWritten": session.transferred,
        })

    async def handle_binary(self, websocket, data: bytes):
        """Handle binary chunk data."""
        # Binary format: upload_id (36 bytes) + file_path (256 bytes) + offset (8 bytes) + data
        # For now, we use JSON chunks - binary is for optimization later
        pass

    async def handle_complete_upload(self, websocket, msg_id: str, payload: dict):
        """Complete an upload and create shortcut."""
        upload_id = payload.get("uploadId", "")
        create_shortcut = payload.get("createShortcut", False)
        shortcut_config = payload.get("shortcut", {})

        session = self.uploads.get(upload_id)
        if not session:
            await self.send_error(websocket, msg_id, 404, "Upload not found")
            return

        session.status = "complete"
        decky.logger.info(f"Upload complete: {session.game_name}")

        result = {
            "success": True,
            "path": session.install_path,
        }

        # Notify frontend to create shortcut using SteamClient
        if create_shortcut and shortcut_config:
            exe_name = shortcut_config.get("exe", "")
            exe_path = os.path.join(session.install_path, exe_name)

            # Make executable on Linux
            if os.path.exists(exe_path):
                os.chmod(exe_path, 0o755)

            await self.plugin.notify_frontend("create_shortcut", {
                "name": shortcut_config.get("name", session.game_name),
                "exe": exe_path,
                "startDir": session.install_path,
                "artwork": shortcut_config.get("artwork", {}),
            })

        await self.plugin.notify_frontend("operation_event", {
            "type": "install",
            "status": "complete",
            "gameName": session.game_name,
            "progress": 100,
        })

        # Cleanup
        del self.uploads[upload_id]

        await self.send(websocket, msg_id, "operation_result", result)

    async def send(self, websocket, msg_id: str, msg_type: str, payload):
        """Send a JSON message."""
        msg = {"id": msg_id, "type": msg_type}
        if payload is not None:
            msg["payload"] = payload
        await websocket.send(json.dumps(msg))

    async def send_error(self, websocket, msg_id: str, code: int, message: str):
        """Send an error message."""
        await websocket.send(json.dumps({
            "id": msg_id,
            "type": "error",
            "error": {"code": code, "message": message},
        }))


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

    async def _main(self):
        """Called when the plugin is loaded."""
        self.settings = SettingsManager(
            name="capydeploy",
            settings_directory=decky.DECKY_PLUGIN_SETTINGS_DIR
        )
        self.pairing = PairingManager(self.settings)
        self.ws_server = WebSocketServer(self)
        self.mdns_service = None

        # Load settings
        self.agent_name = self.settings.getSetting("agent_name", "Steam Deck")
        self.accept_connections = self.settings.getSetting("accept_connections", True)
        self.install_path = self.settings.getSetting(
            "install_path",
            os.path.join(str(Path.home()), "Games")
        )

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
        os.makedirs(self.install_path, exist_ok=True)

        # Start server if enabled
        if self.settings.getSetting("enabled", False):
            await self.ws_server.start()
            self.mdns_service = MDNSService(self.agent_id, self.agent_name, WS_PORT)
            self.mdns_service.start()

        decky.logger.info("CapyDeploy plugin loaded")

    async def _unload(self):
        """Called when the plugin is unloaded."""
        if self.mdns_service:
            self.mdns_service.stop()
            self.mdns_service = None
        await self.ws_server.stop()
        decky.logger.info("CapyDeploy plugin unloaded")

    async def notify_frontend(self, event: str, data: dict):
        """Send event to frontend."""
        # This will be picked up by the frontend via polling or events
        decky.logger.info(f"Frontend event: {event} - {data}")
        # Store for frontend to retrieve
        self.settings.setSetting(f"_event_{event}", {
            "timestamp": time.time(),
            "data": data,
        })

    # Frontend API methods

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
            await self.ws_server.start()
            self.mdns_service = MDNSService(self.agent_id, self.agent_name, WS_PORT)
            self.mdns_service.start()
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
        }

    async def get_event(self, event_name: str) -> Optional[dict]:
        """Get and clear a frontend event."""
        event = self.settings.getSetting(f"_event_{event_name}", None)
        if event:
            self.settings.setSetting(f"_event_{event_name}", None)
        return event

    async def set_agent_name(self, name: str):
        """Set the agent name."""
        self.agent_name = name
        self.settings.setSetting("agent_name", name)

    async def set_install_path(self, path: str):
        """Set the install path."""
        self.install_path = path
        self.settings.setSetting("install_path", path)
        os.makedirs(path, exist_ok=True)

    async def log_info(self, message: str):
        """Log an info message."""
        decky.logger.info(f"[CapyDeploy] {message}")

    async def log_error(self, message: str):
        """Log an error message."""
        decky.logger.error(f"[CapyDeploy] {message}")
