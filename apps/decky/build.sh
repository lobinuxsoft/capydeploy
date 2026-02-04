#!/bin/bash
# Build script for CapyDeploy Decky Plugin

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

PLUGIN_NAME="CapyDeploy"
VERSION=$(grep '"version"' package.json | head -1 | sed 's/.*: "\([^"]*\)".*/\1/')
OUTPUT_DIR="$SCRIPT_DIR/out"
BUILD_DIR="$OUTPUT_DIR/$PLUGIN_NAME"

echo "=== Building $PLUGIN_NAME v$VERSION ==="

# Clean previous builds
rm -rf "$OUTPUT_DIR"
mkdir -p "$BUILD_DIR"

# Install dependencies if needed
if [ ! -d "node_modules" ]; then
    echo "Installing dependencies..."
    pnpm install
fi

# Build frontend
echo "Building frontend..."
pnpm build

# Copy files to build directory
echo "Copying files..."
cp plugin.json "$BUILD_DIR/"
cp package.json "$BUILD_DIR/"
cp main.py "$BUILD_DIR/"
cp requirements.txt "$BUILD_DIR/"

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
