"""
Pairing code and token management for Hub authentication.
"""

import random
import string
import time
from typing import Optional

from settings import SettingsManager  # type: ignore

PAIRING_CODE_LENGTH = 6
PAIRING_CODE_EXPIRY = 60  # seconds


class PairingManager:
    """Manages pairing codes and tokens for Hub authentication."""

    def __init__(self, settings: SettingsManager):
        self.settings = settings
        self.pending_code: Optional[str] = None
        self.pending_hub_id: Optional[str] = None
        self.pending_hub_name: Optional[str] = None
        self.pending_hub_platform: Optional[str] = None
        self.code_expires_at: float = 0

    def generate_code(self, hub_id: str, hub_name: str, hub_platform: str = "") -> str:
        """Generate a new pairing code."""
        self.pending_code = "".join(random.choices(string.digits, k=PAIRING_CODE_LENGTH))
        self.pending_hub_id = hub_id
        self.pending_hub_name = hub_name
        self.pending_hub_platform = hub_platform
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
            "platform": self.pending_hub_platform or "",
            "token": token,
            "paired_at": time.time(),
        }
        self.settings.setSetting("authorized_hubs", authorized)

        # Clear pending
        self.pending_code = None
        self.pending_hub_id = None
        self.pending_hub_name = None
        self.pending_hub_platform = None

        return token

    def validate_token(self, hub_id: str, token: str) -> bool:
        """Check if a token is valid for a hub."""
        authorized = self.settings.getSetting("authorized_hubs", {})
        hub = authorized.get(hub_id)
        return hub is not None and hub.get("token") == token
