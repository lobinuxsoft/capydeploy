#!/bin/bash
# Build AppImage for CapyDeploy Hub and Agent
# Usage: ./build-appimage.sh [hub|agent|all]

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$ROOT_DIR/dist/appimage"
TOOLS_DIR="$ROOT_DIR/.tools"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Architecture
ARCH=$(uname -m)
case $ARCH in
    x86_64) APPIMAGE_ARCH="x86_64" ;;
    aarch64) APPIMAGE_ARCH="aarch64" ;;
    *) echo -e "${RED}Unsupported architecture: $ARCH${NC}"; exit 1 ;;
esac

echo "============================================"
echo "  CapyDeploy AppImage Builder"
echo "============================================"
echo

# Parse arguments
TARGET="${1:-all}"
if [[ ! "$TARGET" =~ ^(hub|agent|all)$ ]]; then
    echo "Usage: $0 [hub|agent|all]"
    exit 1
fi

# ============================================
# Download appimagetool if needed
# ============================================

APPIMAGETOOL="$TOOLS_DIR/appimagetool-$APPIMAGE_ARCH.AppImage"

if [ ! -f "$APPIMAGETOOL" ]; then
    echo -e "${YELLOW}[1/5]${NC} Downloading appimagetool..."
    mkdir -p "$TOOLS_DIR"
    curl -L -o "$APPIMAGETOOL" \
        "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-$APPIMAGE_ARCH.AppImage"
    chmod +x "$APPIMAGETOOL"
    echo -e "  ${GREEN}Downloaded${NC}"
else
    echo -e "${YELLOW}[1/5]${NC} appimagetool already available"
fi
echo

# ============================================
# Build function
# ============================================

