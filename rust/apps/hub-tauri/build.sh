#!/usr/bin/env bash
# Build script for CapyDeploy Hub (Tauri v2)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Building CapyDeploy Hub (Tauri) ==="

# Build frontend
echo "--- Building frontend ---"
cd frontend
bun install --frozen-lockfile
bun run build
cd ..

# Build Tauri app
echo "--- Building Tauri app ---"
cd src-tauri
cargo build --release
cd ..

echo "=== Build complete ==="
echo "Binary: src-tauri/target/release/capydeploy-hub-tauri"
