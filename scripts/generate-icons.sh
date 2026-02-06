#!/bin/bash
# Generate application icons from logo.jpg with circular mask
# Requires: ImageMagick 7+

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
SOURCE="$ROOT_DIR/docs/logo.jpg"
TEMP_DIR=$(mktemp -d)

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "============================================"
echo "  CapyDeploy Icon Generator"
echo "============================================"
echo

# Check ImageMagick
if ! command -v magick &> /dev/null; then
    echo "Error: ImageMagick 7+ is required"
    echo "Install with: rpm-ostree install ImageMagick"
    exit 1
fi

# Check source file
if [ ! -f "$SOURCE" ]; then
    echo "Error: Source file not found: $SOURCE"
    exit 1
fi

echo -e "${YELLOW}[1/4]${NC} Creating circular mask..."

# Get source dimensions
WIDTH=$(magick identify -format "%w" "$SOURCE")
HEIGHT=$(magick identify -format "%h" "$SOURCE")

# Use smaller dimension for square crop
SIZE=$((WIDTH < HEIGHT ? WIDTH : HEIGHT))

echo "  Source: ${WIDTH}x${HEIGHT}, cropping to ${SIZE}x${SIZE}"

# Create base circular icon (512x512)
# 1. Crop to square from center
# 2. Resize to 512x512
# 3. Apply circular mask with transparent background
magick "$SOURCE" \
    -gravity center -crop "${SIZE}x${SIZE}+0+0" +repage \
    -resize 512x512 \
    \( +clone -alpha extract \
        -draw "fill black polygon 0,0 0,512 512,512 512,0 fill white circle 256,256 256,0" \
        -alpha off \
    \) -compose CopyOpacity -composite \
    -background none \
    "$TEMP_DIR/icon-512.png"

echo -e "  ${GREEN}Base icon created${NC}"

echo -e "${YELLOW}[2/4]${NC} Generating PNG sizes..."

# Generate multiple sizes for ICO
for size in 16 32 48 64 128 256; do
    magick "$TEMP_DIR/icon-512.png" -resize ${size}x${size} "$TEMP_DIR/icon-${size}.png"
    echo "  Generated ${size}x${size}"
done

echo -e "${YELLOW}[3/4]${NC} Creating Windows ICO..."

# Create ICO with multiple sizes
magick "$TEMP_DIR/icon-16.png" "$TEMP_DIR/icon-32.png" "$TEMP_DIR/icon-48.png" "$TEMP_DIR/icon-256.png" "$TEMP_DIR/icon.ico"

echo -e "  ${GREEN}icon.ico created${NC}"

echo -e "${YELLOW}[4/4]${NC} Copying to app directories..."

# Copy to Hub
cp "$TEMP_DIR/icon-512.png" "$ROOT_DIR/apps/hub/build/appicon.png"
cp "$TEMP_DIR/icon.ico" "$ROOT_DIR/apps/hub/build/windows/icon.ico"
echo "  Hub: appicon.png, windows/icon.ico"

# Copy to Agent
cp "$TEMP_DIR/icon-512.png" "$ROOT_DIR/apps/agents/desktop/build/appicon.png"
cp "$TEMP_DIR/icon.ico" "$ROOT_DIR/apps/agents/desktop/build/windows/icon.ico"
echo "  Agent: appicon.png, windows/icon.ico"

# Cleanup
rm -rf "$TEMP_DIR"

echo
echo "============================================"
echo -e "  ${GREEN}Icons generated successfully!${NC}"
echo "============================================"
echo
echo "Files updated:"
echo "  - apps/hub/build/appicon.png (512x512)"
echo "  - apps/hub/build/windows/icon.ico"
echo "  - apps/agents/desktop/build/appicon.png (512x512)"
echo "  - apps/agents/desktop/build/windows/icon.ico"
