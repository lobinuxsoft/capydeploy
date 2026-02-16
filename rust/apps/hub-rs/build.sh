#!/bin/bash

set -e

echo "============================================"
echo "  CapyDeploy Hub (Rust) - Build Script"
echo "============================================"
echo

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RUST_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
PROJECT_ROOT="$(cd "$RUST_ROOT/.." && pwd)"
DIST_DIR="$PROJECT_ROOT/dist"

MODE="release"

while [[ $# -gt 0 ]]; do
    case $1 in
        dev|--dev|-d)
            MODE="dev"
            shift
            ;;
        run|--run|-r)
            MODE="run"
            shift
            ;;
        --help|-h)
            echo "Usage: ./build.sh [options]"
            echo
            echo "Options:"
            echo "  dev, --dev, -d    Build in debug mode (faster compile)"
            echo "  run, --run, -r    Build release and run immediately"
            echo "  --help, -h        Show this help message"
            echo
            echo "Examples:"
            echo "  ./build.sh              Build release binary"
            echo "  ./build.sh dev          Build debug binary"
            echo "  ./build.sh run          Build release and run"
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

# Check Rust toolchain.
echo -e "${YELLOW}[1/3]${NC} Checking toolchain..."

if ! command -v cargo &> /dev/null; then
    echo -e "${RED}[ERROR]${NC} cargo not found."
    echo "Install Rust from: https://rustup.rs/"
    exit 1
fi
RUST_VERSION=$(rustc --version | awk '{print $2}')
echo -e "  Rust: ${GREEN}${RUST_VERSION}${NC}"
echo

# Lint.
echo -e "${YELLOW}[2/3]${NC} Running clippy..."
cargo clippy -p capydeploy-hub --manifest-path "$RUST_ROOT/Cargo.toml" -- -D warnings
echo -e "  ${GREEN}Clippy clean${NC}"
echo

# Build.
if [ "$MODE" = "dev" ]; then
    echo -e "${YELLOW}[3/3]${NC} Building debug binary..."
    cargo build -p capydeploy-hub --manifest-path "$RUST_ROOT/Cargo.toml"
    BINARY="$RUST_ROOT/target/debug/capydeploy-hub"
else
    echo -e "${YELLOW}[3/3]${NC} Building release binary..."
    cargo build -p capydeploy-hub --manifest-path "$RUST_ROOT/Cargo.toml" --release
    BINARY="$RUST_ROOT/target/release/capydeploy-hub"
fi

echo
echo "============================================"
echo -e "  ${GREEN}BUILD SUCCESSFUL${NC}"
echo "============================================"
echo

SIZE=$(stat -c%s "$BINARY" 2>/dev/null || stat -f%z "$BINARY" 2>/dev/null || echo "0")
SIZE_MB=$((SIZE / 1048576))
echo "  Binary: $BINARY (${SIZE_MB} MB)"
echo

if [ "$MODE" = "run" ]; then
    echo "  Launching..."
    echo
    exec "$BINARY"
fi
