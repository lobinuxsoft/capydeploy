package modules

const (
	// PlatformWindows is the identifier for Windows-based systems.
	PlatformWindows = "windows"
)

// WindowsModule handles communication with Windows-based Agents.
type WindowsModule struct{}

// NewWindowsModule creates a new Windows platform module.
func NewWindowsModule() *WindowsModule {
	return &WindowsModule{}
}

// Platform returns the platform identifier.
func (m *WindowsModule) Platform() string {
	return PlatformWindows
}

// SupportedImageFormats returns the image formats supported by Windows Steam.
func (m *WindowsModule) SupportedImageFormats() []string {
	return []string{"image/png", "image/jpeg"}
}

// Ensure WindowsModule implements PlatformModule.
var _ PlatformModule = (*WindowsModule)(nil)
