#!/bin/bash
set -e

cd "$(dirname "$0")"

# Create build directory if it doesn't exist
mkdir -p build/linux

# Build Linux version
echo "Building Linux binary..."
CGO_ENABLED=1 GOOS=linux GOARCH=amd64 go build -o build/linux/bazzite-devkit ./cmd/bazzite-devkit

echo ""
echo "Build successful!"
echo "  build/linux/bazzite-devkit"
