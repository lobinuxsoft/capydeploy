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
        self._send_queue: Optional[asyncio.Queue] = None
        self._write_task: Optional[asyncio.Task] = None

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

    async def _write_pump(self, websocket):
        """Dedicated task for writing messages to WebSocket (like Go's writePump)."""
        try:
            while True:
                msg_data = await self._send_queue.get()
                if msg_data is None:  # Shutdown signal
                    break
                try:
                    await websocket.send(msg_data)
                    decky.logger.info(f"WS SENT: {msg_data[:100]}...")
                except Exception as e:
                    decky.logger.error(f"Write error: {e}")
                    break
        except asyncio.CancelledError:
            pass
        except Exception as e:
            decky.logger.error(f"Write pump error: {e}")

    async def handle_connection(self, websocket):
        """Handle a new WebSocket connection."""
        decky.logger.info(f"New connection from {websocket.remote_address}")
        authorized = False
        hub_id = None

        # Create send queue and start write pump (like Go's architecture)
        self._send_queue = asyncio.Queue()
        self._write_task = asyncio.create_task(self._write_pump(websocket))

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

                    decky.logger.info(f"WS RECV [{msg_type}] id={msg_id}")

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
                    elif msg_type == "cancel_upload":
                        await self.handle_cancel_upload(websocket, msg_id, payload)
                    elif msg_type == "get_steam_users":
                        await self.handle_get_steam_users(websocket, msg_id)
                    elif msg_type == "list_shortcuts":
                        await self.handle_list_shortcuts(websocket, msg_id, payload)
                    elif msg_type == "delete_game":
                        await self.handle_delete_game(websocket, msg_id, payload)
                    elif msg_type == "restart_steam":
                        await self.handle_restart_steam(websocket, msg_id)
                    else:
                        decky.logger.warning(f"Unknown message type: {msg_type}")

                except json.JSONDecodeError:
                    decky.logger.error("Failed to parse JSON message")
                except Exception as e:
                    decky.logger.error(f"Error handling message: {e}")

        except Exception as e:
            decky.logger.error(f"Connection error: {e}")
        finally:
            # Stop write pump
            if self._send_queue:
                await self._send_queue.put(None)
            if self._write_task:
                self._write_task.cancel()
                try:
                    await self._write_task
                except asyncio.CancelledError:
                    pass

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

    async def handle_get_steam_users(self, websocket, msg_id: str):
        """Return Steam users from userdata directory."""
        users = self.plugin._get_steam_users()
        proto_users = [{"id": u["id"]} for u in users]
        await self.send(websocket, msg_id, "steam_users_response", {"users": proto_users})

    async def handle_list_shortcuts(self, websocket, msg_id: str, payload: dict):
        """Return shortcuts from tracked data (SteamClient writes VDF lazily)."""
        tracked = self.plugin.settings.getSetting("tracked_shortcuts", [])
        shortcuts = []
        for sc in tracked:
            shortcuts.append({
                "appId": sc.get("appId", 0),
                "name": sc.get("name", ""),
                "exe": sc.get("exe", ""),
                "startDir": sc.get("startDir", ""),
                "launchOptions": "",
                "lastPlayed": 0,
            })
        await self.send(websocket, msg_id, "shortcuts_response", {"shortcuts": shortcuts})

    async def handle_delete_game(self, websocket, msg_id: str, payload: dict):
        """Delete a game completely (like Go agent's handleDeleteGame)."""
        import shutil
        import subprocess

        app_id = payload.get("appId", 0)
        tracked = self.plugin.settings.getSetting("tracked_shortcuts", [])

        # Find game by appId
        game = None
        for sc in tracked:
            if sc.get("appId") == app_id:
                game = sc
                break

        if not game:
            await self.send_error(websocket, msg_id, 404, "game not found")
            return

        game_name = game.get("name", game.get("gameName", ""))

        # Notify frontend: delete start
        await self.plugin.notify_frontend("operation_event", {
            "type": "delete",
            "status": "start",
            "gameName": game_name,
            "progress": 0,
            "message": "Eliminando...",
        })

        # Delete game folder
        start_dir = game.get("startDir", "").strip('"')
        if start_dir and os.path.isdir(start_dir):
            try:
                shutil.rmtree(start_dir)
                decky.logger.info(f"Deleted game folder: {start_dir}")
            except Exception as e:
                decky.logger.error(f"Failed to delete game folder: {e}")

        # Notify frontend to remove Steam shortcut
        await self.plugin.notify_frontend("remove_shortcut", {"appId": app_id})

        # Remove from tracked list
        tracked = [sc for sc in tracked if sc.get("appId") != app_id]
        self.plugin.settings.setSetting("tracked_shortcuts", tracked)

        # Notify progress: restarting Steam
        await self.plugin.notify_frontend("operation_event", {
            "type": "delete",
            "status": "progress",
            "gameName": game_name,
            "progress": 50,
            "message": "Reiniciando Steam...",
        })

        # Restart Steam
        steam_restarted = False
        try:
            subprocess.Popen(["systemctl", "restart", "steam"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            steam_restarted = True
        except Exception as e:
            decky.logger.error(f"Failed to restart Steam: {e}")

        # Notify complete
        await self.plugin.notify_frontend("operation_event", {
            "type": "delete",
            "status": "complete",
            "gameName": game_name,
            "progress": 100,
            "message": "Eliminado",
        })

        await self.send(websocket, msg_id, "operation_result", {
            "status": "deleted",
            "gameName": game_name,
            "steamRestarted": steam_restarted,
        })

    async def handle_restart_steam(self, websocket, msg_id: str):
        """Restart Steam."""
        import subprocess
        try:
            subprocess.Popen(["systemctl", "restart", "steam"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            await self.send(websocket, msg_id, "steam_response", {"success": True, "message": "restarting"})
        except Exception as e:
            await self.send(websocket, msg_id, "steam_response", {"success": False, "message": str(e)})

    async def handle_init_upload(self, websocket, msg_id: str, payload: dict):
        """Initialize an upload session."""
        config = payload.get("config", {})
        game_name = config.get("gameName", "Unknown")
        total_size = payload.get("totalSize", 0)
        files = payload.get("files", [])

        upload_id = f"upload-{int(time.time())}-{random.randint(1000, 9999)}"
        session = UploadSession(upload_id, game_name, total_size, files)
        # Expand path for actual file operations
        expanded_path = self.plugin._expand_path(self.plugin.install_path)
        session.install_path = os.path.join(expanded_path, game_name)
        self.uploads[upload_id] = session

        # Create install directory
        os.makedirs(session.install_path, exist_ok=True)

        decky.logger.info(f"Upload started: {game_name} ({total_size} bytes) -> {session.install_path}")
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
        """Handle binary chunk data. Format: [4 bytes: header len][header JSON][chunk data]"""
        if len(data) < 4:
            decky.logger.error("Binary message too short")
            return

        # Parse header length (big endian)
        header_len = (data[0] << 24) | (data[1] << 16) | (data[2] << 8) | data[3]
        if len(data) < 4 + header_len:
            decky.logger.error("Binary message header incomplete")
            return

        # Parse header JSON
        try:
            header = json.loads(data[4:4 + header_len].decode('utf-8'))
        except Exception as e:
            decky.logger.error(f"Invalid binary header: {e}")
            return

        msg_id = header.get("id", "")
        upload_id = header.get("uploadId", "")
        file_path = header.get("filePath", "")
        offset = header.get("offset", 0)
        checksum = header.get("checksum", "")

        # Extract chunk data
        chunk_data = data[4 + header_len:]

        decky.logger.info(f"Binary chunk: {upload_id}/{file_path} offset={offset} size={len(chunk_data)}")

        # Process the chunk
        session = self.uploads.get(upload_id)
        if not session:
            await self.send_error(websocket, msg_id, 404, "Upload not found")
            return

        # Write chunk to file
        full_path = os.path.join(session.install_path, file_path)
        os.makedirs(os.path.dirname(full_path), exist_ok=True)

        with open(full_path, "ab" if offset > 0 else "wb") as f:
            f.seek(offset)
            f.write(chunk_data)

        session.transferred += len(chunk_data)
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

        # Send response
        await self.send(websocket, msg_id, "upload_chunk_response", {
            "uploadId": upload_id,
            "bytesWritten": len(chunk_data),
            "totalWritten": session.transferred,
        })

    async def handle_cancel_upload(self, websocket, msg_id: str, payload: dict):
        """Cancel an active upload."""
        upload_id = payload.get("uploadId", "")
        session = self.uploads.get(upload_id)

        if session:
            session.status = "cancelled"
            # Clean up partial files
            if session.install_path and os.path.exists(session.install_path):
                import shutil
                try:
                    shutil.rmtree(session.install_path)
                except Exception as e:
                    decky.logger.error(f"Failed to cleanup cancelled upload: {e}")
            del self.uploads[upload_id]
            decky.logger.info(f"Upload cancelled: {session.game_name}")

        await self.send(websocket, msg_id, "operation_result", {"success": True})

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
            # Get just the executable filename (like Go's filepath.Base)
            exe_name = os.path.basename(shortcut_config.get("exe", ""))
            exe_path = os.path.join(session.install_path, exe_name)

            # Make executable on Linux
            if os.path.exists(exe_path):
                os.chmod(exe_path, 0o755)

            # Steam adds quotes to exe automatically, but not to startDir
            quoted_start_dir = f'"{session.install_path}"'

            shortcut_name = shortcut_config.get("name", session.game_name)

            await self.plugin.notify_frontend("create_shortcut", {
                "name": shortcut_name,
                "exe": exe_path,
                "startDir": quoted_start_dir,
                "artwork": shortcut_config.get("artwork", {}),
            })

            # Pre-track the shortcut (appId will be updated by frontend via register_shortcut)
            tracked = self.plugin.settings.getSetting("tracked_shortcuts", [])
            tracked.append({
                "name": shortcut_name,
                "exe": exe_path,
                "startDir": session.install_path,
                "appId": 0,
                "gameName": session.game_name,
                "installedAt": time.time(),
            })
            self.plugin.settings.setSetting("tracked_shortcuts", tracked)

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
        """Send a JSON message via the write queue."""
        msg = {"id": msg_id, "type": msg_type}
        if payload is not None:
            msg["payload"] = payload
        json_str = json.dumps(msg)
        decky.logger.info(f"WS QUEUE [{msg_type}] id={msg_id}")
        if self._send_queue:
            await self._send_queue.put(json_str)
        else:
            decky.logger.error("Send queue not initialized!")

    async def send_error(self, websocket, msg_id: str, code: int, message: str):
        """Send an error message via the write queue."""
        msg = {
            "id": msg_id,
            "type": "error",
            "error": {"code": code, "message": message},
        }
        if self._send_queue:
            await self._send_queue.put(json.dumps(msg))
        else:
            decky.logger.error("Send queue not initialized!")


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

    def _get_steam_dir(self) -> Optional[str]:
        """Find Steam installation directory."""
        home = self._get_user_home()
        candidates = [
            os.path.join(home, ".steam", "steam"),
            os.path.join(home, ".local", "share", "Steam"),
            os.path.join(home, ".var", "app", "com.valvesoftware.Steam", ".steam", "steam"),
        ]
        for path in candidates:
            if os.path.isdir(path):
                return path
        return None

    def _get_steam_users(self) -> list:
        """Get Steam users from userdata directory."""
        steam_dir = self._get_steam_dir()
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
            # Must be numeric and not "0"
            if not entry.isdigit() or entry == "0":
                continue
            has_shortcuts = os.path.exists(
                os.path.join(entry_path, "config", "shortcuts.vdf")
            )
            users.append({"id": entry, "hasShortcuts": has_shortcuts})
        return users

    def _read_shortcuts_vdf(self, user_id: str) -> list:
        """Read shortcuts.vdf for a Steam user (binary VDF format)."""
        steam_dir = self._get_steam_dir()
        if not steam_dir:
            return []
        vdf_path = os.path.join(steam_dir, "userdata", user_id, "config", "shortcuts.vdf")
        if not os.path.exists(vdf_path):
            return []
        try:
            with open(vdf_path, "rb") as f:
                data = f.read()
            return self._parse_binary_vdf(data)
        except Exception as e:
            decky.logger.error(f"Failed to read shortcuts.vdf: {e}")
            return []

    def _parse_binary_vdf(self, data: bytes) -> list:
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

    def _get_user_home(self) -> str:
        """Get the real user home directory (not /root when running as service)."""
        # Try common Steam user homes
        for user_home in ["/home/deck", "/home/lobinux"]:
            if os.path.exists(user_home):
                return user_home

        # Try to find a home with .steam directory
        try:
            for entry in os.listdir("/home"):
                home_path = f"/home/{entry}"
                if os.path.isdir(home_path) and os.path.exists(f"{home_path}/.steam"):
                    return home_path
        except Exception:
            pass

        # Fallback to Path.home() (may be /root in Decky context)
        return str(Path.home())

    def _expand_path(self, path: str) -> str:
        """Expand ~ to actual home directory."""
        if path.startswith("~/"):
            return os.path.join(self._get_user_home(), path[2:])
        return path

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

        # Ensure install path exists (expand ~ for actual file operations)
        os.makedirs(self._expand_path(self.install_path), exist_ok=True)

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

        # Get local IP for display
        import socket
        try:
            s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
            s.connect(("8.8.8.8", 80))
            local_ip = s.getsockname()[0]
            s.close()
        except Exception:
            local_ip = "127.0.0.1"

        # Detect platform
        platform = "linux"
        if os.path.exists("/home/deck"):
            platform = "steamdeck"
        elif os.path.exists("/usr/share/plymouth/themes/legion-go"):
            platform = "legiongologo"
        elif os.path.exists("/usr/share/plymouth/themes/rogally"):
            platform = "rogally"
        else:
            try:
                with open("/etc/os-release", "r") as f:
                    content = f.read().lower()
                    if "steamos" in content:
                        platform = "steamdeck"
                    elif "chimeraos" in content:
                        platform = "chimeraos"
                    elif "bazzite" in content:
                        platform = "bazzite"
            except Exception:
                pass

        return {
            "enabled": self.settings.getSetting("enabled", False),
            "connected": self.ws_server.connected_hub is not None,
            "hubName": self.ws_server.connected_hub.get("name") if self.ws_server.connected_hub else None,
            "agentName": self.agent_name,
            "installPath": self.install_path,
            "platform": platform,
            "version": PLUGIN_VERSION,
            "port": WS_PORT,
            "ip": local_ip,
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
        os.makedirs(self._expand_path(path), exist_ok=True)

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

    async def get_authorized_hubs(self):
        """Get list of authorized hubs."""
        authorized = self.settings.getSetting("authorized_hubs", {})
        hubs = []
        for hub_id, hub_data in authorized.items():
            hubs.append({
                "id": hub_id,
                "name": hub_data.get("name", "Unknown"),
                "pairedAt": hub_data.get("paired_at", 0),
            })
        return hubs

    async def revoke_hub(self, hub_id: str):
        """Revoke authorization for a hub."""
        authorized = self.settings.getSetting("authorized_hubs", {})
        if hub_id in authorized:
            del authorized[hub_id]
            self.settings.setSetting("authorized_hubs", authorized)
            decky.logger.info(f"Revoked hub: {hub_id}")
            return True
        return False

    async def get_installed_games(self):
        """Get list of games installed in the install path."""
        games = []
        expanded_path = self._expand_path(self.install_path)
        try:
            if os.path.exists(expanded_path):
                for name in os.listdir(expanded_path):
                    game_path = os.path.join(expanded_path, name)
                    if os.path.isdir(game_path):
                        # Get folder size
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
        """Remove a game folder from the install path."""
        import shutil
        expanded_path = self._expand_path(self.install_path)
        game_path = os.path.join(expanded_path, game_name)
        try:
            if os.path.exists(game_path) and os.path.isdir(game_path):
                shutil.rmtree(game_path)
                decky.logger.info(f"Uninstalled game: {game_name}")
                return True
        except Exception as e:
            decky.logger.error(f"Error uninstalling game: {e}")
        return False
