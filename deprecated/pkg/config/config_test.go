package config

import (
	"encoding/json"
	"os"
	"testing"
)

func TestLoad_Default(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	cfg, err := Load()
	if err != nil {
		t.Fatalf("Load() error = %v", err)
	}

	// Default config should have image cache enabled
	if !cfg.ImageCacheEnabled {
		t.Error("default ImageCacheEnabled = false, want true")
	}
	if len(cfg.GameSetups) != 0 {
		t.Errorf("default GameSetups len = %d, want 0", len(cfg.GameSetups))
	}
}

func TestSaveAndLoad(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	original := &AppConfig{
		GameSetups: []GameSetup{
			{ID: "game-1", Name: "Test Game", LocalPath: "/games/test"},
		},
		SteamGridDBAPIKey: "test-key",
		ImageCacheEnabled: true,
	}

	if err := Save(original); err != nil {
		t.Fatalf("Save() error = %v", err)
	}

	loaded, err := Load()
	if err != nil {
		t.Fatalf("Load() error = %v", err)
	}

	if len(loaded.GameSetups) != 1 {
		t.Fatalf("loaded GameSetups len = %d, want 1", len(loaded.GameSetups))
	}
	if loaded.GameSetups[0].Name != "Test Game" {
		t.Errorf("loaded game name = %q, want %q", loaded.GameSetups[0].Name, "Test Game")
	}
	if loaded.SteamGridDBAPIKey != "test-key" {
		t.Errorf("loaded API key = %q, want %q", loaded.SteamGridDBAPIKey, "test-key")
	}
}

func TestAddGameSetup(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	setup := GameSetup{
		Name:      "My Game",
		LocalPath: "/home/user/games/mygame",
		Executable: "game.exe",
	}

	if err := AddGameSetup(setup); err != nil {
		t.Fatalf("AddGameSetup() error = %v", err)
	}

	setups, err := GetGameSetups()
	if err != nil {
		t.Fatalf("GetGameSetups() error = %v", err)
	}

	if len(setups) != 1 {
		t.Fatalf("GetGameSetups() len = %d, want 1", len(setups))
	}
	if setups[0].Name != "My Game" {
		t.Errorf("setup name = %q, want %q", setups[0].Name, "My Game")
	}
	// ID should be auto-generated
	if setups[0].ID == "" {
		t.Error("setup ID should be auto-generated")
	}
}

func TestAddGameSetup_WithID(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	setup := GameSetup{
		ID:   "custom-id",
		Name: "Custom Game",
	}

	if err := AddGameSetup(setup); err != nil {
		t.Fatalf("AddGameSetup() error = %v", err)
	}

	setups, _ := GetGameSetups()
	if setups[0].ID != "custom-id" {
		t.Errorf("setup ID = %q, want %q", setups[0].ID, "custom-id")
	}
}

func TestAddGameSetup_UpdateExisting(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	AddGameSetup(GameSetup{ID: "game-1", Name: "Old Name"})

	// Add again with same ID should update
	AddGameSetup(GameSetup{ID: "game-1", Name: "New Name"})

	setups, _ := GetGameSetups()
	if len(setups) != 1 {
		t.Fatalf("expected 1 setup after update, got %d", len(setups))
	}
	if setups[0].Name != "New Name" {
		t.Errorf("updated name = %q, want %q", setups[0].Name, "New Name")
	}
}

func TestUpdateGameSetup(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	AddGameSetup(GameSetup{ID: "game-1", Name: "Original"})

	updated := GameSetup{
		Name:       "Updated Game",
		Executable: "new.exe",
	}

	if err := UpdateGameSetup("game-1", updated); err != nil {
		t.Fatalf("UpdateGameSetup() error = %v", err)
	}

	setups, _ := GetGameSetups()
	if setups[0].Name != "Updated Game" {
		t.Errorf("updated name = %q, want %q", setups[0].Name, "Updated Game")
	}
	if setups[0].ID != "game-1" {
		t.Errorf("ID should be preserved: %q", setups[0].ID)
	}
}

func TestUpdateGameSetup_NotFound(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	err := UpdateGameSetup("nonexistent", GameSetup{Name: "New"})
	if err == nil {
		t.Error("UpdateGameSetup() should return error for nonexistent ID")
	}
}

