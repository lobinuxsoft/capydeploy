"""
WebSocket server for Hub connections.
"""

from __future__ import annotations

import asyncio
import json
import os
import random
import time
from typing import Optional, TYPE_CHECKING

import decky  # type: ignore

from upload import UploadSession
from artwork import download_artwork
from steam_utils import get_steam_users, expand_path, fix_permissions
from pairing import PAIRING_CODE_EXPIRY

if TYPE_CHECKING:
    from main import Plugin

CHUNK_SIZE = 1024 * 1024  # 1MB


class WebSocketServer:
    """WebSocket server for Hub connections."""

    def __init__(self, plugin: Plugin):
        self.plugin = plugin
        self.server = None
        self.actual_port: int = 0  # Assigned by OS after start()
        self.connected_hub: Optional[dict] = None
        self.uploads: dict[str, UploadSession] = {}
        self._send_queue: Optional[asyncio.Queue] = None
        self._write_task: Optional[asyncio.Task] = None
        self._active_websocket = None

    async def start(self) -> bool:
        """Start the WebSocket server. Returns True on success."""
        if self.server:
            decky.logger.info("WebSocket server already running")
            return True

        try:
            import websockets
        except ImportError:
            decky.logger.error(
                "websockets package not found. "
                "Install it on the device: pip install websockets zeroconf"
            )
            return False

        try:
            # Use port 0 for dynamic port assignment by OS
            self.server = await websockets.serve(
                self.handle_connection,
                "0.0.0.0",
                0,
                max_size=10 * 1024 * 1024,  # 10MB max message
                reuse_address=True,
            )
            # Get the actual port assigned by the OS
            self.actual_port = self.server.sockets[0].getsockname()[1]
            decky.logger.info(f"WebSocket server started on port {self.actual_port}")
            return True
        except Exception as e:
            decky.logger.error(f"Failed to start WebSocket server: {e}")
            return False

    async def close_connection(self):
        """Close the active WebSocket connection."""
        if self._active_websocket:
            try:
                await self._active_websocket.close()
            except Exception:
                pass

    async def stop(self):
        """Stop the WebSocket server."""
        await self.close_connection()
        if self.server:
            self.server.close()
            await self.server.wait_closed()
            self.server = None
            decky.logger.info("WebSocket server stopped")

    async def _write_pump(self, websocket, send_queue: asyncio.Queue):
        """Dedicated task for writing messages to WebSocket (like Go's writePump)."""
        try:
            while True:
                msg_data = await send_queue.get()
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
        self._active_websocket = websocket

        # Local refs so cleanup works even if a new connection overwrites instance vars
        send_queue = asyncio.Queue()
        write_task = asyncio.create_task(self._write_pump(websocket, send_queue))
        self._send_queue = send_queue
        self._write_task = write_task

        try:
            async for message in websocket:
                try:
                    if isinstance(message, bytes):
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
            self._active_websocket = None

            # Stop write pump using LOCAL refs (instance vars may belong to a newer connection)
            await send_queue.put(None)
            write_task.cancel()
            try:
                await write_task
            except asyncio.CancelledError:
                pass

            if self.connected_hub and self.connected_hub.get("id") == hub_id:
                self.connected_hub = None
                try:
                    await self.plugin.notify_frontend("hub_disconnected", {})
                except Exception as e:
                    decky.logger.error(f"Failed to notify hub_disconnected: {e}")
            decky.logger.info(f"Connection closed: {websocket.remote_address}")

    async def handle_hub_connected(self, websocket, msg_id: str, payload: dict):
        """Handle hub_connected handshake."""
        hub_id = payload.get("hubId", "")
        hub_name = payload.get("name", "Unknown Hub")
        hub_version = payload.get("version", "")
        hub_platform = payload.get("platform", "")
        token = payload.get("token", "")

        decky.logger.info(f"Hub connected: {hub_name} v{hub_version} ({hub_platform})")

        # Check if token is valid
        if token and hub_id and self.plugin.pairing.validate_token(hub_id, token):
            self.connected_hub = {"id": hub_id, "name": hub_name, "version": hub_version, "platform": hub_platform}
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

        code = self.plugin.pairing.generate_code(hub_id, hub_name, hub_platform)
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
        users = get_steam_users()
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

        # Notify frontend to remove Steam shortcut via SteamClient.Apps.RemoveShortcut
        await self.plugin.notify_frontend("remove_shortcut", {"appId": app_id})

        # Remove from tracked list
        tracked = [sc for sc in tracked if sc.get("appId") != app_id]
        self.plugin.settings.setSetting("tracked_shortcuts", tracked)

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
            "steamRestarted": False,
        })

    async def handle_restart_steam(self, websocket, msg_id: str):
        """Restart Steam."""
        import subprocess

        try:
            subprocess.Popen(
                ["systemctl", "restart", "steam"],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
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
        expanded = expand_path(self.plugin.install_path)
        session.install_path = os.path.join(expanded, game_name)
        session.executable = config.get("executable", "")
        self.uploads[upload_id] = session

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

        full_path = os.path.join(session.install_path, file_path)
        os.makedirs(os.path.dirname(full_path), exist_ok=True)

        if isinstance(data, str):
            import base64

            data = base64.b64decode(data)

        with open(full_path, "ab" if offset > 0 else "wb") as f:
            f.seek(offset)
            f.write(data)

        session.transferred += len(data)
        session.current_file = file_path

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

        header_len = (data[0] << 24) | (data[1] << 16) | (data[2] << 8) | data[3]
        if len(data) < 4 + header_len:
            decky.logger.error("Binary message header incomplete")
            return

        try:
            header = json.loads(data[4:4 + header_len].decode('utf-8'))
        except Exception as e:
            decky.logger.error(f"Invalid binary header: {e}")
            return

        msg_id = header.get("id", "")
        upload_id = header.get("uploadId", "")
        file_path = header.get("filePath", "")
        offset = header.get("offset", 0)

        chunk_data = data[4 + header_len:]

        decky.logger.info(f"Binary chunk: {upload_id}/{file_path} offset={offset} size={len(chunk_data)}")

        session = self.uploads.get(upload_id)
        if not session:
            await self.send_error(websocket, msg_id, 404, "Upload not found")
            return

        full_path = os.path.join(session.install_path, file_path)
        os.makedirs(os.path.dirname(full_path), exist_ok=True)

        with open(full_path, "ab" if offset > 0 else "wb") as f:
            f.seek(offset)
            f.write(chunk_data)

        session.transferred += len(chunk_data)
        session.current_file = file_path

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
            "bytesWritten": len(chunk_data),
            "totalWritten": session.transferred,
        })

    async def handle_cancel_upload(self, websocket, msg_id: str, payload: dict):
        """Cancel an active upload."""
        upload_id = payload.get("uploadId", "")
        session = self.uploads.get(upload_id)

        if session:
            session.status = "cancelled"
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

        # Fix ownership/permissions — Decky runs as root but Steam runs as the real user
        fix_permissions(session.install_path)

        result = {
            "success": True,
            "path": session.install_path,
        }

        if create_shortcut and shortcut_config:
            # Agent calculates all paths — Hub only provides the executable filename
            exe_name = os.path.basename(session.executable.replace("\\", "/"))
            exe_path = os.path.join(session.install_path, exe_name)

            if os.path.exists(exe_path):
                os.chmod(exe_path, 0o755)

            quoted_start_dir = f'"{session.install_path}"'
            shortcut_name = shortcut_config.get("name", session.game_name)

            artwork_b64 = {}
            raw_artwork = shortcut_config.get("artwork", {})
            if raw_artwork:
                artwork_b64 = await download_artwork(raw_artwork)

            # Pass icon URL directly (backend will download it after shortcut creation)
            icon_url = raw_artwork.get("icon", "")

            await self.plugin.notify_frontend("create_shortcut", {
                "name": shortcut_name,
                "exe": exe_path,
                "startDir": quoted_start_dir,
                "artwork": artwork_b64,
                "iconUrl": icon_url,
            })

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
