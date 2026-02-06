#!/bin/bash
# Build script for CapyDeploy Decky Plugin

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Project directories
ROOT_DIR="$(cd ../../.. && pwd)"
DIST_DIR="$ROOT_DIR/dist"

PLUGIN_NAME="CapyDeploy"
VERSION=$(grep '"version"' package.json | head -1 | sed 's/.*: "\([^"]*\)".*/\1/')
OUTPUT_DIR="$DIST_DIR/decky"
BUILD_DIR="$OUTPUT_DIR/$PLUGIN_NAME"

echo "=== Building $PLUGIN_NAME v$VERSION ==="

# Detect package manager
detect_pm() {
    if command -v pnpm &> /dev/null; then
        echo "pnpm"
    elif command -v yarn &> /dev/null; then
        echo "yarn"
    elif command -v npm &> /dev/null; then
        echo "npm"
    else
        echo ""
    fi
}

PM=$(detect_pm)

if [ -z "$PM" ]; then
    echo "ERROR: No package manager found (npm, pnpm, or yarn)"
    echo ""
    echo "Install Node.js first:"
    echo "  Option 1: toolbox create dev && toolbox enter dev && sudo dnf install nodejs"
    echo "  Option 2: rpm-ostree install nodejs && systemctl reboot"
    echo "  Option 3: curl -fsSL https://fnm.vercel.app/install | bash"
    exit 1
fi

echo "Using package manager: $PM"

# Clean previous builds
rm -rf "$OUTPUT_DIR"
mkdir -p "$BUILD_DIR"

# Install dependencies if needed
if [ ! -d "node_modules" ]; then
    echo "Installing dependencies..."
    $PM install
fi

# Build frontend
echo "Building frontend..."
$PM run build

# Install Python dependencies into py_modules (bundled with the plugin)
echo "Installing Python dependencies..."
rm -rf py_modules
mkdir -p py_modules
python3 -m pip install --target py_modules -r requirements.txt --no-cache-dir

# Copy files to build directory
echo "Copying files..."
cp plugin.json "$BUILD_DIR/"
cp package.json "$BUILD_DIR/"
for pyfile in main.py steam_utils.py mdns_service.py pairing.py upload.py artwork.py ws_server.py; do
    cp "$pyfile" "$BUILD_DIR/"
done
cp requirements.txt "$BUILD_DIR/"
cp -r py_modules "$BUILD_DIR/"

# Copy dist (frontend bundle)
if [ -d "dist" ]; then
    cp -r dist "$BUILD_DIR/"
else
    echo "ERROR: dist/ not found. Frontend build failed?"
    exit 1
fi

# Copy assets
if [ -d "assets" ]; then
    cp -r assets "$BUILD_DIR/"
fi

# Copy LICENSE from root
if [ -f "../../LICENSE" ]; then
    cp "../../LICENSE" "$BUILD_DIR/"
fi

# Create ZIP
echo "Creating ZIP..."
cd "$OUTPUT_DIR"
zip -r "${PLUGIN_NAME}-v${VERSION}.zip" "$PLUGIN_NAME"

echo ""
echo "=== Build complete! ==="
echo "Output: $OUTPUT_DIR/${PLUGIN_NAME}-v${VERSION}.zip"
echo ""
echo "Installation options:"
echo "  1. Manual: Copy $BUILD_DIR to ~/homebrew/plugins/ on Steam Deck"
echo "  2. URL: Host the ZIP and use Decky Settings > Install from URL"
echo "  3. Dev: Use decky-cli to deploy during development"
