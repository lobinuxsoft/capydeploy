#!/bin/bash

set -e

echo "============================================"
echo "  CapyDeploy - Build All (Linux)"
echo "============================================"
echo

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Project root
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

# Parse arguments
SKIP_DEPS=0
PARALLEL=0

while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-deps)
            SKIP_DEPS=1
            shift
            ;;
        --parallel|-p)
            PARALLEL=1
            shift
            ;;
        --help|-h)
            echo "Usage: ./build_all.sh [options]"
            echo
            echo "Options:"
            echo "  --skip-deps       Skip frontend dependency installation"
            echo "  --parallel, -p    Build components in parallel (faster but messy output)"
            echo "  --help, -h        Show this help message"
            echo
            echo "Builds:"
            echo "  - Hub (Tauri + Rust)"
            echo "  - Desktop Agent (Tauri + Rust)"
            echo "  - Decky Plugin (TypeScript + Python)"
            echo
            echo "Output: dist/"
            exit 0
            ;;
        *)
            echo -e "${RED}[ERROR]${NC} Unknown option: $1"
            exit 1
            ;;
    esac
done

DIST_DIR="$ROOT_DIR/dist"
TOOLS_DIR="$ROOT_DIR/.tools"

# Track results
declare -A RESULTS

# ============================================
# Helper: generate AppImage
# ============================================
# Usage: generate_appimage <binary_name> <display_name> <icon_path> <appimage_name> <desktop_id>
generate_appimage() {
    local binary_name="$1"
    local display_name="$2"
    local icon_path="$3"
    local appimage_name="$4"
    local desktop_id="$5"

    local appimage_dir="$DIST_DIR/appimage"
    local appdir="$appimage_dir/${desktop_id}.AppDir"

    # Determine arch
    local arch
    arch=$(uname -m)
    local appimage_arch
    case $arch in
        x86_64)  appimage_arch="x86_64" ;;
        aarch64) appimage_arch="aarch64" ;;
        *)
            echo -e "  ${YELLOW}[WARN]${NC} Unsupported architecture for AppImage: $arch"
            return 1
            ;;
    esac

    # Download appimagetool if needed
    local appimagetool="$TOOLS_DIR/appimagetool-$appimage_arch.AppImage"
    if [ ! -f "$appimagetool" ]; then
        echo "  Downloading appimagetool..."
        mkdir -p "$TOOLS_DIR"
        curl -L -o "$appimagetool" \
            "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-$appimage_arch.AppImage"
        chmod +x "$appimagetool"
        echo -e "  ${GREEN}Downloaded${NC}"
    fi

    # Create AppDir
    rm -rf "$appdir"
    mkdir -p "$appdir/usr/bin"

    # Copy binary
    cp "$DIST_DIR/linux/$binary_name" "$appdir/usr/bin/"

    # Copy icon
    if [ -f "$ROOT_DIR/$icon_path" ]; then
        cp "$ROOT_DIR/$icon_path" "$appdir/$desktop_id.png"
    else
        echo -e "  ${YELLOW}[WARN]${NC} Icon not found: $icon_path"
    fi

    # Create .desktop file
    cat > "$appdir/$desktop_id.desktop" << DESKTOP
[Desktop Entry]
Name=$display_name
Comment=Deploy games to Steam Deck and Linux devices
Exec=$binary_name
Icon=$desktop_id
Type=Application
Categories=Game;Development;
Terminal=false
DESKTOP

    # Create AppRun with install/uninstall support
    cat > "$appdir/AppRun" << 'APPRUN'
