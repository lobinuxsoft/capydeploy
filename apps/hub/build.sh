#!/bin/bash

set -e

echo "============================================"
echo "  CapyDeploy Hub - Build Script"
echo "============================================"
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
MODE="production"
SKIP_DEPS=0

# Project directories
ROOT_DIR="$(cd ../.. && pwd)"
DIST_DIR="$ROOT_DIR/dist"
TOOLS_DIR="$ROOT_DIR/.tools"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        dev|--dev|-d)
            MODE="dev"
            shift
            ;;
        --skip-deps)
            SKIP_DEPS=1
            shift
            ;;
        --help|-h)
            echo "Usage: ./build.sh [options]"
            echo
            echo "Options:"
            echo "  dev, --dev, -d    Start in development mode with hot reload"
            echo "  --skip-deps       Skip frontend dependency installation"
            echo "  --help, -h        Show this help message"
            echo
            echo "Examples:"
            echo "  ./build.sh              Build production binary"
            echo "  ./build.sh dev          Start development server"
            echo "  ./build.sh --skip-deps  Build without reinstalling deps"
            echo
            exit 0
            ;;
        *)
            echo -e "${RED}[ERROR]${NC} Unknown option: $1"
            echo "Use --help for usage information."
            exit 1
            ;;
    esac
done

# ============================================
# Check required tools
# ============================================

echo -e "${YELLOW}[1/7]${NC} Checking required tools..."
echo

# Check Go
if ! command -v go &> /dev/null; then
    echo -e "${RED}[ERROR]${NC} Go not found."
    echo "Please install Go 1.23+ from: https://go.dev/dl/"
    exit 1
fi
GO_VERSION=$(go version | awk '{print $3}')
echo -e "  Go: ${GREEN}${GO_VERSION}${NC}"

# Check Bun
if ! command -v bun &> /dev/null; then
    echo -e "${YELLOW}[WARN]${NC} Bun not found. Installing..."
    curl -fsSL https://bun.sh/install | bash

    # Source the updated profile
    export BUN_INSTALL="$HOME/.bun"
    export PATH="$BUN_INSTALL/bin:$PATH"

    if ! command -v bun &> /dev/null; then
        echo -e "${RED}[ERROR]${NC} Failed to install Bun."
        echo "Please install manually from: https://bun.sh"
        echo "Then restart your terminal and run this script again."
        exit 1
    fi
    echo "Bun installed."
fi
BUN_VERSION=$(bun --version 2>/dev/null || echo "unknown")
echo -e "  Bun: ${GREEN}${BUN_VERSION}${NC}"

# Check Wails
if ! command -v wails &> /dev/null; then
    echo
    echo -e "${YELLOW}[WARN]${NC} Wails CLI not found. Installing..."
    go install github.com/wailsapp/wails/v2/cmd/wails@latest

    # Add Go bin to PATH if not already there
    export PATH="$PATH:$(go env GOPATH)/bin"

    if ! command -v wails &> /dev/null; then
        echo -e "${RED}[ERROR]${NC} Failed to install Wails CLI."
        echo "Make sure $(go env GOPATH)/bin is in your PATH."
        exit 1
    fi
    echo "Wails CLI installed."
fi
WAILS_VERSION=$(wails version 2>/dev/null | grep -i version | awk '{print $2}' || echo "unknown")
echo -e "  Wails: ${GREEN}${WAILS_VERSION}${NC}"

echo
echo -e "  ${GREEN}All tools OK!${NC}"
echo

# ============================================
# Install frontend dependencies
# ============================================

if [ $SKIP_DEPS -eq 0 ]; then
    echo -e "${YELLOW}[2/7]${NC} Installing frontend dependencies..."
    cd frontend
    bun install
    if [ $? -ne 0 ]; then
        echo -e "${RED}[ERROR]${NC} Failed to install frontend dependencies."
        cd ..
        exit 1
    fi
    cd ..
    echo "  Dependencies installed."
    echo
else
    echo -e "${YELLOW}[2/7]${NC} Skipping frontend dependencies (--skip-deps)"
    echo
fi

# ============================================
# Generate icons (production only)
# ============================================

if [ "$MODE" = "production" ]; then
    echo -e "${YELLOW}[3/7]${NC} Generating application icons..."
    echo

    ICON_SCRIPT="../../scripts/generate-icons.py"
    if [ -f "$ICON_SCRIPT" ]; then
        if command -v python3 &> /dev/null; then
            python3 "$ICON_SCRIPT"
        elif command -v python &> /dev/null; then
            python "$ICON_SCRIPT"
        else
            echo -e "  ${YELLOW}[WARN]${NC} Python not found, skipping icon generation."
            echo "  Install Python 3 + Pillow to generate icons."
        fi
    else
        echo -e "  ${YELLOW}[WARN]${NC} Icon script not found: $ICON_SCRIPT"
    fi
    echo
