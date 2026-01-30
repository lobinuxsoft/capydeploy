package steam

import (
	"os"
	"path/filepath"
	"testing"
)

func TestNewPathsWithBase(t *testing.T) {
	baseDir := filepath.Join("test", "steam")
	paths := NewPathsWithBase(baseDir)

	if paths.BaseDir() != baseDir {
		t.Errorf("BaseDir() = %q, want %q", paths.BaseDir(), baseDir)
	}
}

func TestPaths_UserDataDir(t *testing.T) {
	baseDir := filepath.Join("test", "steam")
	paths := NewPathsWithBase(baseDir)

	want := filepath.Join("test", "steam", "userdata")
	if got := paths.UserDataDir(); got != want {
		t.Errorf("UserDataDir() = %q, want %q", got, want)
	}
}

func TestPaths_UserDir(t *testing.T) {
	baseDir := filepath.Join("test", "steam")
	paths := NewPathsWithBase(baseDir)

	want := filepath.Join("test", "steam", "userdata", "12345")
	if got := paths.UserDir("12345"); got != want {
		t.Errorf("UserDir() = %q, want %q", got, want)
	}
}

func TestPaths_ConfigDir(t *testing.T) {
	baseDir := filepath.Join("test", "steam")
	paths := NewPathsWithBase(baseDir)

	want := filepath.Join("test", "steam", "userdata", "12345", "config")
	if got := paths.ConfigDir("12345"); got != want {
		t.Errorf("ConfigDir() = %q, want %q", got, want)
	}
}

func TestPaths_ShortcutsPath(t *testing.T) {
	baseDir := filepath.Join("test", "steam")
	paths := NewPathsWithBase(baseDir)

	want := filepath.Join("test", "steam", "userdata", "12345", "config", "shortcuts.vdf")
	if got := paths.ShortcutsPath("12345"); got != want {
		t.Errorf("ShortcutsPath() = %q, want %q", got, want)
	}
}

func TestPaths_GridDir(t *testing.T) {
	baseDir := filepath.Join("test", "steam")
	paths := NewPathsWithBase(baseDir)

	want := filepath.Join("test", "steam", "userdata", "12345", "config", "grid")
	if got := paths.GridDir("12345"); got != want {
		t.Errorf("GridDir() = %q, want %q", got, want)
	}
}

func TestPaths_HasShortcuts(t *testing.T) {
	tmpDir := t.TempDir()
	paths := NewPathsWithBase(tmpDir)

	userID := "12345"
	configDir := filepath.Join(tmpDir, "userdata", userID, "config")

	// Initially no shortcuts
	if paths.HasShortcuts(userID) {
		t.Error("HasShortcuts() should return false when file doesn't exist")
	}

	// Create shortcuts.vdf
	os.MkdirAll(configDir, 0755)
	shortcutsPath := filepath.Join(configDir, "shortcuts.vdf")
	os.WriteFile(shortcutsPath, []byte{}, 0644)

	if !paths.HasShortcuts(userID) {
		t.Error("HasShortcuts() should return true when file exists")
	}
}

func TestPaths_EnsureGridDir(t *testing.T) {
	tmpDir := t.TempDir()
	paths := NewPathsWithBase(tmpDir)

	userID := "12345"
	gridDir := paths.GridDir(userID)

	// Grid dir shouldn't exist yet
	if _, err := os.Stat(gridDir); err == nil {
		t.Error("Grid dir should not exist initially")
	}

	// Ensure creates it
	if err := paths.EnsureGridDir(userID); err != nil {
		t.Fatalf("EnsureGridDir() error = %v", err)
	}

	// Now it should exist
	if _, err := os.Stat(gridDir); err != nil {
		t.Error("Grid dir should exist after EnsureGridDir()")
	}
}

