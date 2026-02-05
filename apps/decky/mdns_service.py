"""
mDNS/DNS-SD service advertisement for Hub discovery.
"""

import os
from typing import Optional

import decky  # type: ignore

from steam_utils import get_local_ip, detect_platform

MDNS_SERVICE_TYPE = "_capydeploy._tcp.local."


class MDNSService:
    """Advertises the agent via mDNS/DNS-SD for Hub discovery."""

    def __init__(self, agent_id: str, agent_name: str, port: int, version: str):
        self.agent_id = agent_id
        self.agent_name = agent_name
        self.port = port
        self.version = version
        self.zeroconf = None
        self.service_info = None
        self._thread = None

    def _register_in_thread(self):
        """Register mDNS service in a separate thread to avoid asyncio conflicts."""
        import socket

        try:
            from zeroconf import Zeroconf, ServiceInfo

            hostname = socket.gethostname()
            local_ip = get_local_ip()
            platform = detect_platform()

            properties = {
                b"id": self.agent_id.encode(),
                b"name": self.agent_name.encode(),
                b"platform": platform.encode(),
                b"version": self.version.encode(),
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
            decky.logger.info(
                f"mDNS service registered: {self.agent_id}._capydeploy._tcp.local on {local_ip}:{self.port}"
            )

        except Exception as e:
            import traceback

            decky.logger.error(f"Failed to start mDNS in thread: {e} - {traceback.format_exc()}")

    def start(self):
        """Start advertising via mDNS."""
        import threading

        decky.logger.info(f"mDNS will advertise on {get_local_ip()}:{self.port}")
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