#!/bin/bash
SELF=$(readlink -f "$0")
HERE=${SELF%/*}
APPIMAGE="${APPIMAGE:-$SELF}"
APP_NAME="__DISPLAY_NAME__"
BINARY_NAME="__BINARY_NAME__"
DESKTOP_ID="__DESKTOP_ID__"
INSTALL_DIR="$HOME/.local/bin"
DESKTOP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons"

install_app() {
    echo "Installing $APP_NAME..."

    mkdir -p "$INSTALL_DIR" "$DESKTOP_DIR" "$ICON_DIR"

    # Move AppImage
    DEST="$INSTALL_DIR/$(basename "$APPIMAGE")"
    if [ "$APPIMAGE" != "$DEST" ]; then
        mv "$APPIMAGE" "$DEST"
        chmod +x "$DEST"
        echo "  Moved to: $DEST"
    fi

    # Extract and install icon
    "$DEST" --appimage-extract "$DESKTOP_ID.png" >/dev/null 2>&1
    if [ -f "squashfs-root/$DESKTOP_ID.png" ]; then
        cp "squashfs-root/$DESKTOP_ID.png" "$ICON_DIR/$DESKTOP_ID.png"
        rm -rf squashfs-root
        echo "  Icon installed"
    fi

    # Create .desktop entry
    cat > "$DESKTOP_DIR/$DESKTOP_ID.desktop" << DESKTOP
[Desktop Entry]
Name=$APP_NAME
Comment=Deploy games to Steam Deck and Linux devices
Exec=$DEST --run
Icon=$ICON_DIR/$DESKTOP_ID.png
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
    find "$INSTALL_DIR" -maxdepth 1 -iname "*capydeploy*$(echo "$BINARY_NAME" | sed 's/capydeploy-//')*.AppImage" -delete 2>/dev/null
    rm -f "$DESKTOP_DIR/$DESKTOP_ID.desktop"
    rm -f "$ICON_DIR/$DESKTOP_ID.png"
    echo "Uninstalled."
}

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
        shift
        export PATH="${HERE}/usr/bin:${PATH}"
        exec "${HERE}/usr/bin/$BINARY_NAME" "$@"
        ;;
    --help)
        echo "Usage: $(basename "$APPIMAGE") [OPTIONS]"
        echo ""
        echo "Options:"
        echo "  --install     Install to ~/.local/bin and create desktop entry"
        echo "  --uninstall   Remove installation"
        echo "  --run         Run without install check (used by .desktop)"
        echo "  --help        Show this help"
        exit 0
        ;;
esac

# Auto-install prompt when double-clicked (no terminal attached)
if [[ "$APPIMAGE" != "$INSTALL_DIR"/* ]] && [ ! -t 0 ]; then
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
exec "${HERE}/usr/bin/$BINARY_NAME" "$@"
APPRUN

    # Replace placeholders
    sed -i "s/__BINARY_NAME__/$binary_name/g" "$appdir/AppRun"
    sed -i "s/__DISPLAY_NAME__/$display_name/g" "$appdir/AppRun"
    sed -i "s/__DESKTOP_ID__/$desktop_id/g" "$appdir/AppRun"
    chmod +x "$appdir/AppRun"

    # Build AppImage
    mkdir -p "$appimage_dir"
    ARCH=$appimage_arch "$appimagetool" "$appdir" "$appimage_dir/$appimage_name" 2>/dev/null

    # Cleanup AppDir
    rm -rf "$appdir"

    echo -e "  ${GREEN}$appimage_name created${NC}"
}

# ============================================
# [1/7] Check required tools
# ============================================

echo -e "${YELLOW}[1/7]${NC} Checking required tools..."
echo

if ! command -v cargo &> /dev/null; then
    echo -e "${RED}[ERROR]${NC} cargo not found."
    echo "Install Rust from: https://rustup.rs/"
    exit 1
fi
CARGO_VERSION=$(cargo --version | awk '{print $2}')
echo -e "  cargo: ${GREEN}${CARGO_VERSION}${NC}"

if ! command -v bun &> /dev/null; then
    echo -e "${RED}[ERROR]${NC} bun not found."
    echo "Install from: https://bun.sh"
    exit 1
fi
BUN_VERSION=$(bun --version 2>/dev/null || echo "unknown")
echo -e "  bun: ${GREEN}${BUN_VERSION}${NC}"

# npm is only needed for Decky if bun is unavailable (already checked above)
if command -v npm &> /dev/null; then
    NPM_VERSION=$(npm --version 2>/dev/null || echo "unknown")
    echo -e "  npm: ${GREEN}${NPM_VERSION}${NC} (for Decky)"
fi

echo
echo -e "  ${GREEN}All tools OK!${NC}"
echo

# ============================================
# [2/7] Init submodules
# ============================================

echo -e "${YELLOW}[2/7]${NC} Initializing submodules..."
git submodule update --init --recursive
echo -e "  ${GREEN}Done${NC}"
echo

# ============================================
# [3/7] Build frontends
# ============================================

echo -e "${YELLOW}[3/7]${NC} Building frontends..."
echo

build_frontend() {
    local name="$1"
    local dir="$2"

    echo -e "  ${YELLOW}[$name]${NC} Installing dependencies & building..."
    pushd "$ROOT_DIR/$dir" > /dev/null
    if [ $SKIP_DEPS -eq 0 ]; then
        bun install --frozen-lockfile 2>/dev/null || bun install
    fi
    bun run build
    popd > /dev/null
    echo -e "  ${GREEN}[$name]${NC} Frontend ready"
}

if [ $PARALLEL -eq 1 ]; then
    build_frontend "Hub" "apps/hub-tauri/frontend" &
    PID_HUB_FE=$!
    build_frontend "Agent" "apps/agents/agent-tauri/frontend" &
    PID_AGENT_FE=$!
    wait $PID_HUB_FE
    wait $PID_AGENT_FE
else
    build_frontend "Hub" "apps/hub-tauri/frontend"
    build_frontend "Agent" "apps/agents/agent-tauri/frontend"
fi

echo

# ============================================
# [4/7] Build Linux (cargo)
# ============================================

echo -e "${YELLOW}[4/7]${NC} Building Linux binaries (cargo release)..."
echo

if cargo build --release -p capydeploy-hub-tauri -p capydeploy-agent-tauri; then
    mkdir -p "$DIST_DIR/linux"
    cp "target/release/capydeploy-hub-tauri" "$DIST_DIR/linux/"
    cp "target/release/capydeploy-agent-tauri" "$DIST_DIR/linux/"
    RESULTS[linux]="success"
    echo
    echo -e "  ${GREEN}Linux binaries ready${NC}"
else
    RESULTS[linux]="failed"
    echo -e "  ${RED}Linux build FAILED${NC}"
fi

echo

# ============================================
# [5/7] Cross-compile for Windows
# ============================================

echo -e "${YELLOW}[5/7]${NC} Cross-compiling for Windows..."
echo

if rustup target list --installed 2>/dev/null | grep -q "x86_64-pc-windows-gnu" && \
   command -v x86_64-w64-mingw32-gcc &> /dev/null; then
    if cargo build --release --target x86_64-pc-windows-gnu \
        -p capydeploy-hub-tauri -p capydeploy-agent-tauri 2>/dev/null; then
        mkdir -p "$DIST_DIR/windows"
        cp "target/x86_64-pc-windows-gnu/release/capydeploy-hub-tauri.exe" "$DIST_DIR/windows/"
        cp "target/x86_64-pc-windows-gnu/release/capydeploy-agent-tauri.exe" "$DIST_DIR/windows/"
        RESULTS[windows]="success"
        echo -e "  ${GREEN}Windows binaries ready${NC}"
    else
        RESULTS[windows]="failed"
        echo -e "  ${YELLOW}[WARN]${NC} Windows cross-compile failed"
    fi
else
    RESULTS[windows]="skipped"
    echo -e "  ${YELLOW}[INFO]${NC} Windows cross-compile not available"
    echo "  Need: rustup target add x86_64-pc-windows-gnu"
    echo "  Need: rpm-ostree install mingw64-gcc"
fi

echo

# ============================================
# [6/7] Generate AppImages
# ============================================

echo -e "${YELLOW}[6/7]${NC} Generating AppImages..."
echo

if [ "${RESULTS[linux]}" = "success" ]; then
    generate_appimage \
        "capydeploy-hub-tauri" \
        "CapyDeploy Hub" \
        "apps/hub-tauri/src-tauri/icons/icon.png" \
        "CapyDeploy_Hub.AppImage" \
        "capydeploy-hub" && RESULTS[appimage_hub]="success" || RESULTS[appimage_hub]="failed"

    generate_appimage \
        "capydeploy-agent-tauri" \
        "CapyDeploy Agent" \
        "apps/agents/agent-tauri/src-tauri/icons/icon.png" \
        "CapyDeploy_Agent.AppImage" \
        "capydeploy-agent" && RESULTS[appimage_agent]="success" || RESULTS[appimage_agent]="failed"
else
    echo -e "  ${YELLOW}[SKIP]${NC} Linux build failed, skipping AppImages"
    RESULTS[appimage_hub]="skipped"
    RESULTS[appimage_agent]="skipped"
fi

echo

# ============================================
# [7/7] Build Decky plugin
# ============================================

echo -e "${YELLOW}[7/7]${NC} Building Decky plugin..."
echo

if [ -f "apps/agents/decky/build.sh" ]; then
    if (cd apps/agents/decky && ./build.sh); then
        RESULTS[decky]="success"
        echo -e "  ${GREEN}Decky plugin ready${NC}"
    else
        RESULTS[decky]="failed"
        echo -e "  ${RED}Decky build FAILED${NC}"
    fi
else
    RESULTS[decky]="skipped"
    echo -e "  ${YELLOW}[SKIP]${NC} Decky build script not found"
fi

echo
echo "============================================"
echo "  Build Summary"
echo "============================================"
echo

echo "Output directory: $DIST_DIR"
echo

# Helper to print file size
print_binary() {
    local label="$1"
    local path="$2"
    if [ -f "$path" ]; then
        local size
        size=$(stat -c%s "$path" 2>/dev/null || echo "0")
        local size_mb=$((size / 1048576))
        echo -e "  ${GREEN}✓${NC} $label: ${size_mb} MB"
    else
        echo -e "  ${RED}✗${NC} $label: NOT FOUND"
    fi
}

# Linux
echo -e "${YELLOW}Linux:${NC}"
print_binary "Hub" "$DIST_DIR/linux/capydeploy-hub-tauri"
print_binary "Agent" "$DIST_DIR/linux/capydeploy-agent-tauri"

# Windows
echo -e "${YELLOW}Windows:${NC}"
if [ "${RESULTS[windows]}" = "skipped" ]; then
    echo -e "  ${YELLOW}–${NC} Skipped (no cross-compile toolchain)"
else
    print_binary "Hub" "$DIST_DIR/windows/capydeploy-hub-tauri.exe"
    print_binary "Agent" "$DIST_DIR/windows/capydeploy-agent-tauri.exe"
fi

# AppImages
echo -e "${YELLOW}AppImages:${NC}"
print_binary "Hub" "$DIST_DIR/appimage/CapyDeploy_Hub.AppImage"
print_binary "Agent" "$DIST_DIR/appimage/CapyDeploy_Agent.AppImage"

# Decky
echo -e "${YELLOW}Decky:${NC}"
DECKY_ZIP=$(find "$DIST_DIR/decky" -name "*.zip" 2>/dev/null | head -1)
if [ -n "$DECKY_ZIP" ] && [ -f "$DECKY_ZIP" ]; then
    SIZE=$(stat -c%s "$DECKY_ZIP" 2>/dev/null || echo "0")
    SIZE_KB=$((SIZE / 1024))
    echo -e "  ${GREEN}✓${NC} Plugin: ${SIZE_KB} KB"
else
    echo -e "  ${RED}✗${NC} Plugin: NOT FOUND"
fi

echo
echo "Done!"
