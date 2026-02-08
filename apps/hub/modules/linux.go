package modules

const (
	// PlatformLinux is the identifier for Linux-based systems (SteamOS, Bazzite, etc.).
	PlatformLinux = "linux"
)

// LinuxModule handles communication with Linux-based Agents.
// This includes SteamOS, Bazzite, and other Linux distributions.
type LinuxModule struct{}

// NewLinuxModule creates a new Linux platform module.
func NewLinuxModule() *LinuxModule {
	return &LinuxModule{}
}

// Platform returns the platform identifier.
func (m *LinuxModule) Platform() string {
	return PlatformLinux
}

// SupportedImageFormats returns the image formats supported by Linux Steam.
// Linux Steam supports PNG, JPEG, WebP, and animated GIF.
func (m *LinuxModule) SupportedImageFormats() []string {
	return []string{"image/png", "image/jpeg", "image/webp", "image/gif"}
}

// Ensure LinuxModule implements PlatformModule.
var _ PlatformModule = (*LinuxModule)(nil)
