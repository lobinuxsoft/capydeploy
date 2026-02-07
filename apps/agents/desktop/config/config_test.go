package config

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestNewManager(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	mgr, err := NewManager()
	if err != nil {
		t.Fatalf("NewManager() error = %v", err)
	}

	// Should have default values
	if mgr.GetName() == "" {
		t.Error("GetName() returned empty string")
	}
	if mgr.GetInstallPath() == "" {
		t.Error("GetInstallPath() returned empty string")
	}
}

func TestGetSetName(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	mgr, err := NewManager()
	if err != nil {
		t.Fatalf("NewManager() error = %v", err)
	}

	if err := mgr.SetName("my-agent"); err != nil {
		t.Fatalf("SetName() error = %v", err)
	}

	if got := mgr.GetName(); got != "my-agent" {
		t.Errorf("GetName() = %q, want %q", got, "my-agent")
	}
}

func TestGetSetInstallPath(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	mgr, err := NewManager()
	if err != nil {
		t.Fatalf("NewManager() error = %v", err)
	}

	if err := mgr.SetInstallPath("/opt/games"); err != nil {
		t.Fatalf("SetInstallPath() error = %v", err)
	}

	if got := mgr.GetInstallPath(); got != "/opt/games" {
		t.Errorf("GetInstallPath() = %q, want %q", got, "/opt/games")
	}
}

func TestAddAuthorizedHub(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	mgr, err := NewManager()
	if err != nil {
		t.Fatalf("NewManager() error = %v", err)
	}

	hub := AuthorizedHub{
		ID:    "hub-1",
		Name:  "Test Hub",
		Token: "secret-token",
	}

	if err := mgr.AddAuthorizedHub(hub); err != nil {
		t.Fatalf("AddAuthorizedHub() error = %v", err)
	}

	hubs := mgr.GetAuthorizedHubs()
	if len(hubs) != 1 {
		t.Fatalf("GetAuthorizedHubs() len = %d, want 1", len(hubs))
	}
	if hubs[0].ID != "hub-1" {
		t.Errorf("hub.ID = %q, want %q", hubs[0].ID, "hub-1")
	}
}

func TestAddAuthorizedHub_UpdateExisting(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	mgr, err := NewManager()
	if err != nil {
		t.Fatalf("NewManager() error = %v", err)
	}

	hub := AuthorizedHub{ID: "hub-1", Name: "Hub Old", Token: "token-old"}
	mgr.AddAuthorizedHub(hub)

	// Update with same ID
	hub2 := AuthorizedHub{ID: "hub-1", Name: "Hub New", Token: "token-new"}
	if err := mgr.AddAuthorizedHub(hub2); err != nil {
		t.Fatalf("AddAuthorizedHub() update error = %v", err)
	}

	hubs := mgr.GetAuthorizedHubs()
	if len(hubs) != 1 {
		t.Fatalf("expected 1 hub after update, got %d", len(hubs))
	}
	if hubs[0].Name != "Hub New" {
		t.Errorf("hub.Name = %q, want %q", hubs[0].Name, "Hub New")
	}
}

func TestRemoveAuthorizedHub(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	mgr, err := NewManager()
	if err != nil {
		t.Fatalf("NewManager() error = %v", err)
	}

	mgr.AddAuthorizedHub(AuthorizedHub{ID: "hub-1", Name: "Hub 1"})
	mgr.AddAuthorizedHub(AuthorizedHub{ID: "hub-2", Name: "Hub 2"})

	if err := mgr.RemoveAuthorizedHub("hub-1"); err != nil {
		t.Fatalf("RemoveAuthorizedHub() error = %v", err)
	}

	hubs := mgr.GetAuthorizedHubs()
	if len(hubs) != 1 {
		t.Fatalf("GetAuthorizedHubs() len = %d, want 1", len(hubs))
	}
	if hubs[0].ID != "hub-2" {
		t.Errorf("remaining hub ID = %q, want %q", hubs[0].ID, "hub-2")
	}
}