else
    echo -e "${YELLOW}[3/7]${NC} Skipping icon generation (dev mode)"
    echo
fi

# ============================================
# Version info (SemVer from git)
# ============================================

echo -e "${YELLOW}[4/7]${NC} Collecting version info..."
echo

# Base version (must match pkg/version/version.go)
BASE_VERSION="0.1.0"

# Get commit hash
COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Check if we're on an exact version tag (v0.1.0, v1.0.0, etc.)
EXACT_TAG=$(git describe --tags --exact-match 2>/dev/null || echo "")

if [[ "$EXACT_TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    # Release build: use tag version (strip 'v' prefix)
    VERSION="${EXACT_TAG#v}"
else
    # Development build: version-dev+commit
    VERSION="${BASE_VERSION}-dev+${COMMIT}"
fi

echo "  Version:    $VERSION"
echo "  Commit:     $COMMIT"
echo "  Build Date: $BUILD_DATE"
echo

LDFLAGS="-X github.com/lobinuxsoft/capydeploy/pkg/version.Version=$VERSION"
LDFLAGS="$LDFLAGS -X github.com/lobinuxsoft/capydeploy/pkg/version.Commit=$COMMIT"
LDFLAGS="$LDFLAGS -X github.com/lobinuxsoft/capydeploy/pkg/version.BuildDate=$BUILD_DATE"

# ============================================
# Build embedded binary (steam-shortcut-manager for Linux)
# ============================================

echo -e "${YELLOW}[5/7]${NC} Building embedded steam-shortcut-manager (Linux)..."
echo

pushd ../../steam-shortcut-manager > /dev/null
GOOS=linux GOARCH=amd64 CGO_ENABLED=0 go build -o ../internal/embedded/steam-shortcut-manager .
if [ $? -ne 0 ]; then
    echo -e "${RED}[ERROR]${NC} Failed to build steam-shortcut-manager."
    popd > /dev/null
    exit 1
fi
popd > /dev/null

echo "  steam-shortcut-manager built successfully."
echo

# ============================================
# Build
# ============================================

if [ "$MODE" = "dev" ]; then
    echo -e "${YELLOW}[6/7]${NC} Starting development server..."
    echo
    echo "  Press Ctrl+C to stop."
    echo
    wails dev -tags webkit2_41 -ldflags "$LDFLAGS"
else
    echo -e "${YELLOW}[6/7]${NC} Building production binary..."
    echo

    if ! wails build -clean -tags webkit2_41 -ldflags "$LDFLAGS"; then
        echo
        echo "============================================"
        echo -e "  ${RED}BUILD FAILED${NC}"
        echo "============================================"
        exit 1
    fi

    echo
    echo -e "  ${GREEN}Wails build complete${NC}"
    echo

    # ============================================
    # Copy binary to dist/
    # ============================================

    mkdir -p "$DIST_DIR/linux"
    cp "build/bin/capydeploy-hub" "$DIST_DIR/linux/"
    echo "  Copied to: $DIST_DIR/linux/capydeploy-hub"
    echo

    # ============================================
    # Cross-compile for Windows (if mingw available)
    # ============================================

    if command -v x86_64-w64-mingw32-gcc &> /dev/null; then
        echo "  Cross-compiling for Windows..."
        if wails build -platform windows/amd64 -clean -ldflags "$LDFLAGS" 2>/dev/null; then
            mkdir -p "$DIST_DIR/windows"
            cp "build/bin/capydeploy-hub.exe" "$DIST_DIR/windows/"
            echo "  Copied to: $DIST_DIR/windows/capydeploy-hub.exe"
        else
            echo -e "  ${YELLOW}[WARN]${NC} Windows cross-compile failed, skipping"
        fi
        echo
    else
        echo -e "  ${YELLOW}[INFO]${NC} mingw-w64 not found, skipping Windows build"
        echo "  Install with: rpm-ostree install mingw64-gcc"
        echo
    fi

    # ============================================
    # Generate AppImage (Linux only)
    # ============================================

    echo -e "${YELLOW}[7/7]${NC} Generating AppImage..."
    echo

    APPIMAGE_DIR="$DIST_DIR/appimage"

    ARCH=$(uname -m)
    case $ARCH in
        x86_64) APPIMAGE_ARCH="x86_64" ;;
        aarch64) APPIMAGE_ARCH="aarch64" ;;
        *)
            echo -e "  ${YELLOW}[WARN]${NC} Unsupported architecture for AppImage: $ARCH"
            echo "  Skipping AppImage generation."
            echo
            # Still show binary output
            if [ -f "$DIST_DIR/linux/capydeploy-hub" ]; then
                SIZE=$(stat -f%z "$DIST_DIR/linux/capydeploy-hub" 2>/dev/null || stat -c%s "$DIST_DIR/linux/capydeploy-hub" 2>/dev/null || echo "0")
                SIZE_MB=$((SIZE / 1048576))
                echo "  Binary: $DIST_DIR/linux/capydeploy-hub (${SIZE_MB} MB)"
            fi
            echo
            echo "Done!"
            exit 0
            ;;
    esac

    # Download appimagetool if needed
    APPIMAGETOOL="$TOOLS_DIR/appimagetool-$APPIMAGE_ARCH.AppImage"

    if [ ! -f "$APPIMAGETOOL" ]; then
        echo "  Downloading appimagetool..."
        mkdir -p "$TOOLS_DIR"
        curl -L -o "$APPIMAGETOOL" \
            "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-$APPIMAGE_ARCH.AppImage"
        chmod +x "$APPIMAGETOOL"
        echo -e "  ${GREEN}Downloaded${NC}"
    else
        echo "  appimagetool already available"
    fi

    # Create AppDir
    APPDIR="$APPIMAGE_DIR/hub.AppDir"
    BINARY_NAME="capydeploy-hub"
    DESKTOP_NAME="capydeploy-hub"

    rm -rf "$APPDIR"
    mkdir -p "$APPDIR/usr/bin"

    # Copy binary
    cp "build/bin/$BINARY_NAME" "$APPDIR/usr/bin/"

    # Copy icon
    cp "build/appicon.png" "$APPDIR/$DESKTOP_NAME.png"

    # Create .desktop file
    cat > "$APPDIR/$DESKTOP_NAME.desktop" << DESKTOP
