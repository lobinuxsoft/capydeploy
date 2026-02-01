package main

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/lobinuxsoft/capydeploy/apps/hub/modules"
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

// mockAgentServer creates a mock Agent HTTP server for testing
func mockAgentServer() *httptest.Server {
	mux := http.NewServeMux()

	// Health endpoint
	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		json.NewEncoder(w).Encode(map[string]string{"status": "ok"})
	})

	// Info endpoint
	mux.HandleFunc("/info", func(w http.ResponseWriter, r *http.Request) {
		json.NewEncoder(w).Encode(protocol.AgentInfo{
			ID:           "test-agent-001",
			Name:         "Test Agent",
			Platform:     "linux",
			Version:      "1.0.0",
			SteamRunning: true,
		})
	})

	// Steam users endpoint
	mux.HandleFunc("/steam/users", func(w http.ResponseWriter, r *http.Request) {
		json.NewEncoder(w).Encode(map[string]interface{}{
			"users": []map[string]interface{}{
				{"id": "76561198012345678", "name": "TestUser"},
			},
		})
	})

	// Shortcuts endpoints
	mux.HandleFunc("/shortcuts/", func(w http.ResponseWriter, r *http.Request) {
		parts := strings.Split(r.URL.Path, "/")

		switch r.Method {
		case "GET":
			// List shortcuts
			json.NewEncoder(w).Encode(map[string]interface{}{
				"shortcuts": []protocol.ShortcutInfo{
					{AppID: 123456, Name: "Test Game", Exe: "/games/test/game.exe", StartDir: "/games/test"},
				},
			})
		case "POST":
			// Create shortcut
			json.NewEncoder(w).Encode(map[string]interface{}{
				"appId": 123456,
			})
		case "DELETE":
			// Delete shortcut
			w.WriteHeader(http.StatusOK)
		default:
			w.WriteHeader(http.StatusMethodNotAllowed)
		}
		_ = parts // avoid unused variable
	})

	// Steam restart endpoint
	mux.HandleFunc("/steam/restart", func(w http.ResponseWriter, r *http.Request) {
		json.NewEncoder(w).Encode(map[string]interface{}{
			"success": true,
			"message": "Steam restarted",
		})
	})

	// Upload endpoints
	mux.HandleFunc("/uploads", func(w http.ResponseWriter, r *http.Request) {
		if r.Method == "POST" {
			json.NewEncoder(w).Encode(map[string]interface{}{
				"uploadId":   "upload-123",
				"chunkSize":  1048576,
				"resumeFrom": map[string]int64{},
			})
		}
	})

	mux.HandleFunc("/uploads/", func(w http.ResponseWriter, r *http.Request) {
		if strings.HasSuffix(r.URL.Path, "/chunks") {
			w.WriteHeader(http.StatusOK)
		} else if strings.HasSuffix(r.URL.Path, "/complete") {
			json.NewEncoder(w).Encode(map[string]interface{}{
				"success": true,
				"path":    "/games/test",
				"appId":   123456,
			})
		} else {
			// Get upload status
			json.NewEncoder(w).Encode(map[string]interface{}{
				"progress": map[string]interface{}{
					"uploadId":         "upload-123",
					"status":           "in_progress",
					"totalBytes":       1000000,
					"transferredBytes": 500000,
				},
			})
		}
	})

	return httptest.NewServer(mux)
}

func TestModulesWithMockAgent(t *testing.T) {
	server := mockAgentServer()
	defer server.Close()

	// Parse server address
	addr := server.Listener.Addr().String()
	parts := strings.Split(addr, ":")
	host := parts[0]
	port, _ := strconv.Atoi(parts[1])

	// Create client using modules
	client, err := modules.GetClientForPlatform("linux", host, port)
	if err != nil {
		t.Fatalf("Failed to create client: %v", err)
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	t.Run("health check", func(t *testing.T) {
		err := client.Health(ctx)
		if err != nil {
			t.Errorf("Health check failed: %v", err)
		}
	})

	t.Run("get info", func(t *testing.T) {
		info, err := client.GetInfo(ctx)
		if err != nil {
			t.Fatalf("GetInfo failed: %v", err)
		}
		if info.Platform != "linux" {
			t.Errorf("Expected platform linux, got %s", info.Platform)
		}
		if info.Name != "Test Agent" {
			t.Errorf("Expected name 'Test Agent', got %s", info.Name)
		}
	})

	t.Run("get steam users", func(t *testing.T) {
		userProvider, ok := modules.AsSteamUserProvider(client)
		if !ok {
			t.Fatal("Client should implement SteamUserProvider")
		}
		users, err := userProvider.GetSteamUsers(ctx)
		if err != nil {
			t.Fatalf("GetSteamUsers failed: %v", err)
		}
		if len(users) != 1 {
			t.Errorf("Expected 1 user, got %d", len(users))
		}
	})

	t.Run("list shortcuts", func(t *testing.T) {
		shortcutMgr, ok := modules.AsShortcutManager(client)
		if !ok {
			t.Fatal("Client should implement ShortcutManager")
		}
		shortcuts, err := shortcutMgr.ListShortcuts(ctx, "76561198012345678")
		if err != nil {
			t.Fatalf("ListShortcuts failed: %v", err)
		}
		if len(shortcuts) != 1 {
			t.Errorf("Expected 1 shortcut, got %d", len(shortcuts))
		}
		if shortcuts[0].Name != "Test Game" {
			t.Errorf("Expected shortcut name 'Test Game', got %s", shortcuts[0].Name)
		}
	})

	t.Run("create shortcut", func(t *testing.T) {
		shortcutMgr, ok := modules.AsShortcutManager(client)
		if !ok {
			t.Fatal("Client should implement ShortcutManager")
		}
		appID, err := shortcutMgr.CreateShortcut(ctx, "76561198012345678", protocol.ShortcutConfig{
			Name:     "New Game",
			Exe:      "/games/new/game.exe",
			StartDir: "/games/new",
		})
		if err != nil {
			t.Fatalf("CreateShortcut failed: %v", err)
		}
		if appID != 123456 {
			t.Errorf("Expected appID 123456, got %d", appID)
		}
	})

	t.Run("restart steam", func(t *testing.T) {
		steamCtrl, ok := modules.AsSteamController(client)
		if !ok {
			t.Fatal("Client should implement SteamController")
		}
		result, err := steamCtrl.RestartSteam(ctx)
		if err != nil {
			t.Fatalf("RestartSteam failed: %v", err)
		}
		if !result.Success {
			t.Error("RestartSteam should return success")
		}
	})
}

func TestAppNewApp(t *testing.T) {
	app := NewApp()
	if app == nil {
		t.Fatal("NewApp returned nil")
	}
	if app.discoveryClient == nil {
		t.Error("discoveryClient should be initialized")
	}
	if app.discoveredCache == nil {
		t.Error("discoveredCache should be initialized")
	}
}

func TestConnectionStatus(t *testing.T) {
	app := NewApp()

	status := app.GetConnectionStatus()
	if status.Connected {
		t.Error("Should not be connected initially")
	}
	if status.AgentID != "" {
		t.Error("AgentID should be empty when not connected")
	}
}

func TestGetDiscoveredAgents(t *testing.T) {
	app := NewApp()

	agents := app.GetDiscoveredAgents()
	if agents == nil {
		t.Error("GetDiscoveredAgents should return empty slice, not nil")
	}
	if len(agents) != 0 {
		t.Error("Should have no agents initially")
	}
}
