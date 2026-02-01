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

// NewClient creates a new client for communicating with a Linux Agent.
func (m *LinuxModule) NewClient(host string, port int) PlatformClient {
	return newBaseClient(host, port, PlatformLinux)
}

// Ensure LinuxModule implements PlatformModule.
var _ PlatformModule = (*LinuxModule)(nil)
