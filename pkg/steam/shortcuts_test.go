package steam

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

func TestNewShortcutManagerWithPaths(t *testing.T) {
	baseDir := filepath.Join("test", "steam")
	paths := NewPathsWithBase(baseDir)
	mgr := NewShortcutManagerWithPaths(paths)

	if mgr == nil {
		t.Fatal("NewShortcutManagerWithPaths() returned nil")
	}
}

func TestShortcutManager_GetShortcutsPath(t *testing.T) {
	baseDir := filepath.Join("test", "steam")
	paths := NewPathsWithBase(baseDir)
	mgr := NewShortcutManagerWithPaths(paths)

	want := filepath.Join("test", "steam", "userdata", "12345", "config", "shortcuts.vdf")
	if got := mgr.GetShortcutsPath("12345"); got != want {
		t.Errorf("GetShortcutsPath() = %q, want %q", got, want)
	}
}

func TestShortcutManager_GetGridDir(t *testing.T) {
	baseDir := filepath.Join("test", "steam")
	paths := NewPathsWithBase(baseDir)
	mgr := NewShortcutManagerWithPaths(paths)

	want := filepath.Join("test", "steam", "userdata", "12345", "config", "grid")
	if got := mgr.GetGridDir("12345"); got != want {
		t.Errorf("GetGridDir() = %q, want %q", got, want)
	}
}

func TestShortcutManager_EnsureGridDir(t *testing.T) {
	tmpDir := t.TempDir()
	paths := NewPathsWithBase(tmpDir)
	mgr := NewShortcutManagerWithPaths(paths)

	userID := "12345"
	gridDir := mgr.GetGridDir(userID)

	// Should not exist initially
	if _, err := os.Stat(gridDir); err == nil {
		t.Error("Grid dir should not exist initially")
	}

	// Create it
	if err := mgr.EnsureGridDir(userID); err != nil {
		t.Fatalf("EnsureGridDir() error = %v", err)
	}

	// Should exist now
	if _, err := os.Stat(gridDir); err != nil {
		t.Error("Grid dir should exist after EnsureGridDir()")
	}
}

func TestGenerateAppID(t *testing.T) {
	// Test that same inputs produce same output
	appID1 := GenerateAppID("/path/to/game.exe", "My Game")
	appID2 := GenerateAppID("/path/to/game.exe", "My Game")

	if appID1 != appID2 {
		t.Errorf("GenerateAppID() not deterministic: %d != %d", appID1, appID2)
	}

	// Test that different inputs produce different outputs
	appID3 := GenerateAppID("/path/to/other.exe", "My Game")
	if appID1 == appID3 {
		t.Error("GenerateAppID() should produce different IDs for different exe paths")
	}

	appID4 := GenerateAppID("/path/to/game.exe", "Other Game")
	if appID1 == appID4 {
		t.Error("GenerateAppID() should produce different IDs for different game names")
	}

	// Test that the ID has the shortcut bit set (high bit)
	if appID1&0x80000000 == 0 {
		t.Error("GenerateAppID() should set the high bit for shortcuts")
	}
}

func TestConvertToShortcutInfo(t *testing.T) {
	cfg := protocol.ShortcutConfig{
		Name:          "Test Game",
		Exe:           "/path/to/game.exe",
		StartDir:      "/path/to",
		LaunchOptions: "-fullscreen",
		Tags:          []string{"action", "indie"},
	}

	info := ConvertToShortcutInfo(cfg)

	if info.Name != cfg.Name {
		t.Errorf("Name = %q, want %q", info.Name, cfg.Name)
	}
	if info.Exe != cfg.Exe {
		t.Errorf("Exe = %q, want %q", info.Exe, cfg.Exe)
	}
	if info.StartDir != cfg.StartDir {
		t.Errorf("StartDir = %q, want %q", info.StartDir, cfg.StartDir)
	}
	if info.LaunchOptions != cfg.LaunchOptions {
		t.Errorf("LaunchOptions = %q, want %q", info.LaunchOptions, cfg.LaunchOptions)
	}
	if len(info.Tags) != len(cfg.Tags) {
		t.Errorf("Tags length = %d, want %d", len(info.Tags), len(cfg.Tags))
	}

	// Verify AppID was generated
	expectedAppID := GenerateAppID(cfg.Exe, cfg.Name)
	if info.AppID != expectedAppID {
		t.Errorf("AppID = %d, want %d", info.AppID, expectedAppID)
	}
}

func TestShortcutManager_ArtworkPaths(t *testing.T) {
	baseDir := filepath.Join("test", "steam")
	paths := NewPathsWithBase(baseDir)
	mgr := NewShortcutManagerWithPaths(paths)

	userID := "12345"
	appID := uint32(99999)

	artPaths := mgr.ArtworkPaths(userID, appID)

	// Should have all artwork types
	if len(artPaths) != 5 {
		t.Errorf("ArtworkPaths() returned %d paths, want 5", len(artPaths))
	}

	// Check each type has a path
	for _, artType := range []ArtworkType{ArtworkGrid, ArtworkHero, ArtworkLogo, ArtworkIcon, ArtworkPortrait} {
		if path, ok := artPaths[artType]; !ok || path == "" {
			t.Errorf("ArtworkPaths() missing path for type %d", artType)
		}
	}
}