[Desktop Entry]
Name=CapyDeploy Hub
Comment=Deploy games to Steam Deck and Linux devices
Exec=$BINARY_NAME
Icon=$DESKTOP_NAME
Type=Application
Categories=Game;Development;
Terminal=false
DESKTOP

    # Create AppRun
    cat > "$APPDIR/AppRun" << 'APPRUN'
#!/bin/bash
SELF=$(readlink -f "$0")
HERE=${SELF%/*}
APPIMAGE="${APPIMAGE:-$SELF}"
APP_NAME="BINARY_NAME"
DESKTOP_NAME="DESKTOP_FILE"
INSTALL_DIR="$HOME/.local/bin"
DESKTOP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons"

install_app() {
    echo "Installing $APP_NAME..."

    # Create directories
    mkdir -p "$INSTALL_DIR" "$DESKTOP_DIR" "$ICON_DIR"

    # Move AppImage
    DEST="$INSTALL_DIR/$(basename "$APPIMAGE")"
    if [ "$APPIMAGE" != "$DEST" ]; then
        mv "$APPIMAGE" "$DEST"
        chmod +x "$DEST"
        echo "  Moved to: $DEST"
    fi

    # Extract and copy icon
    "$DEST" --appimage-extract "$DESKTOP_NAME.png" >/dev/null 2>&1
    if [ -f "squashfs-root/$DESKTOP_NAME.png" ]; then
        cp "squashfs-root/$DESKTOP_NAME.png" "$ICON_DIR/$DESKTOP_NAME.png"
        rm -rf squashfs-root
        echo "  Icon installed"
    fi

    # Create .desktop file (uses --run to skip install check)
    cat > "$DESKTOP_DIR/$DESKTOP_NAME.desktop" << DESKTOP
[Desktop Entry]
Name=APP_DISPLAY_NAME
Comment=Deploy games to Steam Deck and Linux devices
Exec=$DEST --run
Icon=$ICON_DIR/$DESKTOP_NAME.png
Type=Application
Categories=Game;Development;
Terminal=false
DESKTOP
    echo "  Desktop entry created"
    echo ""
    echo "Installation complete! You can find the app in your application menu."
}

uninstall_app() {
    echo "Uninstalling $APP_NAME..."
    # Remove AppImage (case-insensitive match for CapyDeploy_Hub or capydeploy-hub)
    find "$INSTALL_DIR" -maxdepth 1 -iname "*capydeploy*hub*.AppImage" -delete 2>/dev/null
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
    --run)
        # Skip install check, just run (used by .desktop)
        shift
        export PATH="${HERE}/usr/bin:${PATH}"
        exec "${HERE}/usr/bin/BINARY_NAME" "$@"
        ;;
    --help)
        echo "Usage: $(basename "$APPIMAGE") [OPTIONS]"
        echo ""
        echo "Options:"
        echo "  --install     Install to ~/Applications and create desktop entry"
        echo "  --uninstall   Remove installation"
        echo "  --run         Run without install check (used by .desktop)"
        echo "  --help        Show this help"
        exit 0
        ;;
esac

# Auto-install prompt only when double-clicked (no terminal)
# If running from terminal, just run the app without prompts
if [[ "$APPIMAGE" != "$INSTALL_DIR"/* ]] && [ ! -t 0 ]; then
    # No terminal = likely double-clicked from file manager
    if command -v zenity &>/dev/null; then
        if zenity --question --title="Install $APP_NAME" \
            --text="Install to ~/.local/bin and create menu entry?" \
            --width=300 2>/dev/null; then
            install_app
            exec "$INSTALL_DIR/$(basename "$APPIMAGE")" --run "$@"
        fi
    elif command -v kdialog &>/dev/null; then
        if kdialog --yesno "Install to ~/.local/bin and create menu entry?" \
            --title "Install $APP_NAME" 2>/dev/null; then
            install_app
            exec "$INSTALL_DIR/$(basename "$APPIMAGE")" --run "$@"
        fi
    fi
fi

# Run the app
export PATH="${HERE}/usr/bin:${PATH}"
exec "${HERE}/usr/bin/BINARY_NAME" "$@"
APPRUN
    sed -i "s/BINARY_NAME/$BINARY_NAME/g" "$APPDIR/AppRun"
    sed -i "s/DESKTOP_FILE/$DESKTOP_NAME/g" "$APPDIR/AppRun"
    sed -i "s/APP_DISPLAY_NAME/CapyDeploy Hub/g" "$APPDIR/AppRun"
    chmod +x "$APPDIR/AppRun"

    echo -e "  ${GREEN}AppDir created${NC}"

    # Build AppImage
    mkdir -p "$APPIMAGE_DIR"
    APPIMAGE_NAME="CapyDeploy_Hub.AppImage"

    ARCH=$APPIMAGE_ARCH "$APPIMAGETOOL" "$APPDIR" "$APPIMAGE_DIR/$APPIMAGE_NAME" 2>/dev/null

    echo -e "  ${GREEN}AppImage created: $APPIMAGE_NAME${NC}"

    # Cleanup AppDir
    rm -rf "$APPDIR"

    echo
    echo "============================================"
    echo -e "  ${GREEN}BUILD SUCCESSFUL${NC}"
    echo "============================================"
    echo

    # Show results
    echo "  Output directory: $DIST_DIR"
    echo

    if [ -f "$DIST_DIR/linux/capydeploy-hub" ]; then
        SIZE=$(stat -f%z "$DIST_DIR/linux/capydeploy-hub" 2>/dev/null || stat -c%s "$DIST_DIR/linux/capydeploy-hub" 2>/dev/null || echo "0")
        SIZE_MB=$((SIZE / 1048576))
        echo "  Linux:    $DIST_DIR/linux/capydeploy-hub (${SIZE_MB} MB)"
    fi

    if [ -f "$DIST_DIR/windows/capydeploy-hub.exe" ]; then
        SIZE=$(stat -f%z "$DIST_DIR/windows/capydeploy-hub.exe" 2>/dev/null || stat -c%s "$DIST_DIR/windows/capydeploy-hub.exe" 2>/dev/null || echo "0")
        SIZE_MB=$((SIZE / 1048576))
        echo "  Windows:  $DIST_DIR/windows/capydeploy-hub.exe (${SIZE_MB} MB)"
    fi

    APPIMAGE_FILE="$APPIMAGE_DIR/$APPIMAGE_NAME"
    if [ -f "$APPIMAGE_FILE" ]; then
        SIZE=$(stat -f%z "$APPIMAGE_FILE" 2>/dev/null || stat -c%s "$APPIMAGE_FILE" 2>/dev/null || echo "0")
        SIZE_MB=$((SIZE / 1048576))
        echo "  AppImage: $APPIMAGE_FILE (${SIZE_MB} MB)"
    fi

    echo
    echo "Done!"
fi
