#!/bin/bash
# Build script for CapyDeploy Agent

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

VERSION="${VERSION:-dev}"
OUTPUT_DIR="${OUTPUT_DIR:-./build}"
EMBED_BINARY="${EMBED_BINARY:-false}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  -o, --output DIR     Output directory (default: ./build)"
    echo "  -v, --version VER    Version string (default: dev)"
    echo "  -e, --embed          Embed steam-shortcut-manager binary"
    echo "  -p, --platform OS    Target platform: linux, windows (default: current)"
    echo "  -h, --help           Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                           # Build for current platform"
    echo "  $0 -p linux -e               # Build for Linux with embedded binary"
    echo "  $0 -p windows -v 1.0.0       # Build for Windows with version"
}

# Parse arguments
PLATFORM=""
while [[ $# -gt 0 ]]; do
    case $1 in
        -o|--output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        -v|--version)
            VERSION="$2"
            shift 2
            ;;
        -e|--embed)
            EMBED_BINARY="true"
            shift
            ;;
        -p|--platform)
            PLATFORM="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Determine build settings
BUILD_TAGS=""
if [[ "$EMBED_BINARY" == "true" ]]; then
    BUILD_TAGS="-tags embed_binary"
    print_info "Building with embedded binary"

    # Check if binary exists
    if [[ "$PLATFORM" == "windows" ]] || [[ -z "$PLATFORM" && "$OSTYPE" == "msys"* ]]; then
        if [[ ! -f "embedded/steam-shortcut-manager.exe" ]]; then
            print_error "embedded/steam-shortcut-manager.exe not found"
            print_warn "Build steam-shortcut-manager for Windows first"
            exit 1
        fi
    else
        if [[ ! -f "embedded/steam-shortcut-manager" ]]; then
            print_error "embedded/steam-shortcut-manager not found"
            print_warn "Build steam-shortcut-manager for Linux first"
            exit 1
        fi
    fi
fi

LDFLAGS="-X main.Version=$VERSION"

build_for_platform() {
    local goos=$1
    local goarch=$2
    local output_name=$3

    print_info "Building for $goos/$goarch..."

    GOOS=$goos GOARCH=$goarch go build \
        $BUILD_TAGS \
        -ldflags "$LDFLAGS" \
        -o "$OUTPUT_DIR/$output_name" \
        .

    print_info "Built: $OUTPUT_DIR/$output_name"
}

# Build based on platform argument or current OS
if [[ -n "$PLATFORM" ]]; then
    case $PLATFORM in
        linux)
            build_for_platform "linux" "amd64" "capydeploy-agent"
            ;;
        windows)
            build_for_platform "windows" "amd64" "capydeploy-agent.exe"
            ;;
        *)
            print_error "Unknown platform: $PLATFORM"
            print_warn "Supported platforms: linux, windows"
            exit 1
            ;;
    esac
else
    # Build for current platform
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        build_for_platform "linux" "amd64" "capydeploy-agent"
    elif [[ "$OSTYPE" == "msys"* ]] || [[ "$OSTYPE" == "cygwin"* ]] || [[ "$OSTYPE" == "win32"* ]]; then
        build_for_platform "windows" "amd64" "capydeploy-agent.exe"
    else
        print_warn "Unknown OS type: $OSTYPE, attempting native build"
        go build $BUILD_TAGS -ldflags "$LDFLAGS" -o "$OUTPUT_DIR/capydeploy-agent" .
    fi
fi

print_info "Build complete!"
