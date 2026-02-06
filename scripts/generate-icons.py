#!/usr/bin/env python3
"""Generate application icons from logo.jpg with circular mask.

Cross-platform alternative to generate-icons.sh (ImageMagick).
Requires: Pillow (pip install Pillow)

Usage:
    python scripts/generate-icons.py
"""

import sys
from pathlib import Path

try:
    from PIL import Image, ImageDraw
except ImportError:
    print("[ERROR] Pillow is required: pip install Pillow")
    sys.exit(1)

# Colors (ANSI)
GREEN = "\033[0;32m"
YELLOW = "\033[1;33m"
RED = "\033[0;31m"
NC = "\033[0m"

SCRIPT_DIR = Path(__file__).resolve().parent
ROOT_DIR = SCRIPT_DIR.parent
SOURCE = ROOT_DIR / "docs" / "logo.jpg"

# Target directories
TARGETS = [
    ROOT_DIR / "apps" / "hub" / "build",
    ROOT_DIR / "apps" / "agents" / "desktop" / "build",
]

# ICO sizes (standard multi-resolution)
ICO_SIZES = [16, 32, 48, 64, 128, 256]

# Final PNG size
PNG_SIZE = 1024


def create_circular_icon(source: Path, size: int) -> Image.Image:
    """Create a circular icon with transparent background from source image."""
    img = Image.open(source).convert("RGBA")

    # Crop to square from center
    w, h = img.size
    side = min(w, h)
    left = (w - side) // 2
    top = (h - side) // 2
    img = img.crop((left, top, left + side, top + side))

    # Resize to target
    img = img.resize((size, size), Image.LANCZOS)

    # Create circular mask
    mask = Image.new("L", (size, size), 0)
    draw = ImageDraw.Draw(mask)
    draw.ellipse((0, 0, size - 1, size - 1), fill=255)

    # Apply mask
    result = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    result.paste(img, (0, 0), mask)

    return result


def create_ico(base_icon: Image.Image, sizes: list[int]) -> list[Image.Image]:
    """Create list of resized images for ICO file."""
    return [base_icon.resize((s, s), Image.LANCZOS) for s in sizes]


def main():
    print("============================================")
    print("  CapyDeploy Icon Generator (Pillow)")
    print("============================================")
    print()

    # Check source
    if not SOURCE.is_file():
        print(f"{RED}[ERROR]{NC} Source not found: {SOURCE}")
        sys.exit(1)

    print(f"{YELLOW}[1/3]{NC} Creating circular icon ({PNG_SIZE}x{PNG_SIZE})...")
    icon = create_circular_icon(SOURCE, PNG_SIZE)
    print(f"  {GREEN}Done{NC}")

    print(f"{YELLOW}[2/3]{NC} Generating ICO ({', '.join(str(s) for s in ICO_SIZES)})...")
    ico_images = create_ico(icon, ICO_SIZES)
    print(f"  {GREEN}Done{NC}")

    print(f"{YELLOW}[3/3]{NC} Copying to app directories...")

    for target in TARGETS:
        if not target.is_dir():
            print(f"  {RED}[WARN]{NC} Directory not found, creating: {target}")
            target.mkdir(parents=True, exist_ok=True)

        # Save appicon.png
        icon.save(target / "appicon.png", "PNG")

        # Save icon.ico
        windows_dir = target / "windows"
        windows_dir.mkdir(parents=True, exist_ok=True)
        ico_images[0].save(
            windows_dir / "icon.ico",
            format="ICO",
            sizes=[(s, s) for s in ICO_SIZES],
            append_images=ico_images[1:],
        )

        app_name = target.parent.name
        print(f"  {app_name}: appicon.png ({PNG_SIZE}x{PNG_SIZE}), windows/icon.ico")

    print()
    print("============================================")
    print(f"  {GREEN}Icons generated successfully!{NC}")
    print("============================================")
    print()
    print("Files updated:")
    for target in TARGETS:
        app_name = target.parent.name
        print(f"  - apps/{app_name}/build/appicon.png ({PNG_SIZE}x{PNG_SIZE})")
        print(f"  - apps/{app_name}/build/windows/icon.ico")


if __name__ == "__main__":
    main()
