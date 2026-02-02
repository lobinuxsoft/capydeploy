package protocol

import (
	"testing"
	"time"
)

func TestUploadProgress_Percentage(t *testing.T) {
	tests := []struct {
		name     string
		progress UploadProgress
		want     float64
	}{
		{
			name:     "zero total bytes",
			progress: UploadProgress{TotalBytes: 0, TransferredBytes: 0},
			want:     0,
		},
		{
			name:     "zero transferred",
			progress: UploadProgress{TotalBytes: 100, TransferredBytes: 0},
			want:     0,
		},
		{
			name:     "half transferred",
			progress: UploadProgress{TotalBytes: 100, TransferredBytes: 50},
			want:     50,
		},
		{
			name:     "fully transferred",
			progress: UploadProgress{TotalBytes: 100, TransferredBytes: 100},
			want:     100,
		},
		{
			name:     "large file partial",
			progress: UploadProgress{TotalBytes: 1024 * 1024 * 100, TransferredBytes: 1024 * 1024 * 25},
			want:     25,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := tt.progress.Percentage()
			if got != tt.want {
				t.Errorf("Percentage() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestUploadStatus_Constants(t *testing.T) {
	// Verify status constants have expected values
	statuses := map[UploadStatus]string{
		UploadStatusPending:    "pending",
		UploadStatusInProgress: "in_progress",
		UploadStatusCompleted:  "completed",
		UploadStatusFailed:     "failed",
		UploadStatusCancelled:  "cancelled",
	}

	for status, expected := range statuses {
		if string(status) != expected {
			t.Errorf("UploadStatus %v = %q, want %q", status, string(status), expected)
		}
	}
}

func TestAgentInfo_Fields(t *testing.T) {
	info := AgentInfo{
		ID:       "agent-1",
		Name:     "Test Agent",
		Platform: "steamdeck",
		Version:  "1.0.0",
	}

	if info.ID != "agent-1" {
		t.Errorf("ID = %q, want %q", info.ID, "agent-1")
	}
	if info.Name != "Test Agent" {
		t.Errorf("Name = %q, want %q", info.Name, "Test Agent")
	}
	if info.Platform != "steamdeck" {
		t.Errorf("Platform = %q, want %q", info.Platform, "steamdeck")
	}
}

func TestUploadConfig_Fields(t *testing.T) {
	cfg := UploadConfig{
		GameName:      "Test Game",
		InstallPath:    "/games/test",
		Executable:    "game.exe",
		LaunchOptions: "-fullscreen",
		Tags:          "action,indie",
	}

	if cfg.GameName != "Test Game" {
		t.Errorf("GameName = %q, want %q", cfg.GameName, "Test Game")
	}
	if cfg.InstallPath != "/games/test" {
		t.Errorf("InstallPath = %q, want %q", cfg.InstallPath, "/games/test")
	}
}

func TestShortcutConfig_WithArtwork(t *testing.T) {
	artwork := &ArtworkConfig{
		Grid: "/path/to/grid.png",
		Hero: "/path/to/hero.png",
	}

	cfg := ShortcutConfig{
		Name:    "My Game",
		Exe:     "/games/game.exe",
		StartDir: "/games",
		Tags:    []string{"action", "indie"},
		Artwork: artwork,
	}

	if cfg.Artwork == nil {
		t.Fatal("Artwork should not be nil")
	}
	if cfg.Artwork.Grid != "/path/to/grid.png" {
		t.Errorf("Artwork.Grid = %q, want %q", cfg.Artwork.Grid, "/path/to/grid.png")
	}
}

func TestShortcutInfo_Fields(t *testing.T) {
	now := time.Now().Unix()
	info := ShortcutInfo{
		AppID:         12345,
		Name:          "Test Shortcut",
		Exe:           "/path/to/exe",
		StartDir:      "/path/to",
		LaunchOptions: "-test",
		Tags:          []string{"tag1", "tag2"},
		LastPlayed:    now,
	}

	if info.AppID != 12345 {
		t.Errorf("AppID = %d, want %d", info.AppID, 12345)
	}
	if len(info.Tags) != 2 {
		t.Errorf("Tags length = %d, want %d", len(info.Tags), 2)
	}
}