func TestRemoveGameSetup(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	AddGameSetup(GameSetup{ID: "game-1", Name: "Game 1"})
	AddGameSetup(GameSetup{ID: "game-2", Name: "Game 2"})

	if err := RemoveGameSetup("game-1"); err != nil {
		t.Fatalf("RemoveGameSetup() error = %v", err)
	}

	setups, _ := GetGameSetups()
	if len(setups) != 1 {
		t.Fatalf("expected 1 setup after remove, got %d", len(setups))
	}
	if setups[0].ID != "game-2" {
		t.Errorf("remaining setup ID = %q, want %q", setups[0].ID, "game-2")
	}
}

func TestRemoveGameSetup_NotFound(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	// Removing non-existent setup should not error
	if err := RemoveGameSetup("nonexistent"); err != nil {
		t.Errorf("RemoveGameSetup() error = %v, want nil", err)
	}
}

func TestGetSetSteamGridDBAPIKey(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	// Default should be empty
	key, err := GetSteamGridDBAPIKey()
	if err != nil {
		t.Fatalf("GetSteamGridDBAPIKey() error = %v", err)
	}
	if key != "" {
		t.Errorf("default API key = %q, want empty", key)
	}

	// Set
	if err := SetSteamGridDBAPIKey("my-api-key-123"); err != nil {
		t.Fatalf("SetSteamGridDBAPIKey() error = %v", err)
	}

	// Get
	key, err = GetSteamGridDBAPIKey()
	if err != nil {
		t.Fatalf("GetSteamGridDBAPIKey() error = %v", err)
	}
	if key != "my-api-key-123" {
		t.Errorf("API key = %q, want %q", key, "my-api-key-123")
	}
}

func TestGetSetImageCacheEnabled(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	// Default should be true
	enabled, err := GetImageCacheEnabled()
	if err != nil {
		t.Fatalf("GetImageCacheEnabled() error = %v", err)
	}
	if !enabled {
		t.Error("default ImageCacheEnabled = false, want true")
	}

	// Disable
	if err := SetImageCacheEnabled(false); err != nil {
		t.Fatalf("SetImageCacheEnabled() error = %v", err)
	}

	enabled, err = GetImageCacheEnabled()
	if err != nil {
		t.Fatalf("GetImageCacheEnabled() error = %v", err)
	}
	if enabled {
		t.Error("ImageCacheEnabled = true after setting false")
	}
}

func TestSave_FilePermissions(t *testing.T) {
	tmpDir := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", tmpDir)

	Save(&AppConfig{ImageCacheEnabled: true})

	configPath, _ := GetConfigPath()
	info, err := os.Stat(configPath)
	if err != nil {
		t.Fatalf("config file not found: %v", err)
	}

	perm := info.Mode().Perm()
	if perm != 0600 {
		t.Errorf("config file permissions = %o, want 0600", perm)
	}
}

func TestSave_ValidJSON(t *testing.T) {
	tmpDir := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", tmpDir)

	Save(&AppConfig{
		GameSetups:        []GameSetup{{ID: "g1", Name: "Game"}},
		SteamGridDBAPIKey: "key",
		ImageCacheEnabled: true,
	})

	configPath, _ := GetConfigPath()
	data, err := os.ReadFile(configPath)
	if err != nil {
		t.Fatalf("failed to read config: %v", err)
	}

	var cfg AppConfig
	if err := json.Unmarshal(data, &cfg); err != nil {
		t.Fatalf("config is not valid JSON: %v", err)
	}
}

func TestGameSetup_AllFields(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	setup := GameSetup{
		ID:             "test-full",
		Name:           "Full Game",
		LocalPath:      "/home/user/games/full",
		Executable:     "game.sh",
		LaunchOptions:  "--windowed",
		Tags:           "rpg,action",
		InstallPath:    "~/Games/Full",
		GridDBGameID:   42,
		GridPortrait:   "https://example.com/portrait.png",
		GridLandscape:  "https://example.com/landscape.png",
		HeroImage:      "https://example.com/hero.png",
		LogoImage:      "https://example.com/logo.png",
		IconImage:      "https://example.com/icon.png",
	}

	AddGameSetup(setup)

	setups, _ := GetGameSetups()
	if len(setups) != 1 {
		t.Fatalf("expected 1 setup, got %d", len(setups))
	}

	got := setups[0]
	if got.GridDBGameID != 42 {
		t.Errorf("GridDBGameID = %d, want 42", got.GridDBGameID)
	}
	if got.Tags != "rpg,action" {
		t.Errorf("Tags = %q, want %q", got.Tags, "rpg,action")
	}
	if got.HeroImage != "https://example.com/hero.png" {
		t.Errorf("HeroImage = %q, want %q", got.HeroImage, "https://example.com/hero.png")
	}
}
