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

// NewClient creates a new client for communicating with a Windows Agent.
func (m *WindowsModule) NewClient(host string, port int) PlatformClient {
	return newBaseClient(host, port, PlatformWindows)
}

// SupportedImageFormats returns the image formats supported by Windows Steam.
// Windows Steam only supports PNG and JPEG for shortcut artwork.
func (m *WindowsModule) SupportedImageFormats() []string {
	return []string{"image/png", "image/jpeg"}
}

// Ensure WindowsModule implements PlatformModule.
var _ PlatformModule = (*WindowsModule)(nil)