func TestPaths_ArtworkPath(t *testing.T) {
	baseDir := filepath.Join("test", "steam")
	paths := NewPathsWithBase(baseDir)

	tests := []struct {
		name    string
		appID   uint32
		artType ArtworkType
		ext     string
		want    string
	}{
		{
			name:    "grid artwork",
			appID:   12345,
			artType: ArtworkGrid,
			ext:     "png",
			want:    filepath.Join("test", "steam", "userdata", "99999", "config", "grid", "12345.png"),
		},
		{
			name:    "hero artwork",
			appID:   12345,
			artType: ArtworkHero,
			ext:     "jpg",
			want:    filepath.Join("test", "steam", "userdata", "99999", "config", "grid", "12345_hero.jpg"),
		},
		{
			name:    "logo artwork",
			appID:   12345,
			artType: ArtworkLogo,
			ext:     "png",
			want:    filepath.Join("test", "steam", "userdata", "99999", "config", "grid", "12345_logo.png"),
		},
		{
			name:    "icon artwork",
			appID:   12345,
			artType: ArtworkIcon,
			ext:     "ico",
			want:    filepath.Join("test", "steam", "userdata", "99999", "config", "grid", "12345_icon.ico"),
		},
		{
			name:    "portrait artwork",
			appID:   12345,
			artType: ArtworkPortrait,
			ext:     "png",
			want:    filepath.Join("test", "steam", "userdata", "99999", "config", "grid", "12345p.png"),
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := paths.ArtworkPath("99999", tt.appID, tt.artType, tt.ext)
			if got != tt.want {
				t.Errorf("ArtworkPath() = %q, want %q", got, tt.want)
			}
		})
	}
}

func TestArtworkFilename(t *testing.T) {
	tests := []struct {
		name    string
		appID   uint32
		artType ArtworkType
		ext     string
		want    string
	}{
		{"grid", 123, ArtworkGrid, "png", "123.png"},
		{"hero", 123, ArtworkHero, "png", "123_hero.png"},
		{"logo", 123, ArtworkLogo, "png", "123_logo.png"},
		{"icon", 123, ArtworkIcon, "png", "123_icon.png"},
		{"portrait", 123, ArtworkPortrait, "png", "123p.png"},
		{"default ext", 123, ArtworkGrid, "", "123.png"},
		{"unknown type", 123, ArtworkType(99), "png", "123.png"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := artworkFilename(tt.appID, tt.artType, tt.ext)
			if got != tt.want {
				t.Errorf("artworkFilename() = %q, want %q", got, tt.want)
			}
		})
	}
}

func TestFormatFilename(t *testing.T) {
	tests := []struct {
		appID  uint32
		suffix string
		ext    string
		want   string
	}{
		{123, "", "png", "123.png"},
		{123, "_hero", "png", "123_hero.png"},
		{123, "p", "jpg", "123p.jpg"},
		{123, "", "", "123.png"}, // Default extension
	}

	for _, tt := range tests {
		got := formatFilename(tt.appID, tt.suffix, tt.ext)
		if got != tt.want {
			t.Errorf("formatFilename(%d, %q, %q) = %q, want %q",
				tt.appID, tt.suffix, tt.ext, got, tt.want)
		}
	}
}

func TestArtworkType_Constants(t *testing.T) {
	// Verify artwork types have distinct values
	types := []ArtworkType{
		ArtworkGrid,
		ArtworkHero,
		ArtworkLogo,
		ArtworkIcon,
		ArtworkPortrait,
	}

	seen := make(map[ArtworkType]bool)
	for _, at := range types {
		if seen[at] {
			t.Errorf("Duplicate ArtworkType value: %d", at)
		}
		seen[at] = true
	}
}

func TestErrors(t *testing.T) {
	// Verify error variables are defined
	errors := []error{
		ErrSteamNotFound,
		ErrUserNotFound,
		ErrImageNotFound,
		ErrShortcutsNotFound,
	}

	for _, err := range errors {
		if err == nil {
			t.Error("Error should not be nil")
		}
		if err.Error() == "" {
			t.Error("Error message should not be empty")
		}
	}
}
