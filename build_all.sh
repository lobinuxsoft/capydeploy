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
            echo "  - Hub (Wails + Go)"
            echo "  - Desktop Agent (Wails + Go)"
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

# Build arguments
BUILD_ARGS=""
if [ $SKIP_DEPS -eq 1 ]; then
    BUILD_ARGS="--skip-deps"
fi

# Track results
declare -A RESULTS

build_hub() {
    echo -e "${YELLOW}[HUB]${NC} Starting build..."
    if (cd apps/hub && ./build.sh $BUILD_ARGS); then
        RESULTS[hub]="success"
        echo -e "${GREEN}[HUB]${NC} Build complete"
    else
        RESULTS[hub]="failed"
        echo -e "${RED}[HUB]${NC} Build FAILED"
        return 1
    fi
}

build_desktop_agent() {
    echo -e "${YELLOW}[DESKTOP AGENT]${NC} Starting build..."
    if (cd apps/agents/desktop && ./build.sh $BUILD_ARGS); then
        RESULTS[desktop]="success"
        echo -e "${GREEN}[DESKTOP AGENT]${NC} Build complete"
    else
        RESULTS[desktop]="failed"
        echo -e "${RED}[DESKTOP AGENT]${NC} Build FAILED"
        return 1
    fi
}

build_decky() {
    echo -e "${YELLOW}[DECKY]${NC} Starting build..."
    if (cd apps/agents/decky && ./build.sh); then
        RESULTS[decky]="success"
        echo -e "${GREEN}[DECKY]${NC} Build complete"
    else
        RESULTS[decky]="failed"
        echo -e "${RED}[DECKY]${NC} Build FAILED"
        return 1
    fi
}

# Execute builds
if [ $PARALLEL -eq 1 ]; then
    echo -e "${YELLOW}Building in parallel...${NC}"
    echo

    build_hub &
    PID_HUB=$!

    build_desktop_agent &
    PID_DESKTOP=$!

    build_decky &
    PID_DECKY=$!

    # Wait for all
    wait $PID_HUB || true
    wait $PID_DESKTOP || true
    wait $PID_DECKY || true
else
    echo -e "${YELLOW}Building sequentially...${NC}"
    echo

    build_hub || true
    echo
    build_desktop_agent || true
    echo
    build_decky || true
fi

echo
echo "============================================"
echo "  Build Summary"
echo "============================================"
echo

# Show results
DIST_DIR="$ROOT_DIR/dist"

echo "Output directory: $DIST_DIR"
echo

# Linux
echo -e "${YELLOW}Linux:${NC}"
if [ -f "$DIST_DIR/linux/capydeploy-hub" ]; then
    SIZE=$(stat -c%s "$DIST_DIR/linux/capydeploy-hub" 2>/dev/null || echo "0")
    SIZE_MB=$((SIZE / 1048576))
    echo -e "  ${GREEN}✓${NC} Hub: ${SIZE_MB} MB"
else
    echo -e "  ${RED}✗${NC} Hub: NOT FOUND"
fi

if [ -f "$DIST_DIR/linux/capydeploy-agent" ]; then
    SIZE=$(stat -c%s "$DIST_DIR/linux/capydeploy-agent" 2>/dev/null || echo "0")
    SIZE_MB=$((SIZE / 1048576))
    echo -e "  ${GREEN}✓${NC} Desktop Agent: ${SIZE_MB} MB"
else
    echo -e "  ${RED}✗${NC} Desktop Agent: NOT FOUND"
fi

# Windows
echo -e "${YELLOW}Windows:${NC}"
if [ -f "$DIST_DIR/windows/capydeploy-hub.exe" ]; then
    SIZE=$(stat -c%s "$DIST_DIR/windows/capydeploy-hub.exe" 2>/dev/null || echo "0")
    SIZE_MB=$((SIZE / 1048576))
    echo -e "  ${GREEN}✓${NC} Hub: ${SIZE_MB} MB"
else
    echo -e "  ${RED}✗${NC} Hub: NOT FOUND (install mingw64-gcc)"
fi

if [ -f "$DIST_DIR/windows/capydeploy-agent.exe" ]; then
    SIZE=$(stat -c%s "$DIST_DIR/windows/capydeploy-agent.exe" 2>/dev/null || echo "0")
    SIZE_MB=$((SIZE / 1048576))
    echo -e "  ${GREEN}✓${NC} Desktop Agent: ${SIZE_MB} MB"
else
    echo -e "  ${RED}✗${NC} Desktop Agent: NOT FOUND (install mingw64-gcc)"
fi

# AppImages
echo -e "${YELLOW}AppImages:${NC}"
if [ -f "$DIST_DIR/appimage/CapyDeploy_Hub.AppImage" ]; then
    SIZE=$(stat -c%s "$DIST_DIR/appimage/CapyDeploy_Hub.AppImage" 2>/dev/null || echo "0")
    SIZE_MB=$((SIZE / 1048576))
    echo -e "  ${GREEN}✓${NC} Hub: ${SIZE_MB} MB"
else
    echo -e "  ${RED}✗${NC} Hub: NOT FOUND"
fi

if [ -f "$DIST_DIR/appimage/CapyDeploy_Agent.AppImage" ]; then
    SIZE=$(stat -c%s "$DIST_DIR/appimage/CapyDeploy_Agent.AppImage" 2>/dev/null || echo "0")
    SIZE_MB=$((SIZE / 1048576))
    echo -e "  ${GREEN}✓${NC} Desktop Agent: ${SIZE_MB} MB"
else
    echo -e "  ${RED}✗${NC} Desktop Agent: NOT FOUND"
fi

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
