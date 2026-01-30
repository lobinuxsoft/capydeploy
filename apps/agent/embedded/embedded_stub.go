//go:build !embed_binary

package embedded

// binaryData is empty when building without embed_binary tag.
// For development builds without embedded binaries.
// Production builds should use: go build -tags embed_binary
var binaryData []byte

const binaryName = ""
