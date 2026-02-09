package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/lobinuxsoft/capydeploy/pkg/config"
)

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

// =============================================================================
// Artwork local file tests
// =============================================================================

func TestDetectContentType(t *testing.T) {
	tests := []struct {
		path string
		want string
	}{
		{"/images/hero.png", "image/png"},
		{"/images/hero.PNG", "image/png"},
		{"/images/capsule.jpg", "image/jpeg"},
		{"/images/capsule.jpeg", "image/jpeg"},
		{"/images/logo.webp", "image/webp"},
		{"/images/readme.txt", ""},
		{"/images/noext", ""},
	}

	for _, tt := range tests {
		t.Run(filepath.Base(tt.path), func(t *testing.T) {
			got := detectContentType(tt.path)
			if got != tt.want {
				t.Errorf("detectContentType(%q) = %q, want %q", tt.path, got, tt.want)
			}
		})
	}
}

func TestReadArtworkFile(t *testing.T) {
	tmpDir := t.TempDir()

	t.Run("valid PNG file", func(t *testing.T) {
		// Create a small PNG-like file
		path := filepath.Join(tmpDir, "test.png")
		data := []byte{0x89, 'P', 'N', 'G', 0x0D, 0x0A, 0x1A, 0x0A}
		if err := os.WriteFile(path, data, 0644); err != nil {
			t.Fatal(err)
		}

		result, err := readArtworkFile(path)
		if err != nil {
			t.Fatalf("readArtworkFile failed: %v", err)
		}
		if result.Path != path {
			t.Errorf("path = %q, want %q", result.Path, path)
		}
		if result.ContentType != "image/png" {
			t.Errorf("contentType = %q, want image/png", result.ContentType)
		}
		if result.Size != int64(len(data)) {
			t.Errorf("size = %d, want %d", result.Size, len(data))
		}
		if !strings.HasPrefix(result.DataURI, "data:image/png;base64,") {
			t.Errorf("dataURI should start with data:image/png;base64, got: %s", result.DataURI[:40])
		}
	})

	t.Run("unsupported format", func(t *testing.T) {
		path := filepath.Join(tmpDir, "test.txt")
		if err := os.WriteFile(path, []byte("hello"), 0644); err != nil {
			t.Fatal(err)
		}

		_, err := readArtworkFile(path)
		if err == nil {
			t.Error("expected error for unsupported format")
		}
	})

	t.Run("file too large", func(t *testing.T) {
		path := filepath.Join(tmpDir, "huge.png")
		// Create a file just over the 8MB limit
		data := make([]byte, maxArtworkSize+1)
		if err := os.WriteFile(path, data, 0644); err != nil {
			t.Fatal(err)
		}

		_, err := readArtworkFile(path)
		if err == nil {
			t.Error("expected error for file too large")
		}
		if !strings.Contains(err.Error(), "too large") {
			t.Errorf("error should mention 'too large', got: %v", err)
		}
	})

	t.Run("nonexistent file", func(t *testing.T) {
		_, err := readArtworkFile(filepath.Join(tmpDir, "nope.png"))
		if err == nil {
			t.Error("expected error for nonexistent file")
		}
	})
}

func TestBuildRemoteArtworkConfig(t *testing.T) {
	t.Run("all remote", func(t *testing.T) {
		setup := &config.GameSetup{
			GridPortrait:  "https://cdn.steamgriddb.com/grid.png",
			GridLandscape: "https://cdn.steamgriddb.com/banner.png",
			HeroImage:     "https://cdn.steamgriddb.com/hero.png",
			LogoImage:     "https://cdn.steamgriddb.com/logo.png",
			IconImage:     "https://cdn.steamgriddb.com/icon.png",
		}

		cfg := buildRemoteArtworkConfig(setup)
		if cfg == nil {
			t.Fatal("config should not be nil for remote URLs")
		}
		if cfg.Grid != setup.GridPortrait {
			t.Errorf("Grid = %q, want %q", cfg.Grid, setup.GridPortrait)
		}
		if cfg.Banner != setup.GridLandscape {
			t.Errorf("Banner = %q, want %q", cfg.Banner, setup.GridLandscape)
		}
		if cfg.Hero != setup.HeroImage {
			t.Errorf("Hero = %q, want %q", cfg.Hero, setup.HeroImage)
		}
	})

	t.Run("all local", func(t *testing.T) {
		setup := &config.GameSetup{
			GridPortrait: "file:///home/user/grid.png",
			HeroImage:    "file:///home/user/hero.png",
		}

		cfg := buildRemoteArtworkConfig(setup)
		if cfg != nil {
			t.Error("config should be nil when all artwork is local")
		}
	})

	t.Run("mixed", func(t *testing.T) {
		setup := &config.GameSetup{
			GridPortrait: "https://cdn.steamgriddb.com/grid.png",
			HeroImage:    "file:///home/user/hero.png",
			LogoImage:    "https://cdn.steamgriddb.com/logo.png",
		}

		cfg := buildRemoteArtworkConfig(setup)
		if cfg == nil {
			t.Fatal("config should not be nil for mixed URLs")
		}
		if cfg.Grid != setup.GridPortrait {
			t.Errorf("Grid should be set for remote URL")
		}
		if cfg.Hero != "" {
			t.Errorf("Hero should be empty for local file, got %q", cfg.Hero)
		}
		if cfg.Logo != setup.LogoImage {
			t.Errorf("Logo should be set for remote URL")
		}
	})

	t.Run("empty", func(t *testing.T) {
		setup := &config.GameSetup{}
		cfg := buildRemoteArtworkConfig(setup)
		if cfg != nil {
			t.Error("config should be nil when no artwork is set")
		}
	})
}
