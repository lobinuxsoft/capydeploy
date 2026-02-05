"""
Upload session state tracking.
"""

from typing import Optional


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
        self.executable: Optional[str] = None

    def progress(self) -> float:
        if self.total_size == 0:
            return 100.0
        return (self.transferred / self.total_size) * 100