func TestRemoveAuthorizedHub_NotFound(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	mgr, err := NewManager()
	if err != nil {
		t.Fatalf("NewManager() error = %v", err)
	}

	// Removing non-existent hub should not error
	if err := mgr.RemoveAuthorizedHub("nonexistent"); err != nil {
		t.Errorf("RemoveAuthorizedHub() error = %v, want nil", err)
	}
}

func TestSaveAndReload(t *testing.T) {
	tmpDir := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", tmpDir)

	mgr1, err := NewManager()
	if err != nil {
		t.Fatalf("NewManager() error = %v", err)
	}

	mgr1.SetName("persistent-agent")
	mgr1.SetInstallPath("/data/games")
	mgr1.AddAuthorizedHub(AuthorizedHub{ID: "hub-x", Name: "Hub X", Token: "tok"})

	// Create a new manager that loads the saved config
	mgr2, err := NewManager()
	if err != nil {
		t.Fatalf("NewManager() reload error = %v", err)
	}

	if got := mgr2.GetName(); got != "persistent-agent" {
		t.Errorf("reloaded name = %q, want %q", got, "persistent-agent")
	}
	if got := mgr2.GetInstallPath(); got != "/data/games" {
		t.Errorf("reloaded installPath = %q, want %q", got, "/data/games")
	}
	hubs := mgr2.GetAuthorizedHubs()
	if len(hubs) != 1 || hubs[0].ID != "hub-x" {
		t.Errorf("reloaded hubs = %v, want [{ID:hub-x}]", hubs)
	}
}

func TestLoad_CorruptFile(t *testing.T) {
	tmpDir := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", tmpDir)

	// Write corrupt data to the config file
	configDir := filepath.Join(tmpDir, "capydeploy-agent")
	os.MkdirAll(configDir, 0755)
	os.WriteFile(filepath.Join(configDir, "config.json"), []byte("{invalid json!"), 0600)

	// Should fall back to defaults without error
	mgr, err := NewManager()
	if err != nil {
		t.Fatalf("NewManager() error = %v", err)
	}

	// Should have default values (not empty)
	if mgr.GetName() == "" {
		t.Error("GetName() should have default value after corrupt config")
	}
}

func TestGetConfig(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	mgr, err := NewManager()
	if err != nil {
		t.Fatalf("NewManager() error = %v", err)
	}

	mgr.SetName("test-agent")

	cfg := mgr.GetConfig()
	if cfg.Name != "test-agent" {
		t.Errorf("GetConfig().Name = %q, want %q", cfg.Name, "test-agent")
	}
}

func TestUpdateHubLastSeen(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	mgr, err := NewManager()
	if err != nil {
		t.Fatalf("NewManager() error = %v", err)
	}

	mgr.AddAuthorizedHub(AuthorizedHub{ID: "hub-1", Name: "Hub"})

	if err := mgr.UpdateHubLastSeen("hub-1", "2025-01-15T10:30:00Z"); err != nil {
		t.Fatalf("UpdateHubLastSeen() error = %v", err)
	}

	hubs := mgr.GetAuthorizedHubs()
	if len(hubs) != 1 || hubs[0].LastSeen != "2025-01-15T10:30:00Z" {
		t.Errorf("hub LastSeen = %q, want %q", hubs[0].LastSeen, "2025-01-15T10:30:00Z")
	}
}

func TestSave_FilePermissions(t *testing.T) {
	tmpDir := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", tmpDir)

	mgr, err := NewManager()
	if err != nil {
		t.Fatalf("NewManager() error = %v", err)
	}

	mgr.SetName("perm-test")

	configPath := filepath.Join(tmpDir, "capydeploy-agent", "config.json")
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

	mgr, err := NewManager()
	if err != nil {
		t.Fatalf("NewManager() error = %v", err)
	}

	mgr.SetName("json-test")

	configPath := filepath.Join(tmpDir, "capydeploy-agent", "config.json")
	data, err := os.ReadFile(configPath)
	if err != nil {
		t.Fatalf("failed to read config: %v", err)
	}

	var cfg Config
	if err := json.Unmarshal(data, &cfg); err != nil {
		t.Fatalf("config is not valid JSON: %v", err)
	}

	if cfg.Name != "json-test" {
		t.Errorf("saved name = %q, want %q", cfg.Name, "json-test")
	}
}
