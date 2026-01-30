//go:build windows && embed_binary

package embedded

import (
	_ "embed"
)

// binaryData is the embedded Windows binary for steam-shortcut-manager.
// Build with: go build -tags embed_binary
// Requires steam-shortcut-manager.exe binary in this directory.
//
//go:embed steam-shortcut-manager.exe
var binaryData []byte

const binaryName = "steam-shortcut-manager.exe"
