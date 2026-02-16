package discovery

import (
	"net"
	"os"
	"testing"
)

func TestNewServer(t *testing.T) {
	info := ServiceInfo{
		ID:       "test-agent",
		Name:     "Test Agent",
		Platform: "linux",
		Version:  "1.0.0",
		Port:     8765,
	}

	server := NewServer(info)

	if server == nil {
		t.Fatal("NewServer() returned nil")
	}
	if server.info.ID != info.ID {
		t.Errorf("info.ID = %q, want %q", server.info.ID, info.ID)
	}
}

func TestServer_Info(t *testing.T) {
	info := ServiceInfo{
		ID:       "test-agent",
		Name:     "Test Agent",
		Platform: "linux",
		Version:  "1.0.0",
		Port:     8765,
	}

	server := NewServer(info)

	got := server.Info()
	if got.ID != info.ID {
		t.Errorf("Info().ID = %q, want %q", got.ID, info.ID)
	}
	if got.Name != info.Name {
		t.Errorf("Info().Name = %q, want %q", got.Name, info.Name)
	}
}

func TestServer_Stop_NotStarted(t *testing.T) {
	server := NewServer(ServiceInfo{})

	// Should not error when not started
	if err := server.Stop(); err != nil {
		t.Errorf("Stop() error = %v", err)
	}
}

func TestGetHostname(t *testing.T) {
	hostname := GetHostname()

	if hostname == "" {
		t.Error("GetHostname() should not return empty string")
	}

	// Should match os.Hostname()
	expected, err := os.Hostname()
	if err == nil && hostname != expected {
		t.Errorf("GetHostname() = %q, want %q", hostname, expected)
	}
}

func TestGetPlatform(t *testing.T) {
	platform := GetPlatform()

	if platform == "" {
		t.Error("GetPlatform() should not return empty string")
	}

	// Should be one of known platforms
	knownPlatforms := []string{"steamdeck", "legiongologo", "rogally", "chimeraos", "linux", "windows"}
	found := false
	for _, p := range knownPlatforms {
		if platform == p {
			found = true
			break
		}
	}
	if !found {
		t.Errorf("GetPlatform() = %q, not a known platform", platform)
	}
}

func TestDetectPlatform(t *testing.T) {
	// Just verify it returns something
	platform := detectPlatform()
	if platform == "" {
		t.Error("detectPlatform() should not return empty string")
	}
}

func TestFileExists(t *testing.T) {
	// Create a temp file
	tmpFile, err := os.CreateTemp("", "test")
	if err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}
	defer os.Remove(tmpFile.Name())
	tmpFile.Close()

	if !fileExists(tmpFile.Name()) {
		t.Error("fileExists() should return true for existing file")
	}

	if fileExists("/nonexistent/path/file.txt") {
		t.Error("fileExists() should return false for non-existent file")
	}
}

func TestGetLocalIPs(t *testing.T) {
	ips, err := getLocalIPs()
	if err != nil {
		t.Fatalf("getLocalIPs() error = %v", err)
	}

	// Should have at least one IP on most systems
	// (might be empty in CI containers)
	t.Logf("Found %d local IPs", len(ips))

	// Verify all IPs are IPv4
	for _, ip := range ips {
		if ip.To4() == nil {
			t.Errorf("IP %v is not IPv4", ip)
		}
		if ip.IsLoopback() {
			t.Errorf("IP %v should not be loopback", ip)
		}
	}
}

func TestServer_ZeroPortReturnsError(t *testing.T) {
	info := ServiceInfo{
		ID:   "test",
		Name: "Test",
		Port: 0, // No port specified
	}

	server := NewServer(info)

	// Start() should return an error if port is 0
	err := server.Start()
	if err == nil {
		t.Error("Start() should return error when port is 0")
		server.Stop()
	}
}

func TestServiceInfo_IPs(t *testing.T) {
	ips := []net.IP{
		net.ParseIP("192.168.1.100"),
		net.ParseIP("10.0.0.50"),
	}

	info := ServiceInfo{
		ID:   "test",
		IPs:  ips,
		Port: 8765,
	}

	if len(info.IPs) != 2 {
		t.Errorf("IPs length = %d, want 2", len(info.IPs))
	}
}

// Note: We don't test Start() because it requires network access
// and might fail in CI environments. Integration tests would cover that.
