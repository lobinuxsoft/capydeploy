package modules

import (
	"net"
	"testing"

	"github.com/lobinuxsoft/capydeploy/pkg/discovery"
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

func TestRegistry(t *testing.T) {
	t.Run("default registry has linux and windows", func(t *testing.T) {
		platforms := DefaultRegistry.Platforms()

		hasLinux := false
		hasWindows := false
		for _, p := range platforms {
			if p == PlatformLinux {
				hasLinux = true
			}
			if p == PlatformWindows {
				hasWindows = true
			}
		}

		if !hasLinux {
			t.Error("registry missing linux module")
		}
		if !hasWindows {
			t.Error("registry missing windows module")
		}
	})

	t.Run("get module by platform", func(t *testing.T) {
		linux := DefaultRegistry.Get(PlatformLinux)
		if linux == nil {
			t.Fatal("linux module is nil")
		}
		if linux.Platform() != PlatformLinux {
			t.Errorf("expected platform %q, got %q", PlatformLinux, linux.Platform())
		}

		windows := DefaultRegistry.Get(PlatformWindows)
		if windows == nil {
			t.Fatal("windows module is nil")
		}
		if windows.Platform() != PlatformWindows {
			t.Errorf("expected platform %q, got %q", PlatformWindows, windows.Platform())
		}
	})

	t.Run("is platform supported", func(t *testing.T) {
		if !IsPlatformSupported(PlatformLinux) {
			t.Error("linux should be supported")
		}
		if !IsPlatformSupported(PlatformWindows) {
			t.Error("windows should be supported")
		}
		if IsPlatformSupported("bsd") {
			t.Error("bsd should not be supported")
		}
	})
}

func TestNormalizePlatform(t *testing.T) {
	tests := []struct {
		input string
		want  string
	}{
		{"linux", "linux"},
		{"windows", "windows"},
		{"steamdeck", PlatformLinux},
		{"steamos", PlatformLinux},
		{"bazzite", PlatformLinux},
		{"chimera", PlatformLinux},
		{"unknown", "unknown"},
	}

	for _, tt := range tests {
		t.Run(tt.input, func(t *testing.T) {
			got := normalizePlatform(tt.input)
			if got != tt.want {
				t.Errorf("normalizePlatform(%q) = %q, want %q", tt.input, got, tt.want)
			}
		})
	}
}

func TestGetSupportedImageFormats(t *testing.T) {
	t.Run("linux formats", func(t *testing.T) {
		formats := GetSupportedImageFormats(PlatformLinux)
		if len(formats) < 3 {
			t.Errorf("expected at least 3 formats for linux, got %d", len(formats))
		}
	})

	t.Run("windows formats", func(t *testing.T) {
		formats := GetSupportedImageFormats(PlatformWindows)
		if len(formats) < 2 {
			t.Errorf("expected at least 2 formats for windows, got %d", len(formats))
		}
	})

	t.Run("steamdeck normalizes to linux", func(t *testing.T) {
		formats := GetSupportedImageFormats("steamdeck")
		linuxFormats := GetSupportedImageFormats(PlatformLinux)
		if len(formats) != len(linuxFormats) {
			t.Errorf("steamdeck formats (%d) should match linux formats (%d)", len(formats), len(linuxFormats))
		}
	})

	t.Run("unknown platform returns fallback", func(t *testing.T) {
		formats := GetSupportedImageFormats("unknown")
		if len(formats) == 0 {
			t.Error("should return fallback formats for unknown platform")
		}
	})
}

func TestTypeAssertions(t *testing.T) {
	// WSClient implements all interfaces — verify assertions work
	client := NewWSClient("localhost", 8765, PlatformLinux, "test-hub", "1.0.0")

	t.Run("client implements ShortcutManager", func(t *testing.T) {
		sm, ok := AsShortcutManager(client)
		if !ok {
			t.Error("WSClient should implement ShortcutManager")
		}
		if sm == nil {
			t.Error("ShortcutManager is nil")
		}
	})

	t.Run("client implements ArtworkManager", func(t *testing.T) {
		am, ok := AsArtworkManager(client)
		if !ok {
			t.Error("WSClient should implement ArtworkManager")
		}
		if am == nil {
			t.Error("ArtworkManager is nil")
		}
	})

	t.Run("client implements SteamController", func(t *testing.T) {
		sc, ok := AsSteamController(client)
		if !ok {
			t.Error("WSClient should implement SteamController")
		}
		if sc == nil {
			t.Error("SteamController is nil")
		}
	})

	t.Run("client implements FileUploader", func(t *testing.T) {
		fu, ok := AsFileUploader(client)
		if !ok {
			t.Error("WSClient should implement FileUploader")
		}
		if fu == nil {
			t.Error("FileUploader is nil")
		}
	})

	t.Run("client implements SteamUserProvider", func(t *testing.T) {
		sup, ok := AsSteamUserProvider(client)
		if !ok {
			t.Error("WSClient should implement SteamUserProvider")
		}
		if sup == nil {
			t.Error("SteamUserProvider is nil")
		}
	})

	t.Run("client implements FullPlatformClient", func(t *testing.T) {
		fc, ok := AsFullClient(client)
		if !ok {
			t.Error("WSClient should implement FullPlatformClient")
		}
		if fc == nil {
			t.Error("FullPlatformClient is nil")
		}
	})

	t.Run("GetCapabilities returns all true", func(t *testing.T) {
		caps := GetCapabilities(client)
		if !caps.Shortcuts {
			t.Error("Shortcuts should be true")
		}
		if !caps.Artwork {
			t.Error("Artwork should be true")
		}
		if !caps.Steam {
			t.Error("Steam should be true")
		}
		if !caps.Upload {
			t.Error("Upload should be true")
		}
		if !caps.SteamUsers {
			t.Error("SteamUsers should be true")
		}
	})
}

func TestWSClientFromAgent(t *testing.T) {
	t.Run("nil agent returns error", func(t *testing.T) {
		_, err := WSClientFromAgent(nil, "hub", "1.0")
		if err == nil {
			t.Error("expected error for nil agent")
		}
	})

	t.Run("agent without platform returns error", func(t *testing.T) {
		agent := &discovery.DiscoveredAgent{
			Info: protocol.AgentInfo{},
			IPs:  []net.IP{net.ParseIP("192.168.1.100")},
			Port: 8765,
		}
		_, err := WSClientFromAgent(agent, "hub", "1.0")
		if err == nil {
			t.Error("expected error for agent without platform")
		}
	})

	t.Run("agent without address returns error", func(t *testing.T) {
		agent := &discovery.DiscoveredAgent{
			Info: protocol.AgentInfo{Platform: PlatformLinux},
			IPs:  nil,
			Host: "",
			Port: 8765,
		}
		_, err := WSClientFromAgent(agent, "hub", "1.0")
		if err == nil {
			t.Error("expected error for agent without address")
		}
	})

	t.Run("valid agent creates client", func(t *testing.T) {
		agent := &discovery.DiscoveredAgent{
			Info: protocol.AgentInfo{
				ID:       "test-agent",
				Name:     "Test Agent",
				Platform: PlatformLinux,
				Version:  "1.0.0",
			},
			IPs:  []net.IP{net.ParseIP("192.168.1.100")},
			Port: 8765,
		}
		client, err := WSClientFromAgent(agent, "hub", "1.0")
		if err != nil {
			t.Fatalf("failed to create client: %v", err)
		}
		if client == nil {
			t.Fatal("client is nil")
		}
	})

	t.Run("agent with host fallback", func(t *testing.T) {
		agent := &discovery.DiscoveredAgent{
			Info: protocol.AgentInfo{
				ID:       "test-agent",
				Platform: PlatformWindows,
			},
			Host: "windows-pc.local",
			Port: 8765,
		}
		client, err := WSClientFromAgent(agent, "hub", "1.0")
		if err != nil {
			t.Fatalf("failed to create client: %v", err)
		}
		if client == nil {
			t.Fatal("client is nil")
		}
	})
}

func TestCustomRegistry(t *testing.T) {
	registry := NewRegistry()

	if !registry.IsSupported(PlatformLinux) {
		t.Error("custom registry should support linux")
	}
	if !registry.IsSupported(PlatformWindows) {
		t.Error("custom registry should support windows")
	}
}
