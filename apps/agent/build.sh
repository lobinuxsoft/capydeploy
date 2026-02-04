#!/bin/bash

set -e

echo "============================================"
echo "  CapyDeploy Agent - Build Script"
echo "============================================"
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Default values
MODE="production"
SKIP_DEPS=0

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

echo -e "${YELLOW}[1/4]${NC} Checking required tools..."
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
    echo -e "${YELLOW}[2/4]${NC} Installing frontend dependencies..."
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
    echo -e "${YELLOW}[2/4]${NC} Skipping frontend dependencies (--skip-deps)"
    echo
fi

# ============================================
# Version info (from git)
# ============================================

echo -e "${YELLOW}[3/4]${NC} Collecting version info..."
echo

VERSION=$(git describe --tags --always --dirty 2>/dev/null || echo "dev")
COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

echo "  Version:    $VERSION"
echo "  Commit:     $COMMIT"
echo "  Build Date: $BUILD_DATE"
echo

LDFLAGS="-X github.com/lobinuxsoft/capydeploy/pkg/version.Version=$VERSION"
LDFLAGS="$LDFLAGS -X github.com/lobinuxsoft/capydeploy/pkg/version.Commit=$COMMIT"
LDFLAGS="$LDFLAGS -X github.com/lobinuxsoft/capydeploy/pkg/version.BuildDate=$BUILD_DATE"

# ============================================
# Build
# ============================================

if [ "$MODE" = "dev" ]; then
    echo -e "${YELLOW}[4/5]${NC} Starting development server..."
    echo
    echo "  Press Ctrl+C to stop."
    echo
    wails dev -tags webkit2_41 -ldflags "$LDFLAGS"
else
    echo -e "${YELLOW}[4/5]${NC} Building production binary..."
    echo

    if ! wails build -clean -tags webkit2_41 -ldflags "$LDFLAGS"; then
        echo
        echo "============================================"
        echo -e "  ${RED}BUILD FAILED${NC}"
        echo "============================================"
        exit 1
    fi

    echo
    echo "============================================"
    echo -e "  ${GREEN}BUILD SUCCESSFUL${NC}"
    echo "============================================"
    echo

    # Show result
    echo -e "${YELLOW}[5/5]${NC} Build output:"
    echo

    BINARY="build/bin/capydeploy-agent"
    if [ -f "$BINARY" ]; then
        SIZE=$(stat -f%z "$BINARY" 2>/dev/null || stat -c%s "$BINARY" 2>/dev/null || echo "0")
        SIZE_KB=$((SIZE / 1024))
        SIZE_MB=$((SIZE / 1048576))
        echo "  File: $BINARY"
        echo "  Size: ${SIZE_MB} MB (${SIZE_KB} KB)"
    elif [ -d "build/bin" ]; then
        echo "  Output directory: build/bin/"
        ls -lh build/bin/
    fi

    echo
    echo "Done! Run with: ./build/bin/capydeploy-agent"
fi
