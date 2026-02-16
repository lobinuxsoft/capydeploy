#!/usr/bin/env bash
# Build script for CapyDeploy Agent (Tauri v2)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Building CapyDeploy Agent (Tauri) ==="

# Build frontend
echo "--- Building frontend ---"
cd frontend
bun install --frozen-lockfile
bun run build
cd ..

# Build Tauri app
echo "--- Building Tauri app ---"
cargo build --release -p capydeploy-agent-tauri --manifest-path "$SCRIPT_DIR/../../../Cargo.toml"

BINARY="$SCRIPT_DIR/../../../target/release/capydeploy-agent-tauri"
echo "=== Build complete ==="
echo "Binary: $BINARY"
