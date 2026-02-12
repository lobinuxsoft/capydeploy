package protocol_test

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/steam"
)

// TestGenerateFixtures generates golden JSON fixtures for wire compatibility testing.
// Run with: GENERATE_FIXTURES=1 go test ./pkg/protocol/ -run TestGenerateFixtures -v
func TestGenerateFixtures(t *testing.T) {
	if os.Getenv("GENERATE_FIXTURES") != "1" {
		t.Skip("Set GENERATE_FIXTURES=1 to generate wire compatibility fixtures")
	}

	dir := filepath.Join("..", "..", "rust", "tests", "wire_compat", "fixtures")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatalf("failed to create fixtures dir: %v", err)
	}

	writeFixture := func(name string, v any) {
		t.Helper()
		data, err := json.MarshalIndent(v, "", "  ")
		if err != nil {
			t.Fatalf("failed to marshal %s: %v", name, err)
		}
		path := filepath.Join(dir, name)
		if err := os.WriteFile(path, data, 0o644); err != nil {
			t.Fatalf("failed to write %s: %v", path, err)
		}
		t.Logf("wrote %s (%d bytes)", path, len(data))
	}

	// Message envelope
	msg, err := protocol.NewMessage("test-msg-1", protocol.MsgTypeGetInfo, map[string]string{"key": "value"})
	if err != nil {
		t.Fatal(err)
	}
	writeFixture("message_envelope.json", msg)

	// AgentInfo
	writeFixture("agent_info.json", protocol.AgentInfo{
		ID:                    "agent-1",
		Name:                  "Test Agent",
		Platform:              "steamdeck",
		Version:               "0.6.0",
		AcceptConnections:     true,
		SupportedImageFormats: []string{"png", "jpg"},
	})

	// HubConnectedRequest
	writeFixture("hub_connected_request.json", protocol.HubConnectedRequest{
		Name:     "CapyDeploy Hub",
		Version:  "0.6.0",
		Platform: "windows",
		HubID:    "hub-123",
		Token:    "auth-token-abc",
	})

	// AgentStatusResponse
	writeFixture("agent_status_response.json", protocol.AgentStatusResponse{
		Name:              "Agent",
		Version:           "0.6.0",
		Platform:          "steamdeck",
		AcceptConnections: true,
		TelemetryEnabled:  true,
		TelemetryInterval: 5,
		ConsoleLogEnabled: false,
	})

	// InitUploadRequest
	writeFixture("init_upload_request.json", protocol.InitUploadRequest{
		Config: protocol.UploadConfig{
			GameName:      "Test Game",
			InstallPath:   "/games/test",
			Executable:    "game.exe",
			LaunchOptions: "--windowed",
			Tags:          "indie,rpg",
		},
		TotalSize: 1073741824,
		FileCount: 42,
	})

	// UploadChunkRequest (with base64 []byte data)
	writeFixture("upload_chunk_request.json", protocol.UploadChunkRequest{
		UploadID: "upload-001",
		Offset:   0,
		Data:     []byte("Hello, World!"),
		FilePath: "data/level1.bin",
		IsLast:   false,
	})

	// TelemetryData
	writeFixture("telemetry_data.json", protocol.TelemetryData{
		Timestamp: 1700000000,
		CPU: &protocol.CPUMetrics{
			UsagePercent: 45.5,
			TempCelsius:  65.0,
			FreqMHz:      3200.0,
		},
		GPU: &protocol.GPUMetrics{
			UsagePercent:   80.0,
			TempCelsius:    75.0,
			FreqMHz:        1800.0,
			MemFreqMHz:     6000.0,
			VRAMUsedBytes:  4294967296,
			VRAMTotalBytes: 8589934592,
		},
		Memory: &protocol.MemoryMetrics{
			TotalBytes:     16000000000,
			AvailableBytes: 8000000000,
			UsagePercent:   50.0,
			SwapTotalBytes: 8000000000,
			SwapFreeBytes:  6000000000,
		},
		Battery: &protocol.BatteryMetrics{
			Capacity: 85,
			Status:   "discharging",
		},
		Power: &protocol.PowerMetrics{
			TDPWatts:   15.0,
			PowerWatts: 12.5,
		},
		Fan: &protocol.FanMetrics{
			RPM: 2400,
		},
		Steam: &protocol.SteamStatus{
			Running:    true,
			GamingMode: true,
		},
	})

	// ConsoleLogBatch
	writeFixture("console_log_batch.json", protocol.ConsoleLogBatch{
		Entries: []protocol.ConsoleLogEntry{
			{
				Timestamp: 1700000001,
				Level:     "error",
				Source:    "console",
				Text:      "Something went wrong",
				URL:       "https://example.com/app.js",
				Line:      42,
				Segments: []protocol.StyledSegment{
					{Text: "Error: ", CSS: "color: red; font-weight: bold"},
					{Text: "details here"},
				},
			},
			{
				Timestamp: 1700000002,
				Level:     "log",
				Source:    "console",
				Text:      "Hello world",
			},
		},
		Dropped: 3,
	})

	// GameLogWrapperStatusEvent
	writeFixture("game_log_wrapper_status.json", protocol.GameLogWrapperStatusEvent{
		Wrappers: map[uint32]bool{
			12345: true,
			67890: false,
		},
	})

	// ShortcutConfig
	writeFixture("shortcut_config.json", protocol.ShortcutConfig{
		Name:          "Test Game",
		Exe:           "/games/test/game.exe",
		StartDir:      "/games/test",
		LaunchOptions: "--fullscreen",
		Tags:          []string{"RPG", "Indie"},
		Artwork: &protocol.ArtworkConfig{
			Grid: "/art/grid.png",
			Hero: "/art/hero.png",
			Logo: "/art/logo.png",
		},
	})

	// ShortcutInfo
	writeFixture("shortcut_info.json", protocol.ShortcutInfo{
		AppID:         2147483648,
		Name:          "Test Shortcut",
		Exe:           "/usr/bin/game",
		StartDir:      "/home/user",
		LaunchOptions: "--fullscreen",
		Tags:          []string{"Action", "RPG"},
		LastPlayed:    1700000000,
	})

	// OperationEvent
	writeFixture("operation_event.json", protocol.OperationEvent{
		Type:     "install",
		Status:   "progress",
		GameName: "Test Game",
		Progress: 42.5,
		Message:  "Extracting files...",
	})

	// Steam AppID generation fixture
	exe := "/usr/bin/game"
	name := "My Test Game"
	writeFixture("steam_app_id.json", map[string]any{
		"exe":   exe,
		"name":  name,
		"appId": steam.GenerateAppID(exe, name),
	})

	t.Logf("All fixtures generated in %s", dir)
}
