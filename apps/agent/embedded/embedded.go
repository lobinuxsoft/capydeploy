// Package embedded provides embedded binaries for the agent.
package embedded

// BinaryName returns the platform-specific binary name.
func BinaryName() string {
	return binaryName
}

// Binary returns the embedded binary data.
func Binary() []byte {
	return binaryData
}

// HasBinary returns true if a binary is embedded.
func HasBinary() bool {
	return len(binaryData) > 0
}