func TestShortcutManager_FindExistingArtwork(t *testing.T) {
	tmpDir := t.TempDir()
	paths := NewPathsWithBase(tmpDir)
	mgr := NewShortcutManagerWithPaths(paths)

	userID := "12345"
	appID := uint32(99999)

	// Create grid directory and some artwork
	gridDir := mgr.GetGridDir(userID)
	os.MkdirAll(gridDir, 0755)

	// Create grid artwork
	os.WriteFile(filepath.Join(gridDir, "99999.png"), []byte("grid"), 0644)
	// Create hero artwork
	os.WriteFile(filepath.Join(gridDir, "99999_hero.jpg"), []byte("hero"), 0644)

	existing, err := mgr.FindExistingArtwork(userID, appID)
	if err != nil {
		t.Fatalf("FindExistingArtwork() error = %v", err)
	}

	if _, ok := existing[ArtworkGrid]; !ok {
		t.Error("Should find grid artwork")
	}
	if _, ok := existing[ArtworkHero]; !ok {
		t.Error("Should find hero artwork")
	}
	if _, ok := existing[ArtworkLogo]; ok {
		t.Error("Should not find logo artwork (not created)")
	}
}

func TestShortcutManager_SaveArtwork(t *testing.T) {
	tmpDir := t.TempDir()
	paths := NewPathsWithBase(tmpDir)
	mgr := NewShortcutManagerWithPaths(paths)

	userID := "12345"
	appID := uint32(99999)
	data := []byte("PNG image data")

	if err := mgr.SaveArtwork(userID, appID, ArtworkGrid, data, "png"); err != nil {
		t.Fatalf("SaveArtwork() error = %v", err)
	}

	// Verify file was created
	expected := filepath.Join(mgr.GetGridDir(userID), "99999.png")
	content, err := os.ReadFile(expected)
	if err != nil {
		t.Fatalf("Failed to read saved artwork: %v", err)
	}
	if string(content) != string(data) {
		t.Errorf("Saved content = %q, want %q", content, data)
	}
}

func TestShortcutManager_SaveArtwork_CleanExtension(t *testing.T) {
	tmpDir := t.TempDir()
	paths := NewPathsWithBase(tmpDir)
	mgr := NewShortcutManagerWithPaths(paths)

	userID := "12345"
	appID := uint32(99999)

	// Test with leading dot
	if err := mgr.SaveArtwork(userID, appID, ArtworkHero, []byte("data"), ".jpg"); err != nil {
		t.Fatalf("SaveArtwork() error = %v", err)
	}

	expected := filepath.Join(mgr.GetGridDir(userID), "99999_hero.jpg")
	if _, err := os.Stat(expected); err != nil {
		t.Errorf("File should be saved at %q", expected)
	}
}

func TestShortcutManager_SaveArtwork_DefaultExtension(t *testing.T) {
	tmpDir := t.TempDir()
	paths := NewPathsWithBase(tmpDir)
	mgr := NewShortcutManagerWithPaths(paths)

	userID := "12345"
	appID := uint32(99999)

	// Test with empty extension
	if err := mgr.SaveArtwork(userID, appID, ArtworkLogo, []byte("data"), ""); err != nil {
		t.Fatalf("SaveArtwork() error = %v", err)
	}

	expected := filepath.Join(mgr.GetGridDir(userID), "99999_logo.png")
	if _, err := os.Stat(expected); err != nil {
		t.Errorf("File should be saved at %q with default .png extension", expected)
	}
}

func TestShortcutManager_DeleteArtwork(t *testing.T) {
	tmpDir := t.TempDir()
	paths := NewPathsWithBase(tmpDir)
	mgr := NewShortcutManagerWithPaths(paths)

	userID := "12345"
	appID := uint32(99999)

	// Create some artwork
	gridDir := mgr.GetGridDir(userID)
	os.MkdirAll(gridDir, 0755)
	os.WriteFile(filepath.Join(gridDir, "99999.png"), []byte("grid"), 0644)
	os.WriteFile(filepath.Join(gridDir, "99999_hero.png"), []byte("hero"), 0644)

	// Delete all artwork
	if err := mgr.DeleteArtwork(userID, appID); err != nil {
		t.Fatalf("DeleteArtwork() error = %v", err)
	}

	// Verify files are gone
	if _, err := os.Stat(filepath.Join(gridDir, "99999.png")); err == nil {
		t.Error("Grid artwork should be deleted")
	}
	if _, err := os.Stat(filepath.Join(gridDir, "99999_hero.png")); err == nil {
		t.Error("Hero artwork should be deleted")
	}
}

func TestShortcutManager_DeleteArtwork_NoFiles(t *testing.T) {
	tmpDir := t.TempDir()
	paths := NewPathsWithBase(tmpDir)
	mgr := NewShortcutManagerWithPaths(paths)

	userID := "12345"
	appID := uint32(99999)

	// Should not error even if no files exist
	if err := mgr.DeleteArtwork(userID, appID); err != nil {
		t.Errorf("DeleteArtwork() with no files should not error: %v", err)
	}
}
