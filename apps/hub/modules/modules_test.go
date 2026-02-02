package modules

import (
	"context"
	"encoding/json"
	"net"
	"net/http"
	"net/http/httptest"
	"strconv"
	"strings"
	"testing"
	"time"

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

	t.Run("unsupported platform returns error", func(t *testing.T) {
		_, err := DefaultRegistry.GetClient("unsupported", "localhost", 8765)
		if err == nil {
			t.Error("expected error for unsupported platform")
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

func TestClientCreation(t *testing.T) {
	t.Run("create linux client", func(t *testing.T) {
		client, err := GetClientForPlatform(PlatformLinux, "192.168.1.100", 8765)
		if err != nil {
			t.Fatalf("failed to create client: %v", err)
		}
		if client == nil {
			t.Fatal("client is nil")
		}
	})

	t.Run("create windows client", func(t *testing.T) {
		client, err := GetClientForPlatform(PlatformWindows, "192.168.1.100", 8765)
		if err != nil {
			t.Fatalf("failed to create client: %v", err)
		}
		if client == nil {
			t.Fatal("client is nil")
		}
	})
}

func TestTypeAssertions(t *testing.T) {
	client, _ := GetClientForPlatform(PlatformLinux, "localhost", 8765)

	t.Run("client implements ShortcutManager", func(t *testing.T) {
		sm, ok := AsShortcutManager(client)
		if !ok {
			t.Error("client should implement ShortcutManager")
		}
		if sm == nil {
			t.Error("ShortcutManager is nil")
		}
	})

	t.Run("client implements ArtworkManager", func(t *testing.T) {
		am, ok := AsArtworkManager(client)
		if !ok {
			t.Error("client should implement ArtworkManager")
		}
		if am == nil {
			t.Error("ArtworkManager is nil")
		}
	})

	t.Run("client implements SteamController", func(t *testing.T) {
		sc, ok := AsSteamController(client)
		if !ok {
			t.Error("client should implement SteamController")
		}
		if sc == nil {
			t.Error("SteamController is nil")
		}
	})

	t.Run("client implements FileUploader", func(t *testing.T) {
		fu, ok := AsFileUploader(client)
		if !ok {
			t.Error("client should implement FileUploader")
		}
		if fu == nil {
			t.Error("FileUploader is nil")
		}
	})

	t.Run("client implements SteamUserProvider", func(t *testing.T) {
		sup, ok := AsSteamUserProvider(client)
		if !ok {
			t.Error("client should implement SteamUserProvider")
		}
		if sup == nil {
			t.Error("SteamUserProvider is nil")
		}
	})

	t.Run("client implements FullPlatformClient", func(t *testing.T) {
		fc, ok := AsFullClient(client)
		if !ok {
			t.Error("client should implement FullPlatformClient")
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

func TestClientFromAgent(t *testing.T) {
	t.Run("nil agent returns error", func(t *testing.T) {
		_, err := ClientFromAgent(nil)
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
		_, err := ClientFromAgent(agent)
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
		_, err := ClientFromAgent(agent)
		if err == nil {
			t.Error("expected error for agent without address")
		}
	})

	t.Run("valid linux agent creates client", func(t *testing.T) {
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
		client, err := ClientFromAgent(agent)
		if err != nil {
			t.Fatalf("failed to create client: %v", err)
		}
		if client == nil {
			t.Fatal("client is nil")
		}
	})

	t.Run("valid windows agent creates client", func(t *testing.T) {
		agent := &discovery.DiscoveredAgent{
			Info: protocol.AgentInfo{
				ID:       "test-agent",
				Name:     "Test Agent",
				Platform: PlatformWindows,
				Version:  "1.0.0",
			},
			Host: "windows-pc.local",
			Port: 8765,
		}
		client, err := ClientFromAgent(agent)
		if err != nil {
			t.Fatalf("failed to create client: %v", err)
		}
		if client == nil {
			t.Fatal("client is nil")
		}
	})
}

// Integration test with mock server
func TestClientWithMockServer(t *testing.T) {
	// Create mock server
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch r.URL.Path {
		case "/health":
			w.WriteHeader(http.StatusOK)
			json.NewEncoder(w).Encode(map[string]string{"status": "ok"})
		case "/info":
			json.NewEncoder(w).Encode(protocol.AgentInfo{
				ID:       "mock-agent",
				Name:     "Mock Agent",
				Platform: PlatformLinux,
				Version:  "1.0.0",
			})
		case "/steam/users":
			json.NewEncoder(w).Encode(map[string]interface{}{
				"users": []map[string]interface{}{
					{"id": "123456", "name": "TestUser"},
				},
			})
		default:
			w.WriteHeader(http.StatusNotFound)
		}
	}))
	defer server.Close()

	// Parse server address
	addr := server.Listener.Addr().String()
	parts := strings.Split(addr, ":")
	host := parts[0]
	port, _ := strconv.Atoi(parts[1])

	// Create client
	client, err := GetClientForPlatform(PlatformLinux, host, port)
	if err != nil {
		t.Fatalf("failed to create client: %v", err)
	}

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	t.Run("health check", func(t *testing.T) {
		err := client.Health(ctx)
		if err != nil {
			t.Errorf("health check failed: %v", err)
		}
	})

	t.Run("get info", func(t *testing.T) {
		info, err := client.GetInfo(ctx)
		if err != nil {
			t.Fatalf("get info failed: %v", err)
		}
		if info.ID != "mock-agent" {
			t.Errorf("expected ID %q, got %q", "mock-agent", info.ID)
		}
		if info.Platform != PlatformLinux {
			t.Errorf("expected platform %q, got %q", PlatformLinux, info.Platform)
		}
	})

	t.Run("get steam users via type assertion", func(t *testing.T) {
		sup, ok := AsSteamUserProvider(client)
		if !ok {
			t.Fatal("client should implement SteamUserProvider")
		}
		users, err := sup.GetSteamUsers(ctx)
		if err != nil {
			t.Fatalf("get steam users failed: %v", err)
		}
		if len(users) != 1 {
			t.Errorf("expected 1 user, got %d", len(users))
		}
	})
}

func TestCustomRegistry(t *testing.T) {
	// Create custom registry
	registry := NewRegistry()

	// Verify it has default modules
	if !registry.IsSupported(PlatformLinux) {
		t.Error("custom registry should support linux")
	}
	if !registry.IsSupported(PlatformWindows) {
		t.Error("custom registry should support windows")
	}

	// Create client from custom registry
	client, err := registry.GetClient(PlatformLinux, "localhost", 8765)
	if err != nil {
		t.Fatalf("failed to create client: %v", err)
	}
	if client == nil {
		t.Fatal("client is nil")
	}
}
