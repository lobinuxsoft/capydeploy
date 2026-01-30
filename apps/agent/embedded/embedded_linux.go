//go:build linux && embed_binary

package embedded

import (
	_ "embed"
)

// binaryData is the embedded Linux binary for steam-shortcut-manager.
// Build with: go build -tags embed_binary
// Requires steam-shortcut-manager binary in this directory.
//
//go:embed steam-shortcut-manager
var binaryData []byte

const binaryName = "steam-shortcut-manager"