build_appimage() {
    local APP_NAME="$1"
    local APP_DIR="$ROOT_DIR/apps/$APP_NAME"
    local APPDIR="$BUILD_DIR/${APP_NAME}.AppDir"
    local BINARY_NAME="capydeploy-$APP_NAME"
    local DESKTOP_NAME="capydeploy-$APP_NAME"

    echo "----------------------------------------"
    echo "  Building: $APP_NAME"
    echo "----------------------------------------"
    echo

    # Clean previous build
    rm -rf "$APPDIR"
    mkdir -p "$APPDIR/usr/bin"

    # Build with Wails
    echo -e "${YELLOW}[2/5]${NC} Building $APP_NAME with Wails..."
    pushd "$APP_DIR" > /dev/null

    # Get version info
    VERSION=$(git describe --tags --always 2>/dev/null || echo "dev")
    COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
    BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

    BASE_VERSION="0.1.0"
    EXACT_TAG=$(git describe --tags --exact-match 2>/dev/null || echo "")
    if [[ "$EXACT_TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        VERSION="${EXACT_TAG#v}"
    else
        VERSION="${BASE_VERSION}-dev+${COMMIT}"
    fi

    LDFLAGS="-X github.com/lobinuxsoft/capydeploy/pkg/version.Version=$VERSION"
    LDFLAGS="$LDFLAGS -X github.com/lobinuxsoft/capydeploy/pkg/version.Commit=$COMMIT"
    LDFLAGS="$LDFLAGS -X github.com/lobinuxsoft/capydeploy/pkg/version.BuildDate=$BUILD_DATE"

    wails build -clean -tags webkit2_41 -ldflags "$LDFLAGS" -o "$BINARY_NAME"

    popd > /dev/null
    echo -e "  ${GREEN}Build complete${NC}"
    echo

    # Copy binary
    echo -e "${YELLOW}[3/5]${NC} Creating AppDir structure..."
    cp "$APP_DIR/build/bin/$BINARY_NAME" "$APPDIR/usr/bin/"

    # Copy icon
    cp "$APP_DIR/build/appicon.png" "$APPDIR/$DESKTOP_NAME.png"

    # Create .desktop file
    cat > "$APPDIR/$DESKTOP_NAME.desktop" << DESKTOP
[Desktop Entry]
Name=CapyDeploy ${APP_NAME^}
Comment=Deploy games to Steam Deck and Linux devices
Exec=$BINARY_NAME
Icon=$DESKTOP_NAME
Type=Application
Categories=Utility;
Terminal=false
DESKTOP

    # Create AppRun with auto-install support
    cat > "$APPDIR/AppRun" << 'APPRUN'
#!/bin/bash
SELF=$(readlink -f "$0")
HERE=${SELF%/*}
APPIMAGE="${APPIMAGE:-$SELF}"
APP_NAME="BINARY_NAME"
DESKTOP_NAME="DESKTOP_FILE"
INSTALL_DIR="$HOME/Applications"
DESKTOP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons"

install_app() {
    echo "Installing $APP_NAME..."

    # Create directories
    mkdir -p "$INSTALL_DIR" "$DESKTOP_DIR" "$ICON_DIR"

    # Copy AppImage
    DEST="$INSTALL_DIR/$(basename "$APPIMAGE")"
    if [ "$APPIMAGE" != "$DEST" ]; then
        cp "$APPIMAGE" "$DEST"
        chmod +x "$DEST"
        echo "  Copied to: $DEST"
    fi

    # Extract and copy icon
    "$DEST" --appimage-extract "$DESKTOP_NAME.png" >/dev/null 2>&1
    if [ -f "squashfs-root/$DESKTOP_NAME.png" ]; then
        cp "squashfs-root/$DESKTOP_NAME.png" "$ICON_DIR/$DESKTOP_NAME.png"
        rm -rf squashfs-root
        echo "  Icon installed"
    fi

    # Create .desktop file
    cat > "$DESKTOP_DIR/$DESKTOP_NAME.desktop" << DESKTOP
[Desktop Entry]
Name=APP_DISPLAY_NAME
Comment=Deploy games to Steam Deck and Linux devices
Exec=$DEST
Icon=$ICON_DIR/$DESKTOP_NAME.png
Type=Application
Categories=Utility;
Terminal=false
DESKTOP
    echo "  Desktop entry created"
    echo ""
    echo "Installation complete! You can find the app in your application menu."
}

uninstall_app() {
    echo "Uninstalling $APP_NAME..."
    rm -f "$INSTALL_DIR"/*"$APP_NAME"*.AppImage
    rm -f "$DESKTOP_DIR/$DESKTOP_NAME.desktop"
    rm -f "$ICON_DIR/$DESKTOP_NAME.png"
    echo "Uninstalled."
}

# Handle special arguments
case "$1" in
    --install)
        install_app
        exit 0
        ;;
    --uninstall)
        uninstall_app
        exit 0
        ;;
    --help)
        echo "Usage: $(basename "$APPIMAGE") [OPTIONS]"
        echo ""
        echo "Options:"
        echo "  --install     Install to ~/Applications and create desktop entry"
        echo "  --uninstall   Remove installation"
        echo "  --help        Show this help"
        exit 0
        ;;
esac

# Auto-install prompt if not in ~/Applications
if [[ "$APPIMAGE" != "$INSTALL_DIR"/* ]] && [ -t 0 ]; then
    echo ""
    echo "$APP_NAME is not installed."
    read -p "Install to ~/Applications? [y/N] " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        install_app
        echo "Launching..."
        exec "$INSTALL_DIR/$(basename "$APPIMAGE")" "$@"
    fi
fi

# Run the app
export PATH="${HERE}/usr/bin:${PATH}"
exec "${HERE}/usr/bin/BINARY_NAME" "$@"
APPRUN
    sed -i "s/BINARY_NAME/$BINARY_NAME/g" "$APPDIR/AppRun"
    sed -i "s/DESKTOP_FILE/$DESKTOP_NAME/g" "$APPDIR/AppRun"
    sed -i "s/APP_DISPLAY_NAME/CapyDeploy ${APP_NAME^}/g" "$APPDIR/AppRun"
    chmod +x "$APPDIR/AppRun"

    echo -e "  ${GREEN}AppDir created${NC}"
    echo

    # Build AppImage
    echo -e "${YELLOW}[4/5]${NC} Generating AppImage..."
    mkdir -p "$BUILD_DIR"

    APPIMAGE_NAME="CapyDeploy_${APP_NAME^}-${VERSION}-${APPIMAGE_ARCH}.AppImage"

    # Run appimagetool
    ARCH=$APPIMAGE_ARCH "$APPIMAGETOOL" "$APPDIR" "$BUILD_DIR/$APPIMAGE_NAME" 2>/dev/null

    echo -e "  ${GREEN}AppImage created: $APPIMAGE_NAME${NC}"
    echo

    # Cleanup AppDir
    rm -rf "$APPDIR"
}

# ============================================
# Build targets
# ============================================

echo -e "${YELLOW}[2/5]${NC} Building target(s): $TARGET"
echo

if [ "$TARGET" = "all" ] || [ "$TARGET" = "hub" ]; then
    build_appimage "hub"
fi

if [ "$TARGET" = "all" ] || [ "$TARGET" = "agent" ]; then
    build_appimage "agent"
fi

# ============================================
# Summary
# ============================================

echo "============================================"
echo -e "  ${GREEN}Build Complete!${NC}"
echo "============================================"
echo
echo "Output directory: $BUILD_DIR"
echo
ls -lh "$BUILD_DIR"/*.AppImage 2>/dev/null || echo "No AppImages found"
echo
echo "To run:"
echo "  chmod +x dist/appimage/*.AppImage"
echo "  ./dist/appimage/CapyDeploy_Hub-*.AppImage"
