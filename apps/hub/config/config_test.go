package config

import (
	"encoding/json"
	"os"
	"path/filepath"
	"runtime"
	"testing"
)

func TestNewManager(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	mgr, err := NewManager()
	if err != nil {
		t.Fatalf("NewManager() error = %v", err)
	}

	if mgr.GetID() == "" {
		t.Error("GetID() returned empty string")
	}
	if mgr.GetName() == "" {
		t.Error("GetName() returned empty string")
	}
	if mgr.GetPlatform() == "" {
		t.Error("GetPlatform() returned empty string")
	}
}

func TestGetID_Stable(t *testing.T) {
	tmpDir := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", tmpDir)

	mgr1, _ := NewManager()
	id1 := mgr1.GetID()

	// Create another manager from the same config dir
	mgr2, _ := NewManager()
	id2 := mgr2.GetID()

	if id1 != id2 {
		t.Errorf("IDs should be stable: %q != %q", id1, id2)
	}
}

func TestGetID_Length(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	mgr, _ := NewManager()
	id := mgr.GetID()

	// ID is first 8 hex chars of SHA256
	if len(id) != 8 {
		t.Errorf("GetID() length = %d, want 8", len(id))
	}
}

func TestSetName(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	mgr, _ := NewManager()

	if err := mgr.SetName("my-hub"); err != nil {
		t.Fatalf("SetName() error = %v", err)
	}

	if got := mgr.GetName(); got != "my-hub" {
		t.Errorf("GetName() = %q, want %q", got, "my-hub")
	}
}

func TestGetPlatform(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	mgr, _ := NewManager()

	if got := mgr.GetPlatform(); got != runtime.GOOS {
		t.Errorf("GetPlatform() = %q, want %q", got, runtime.GOOS)
	}
}

func TestSaveAndReload(t *testing.T) {
	tmpDir := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", tmpDir)

	mgr1, _ := NewManager()
	originalID := mgr1.GetID()
	mgr1.SetName("persistent-hub")

	// Create new manager that reloads from disk
	mgr2, _ := NewManager()

	if got := mgr2.GetName(); got != "persistent-hub" {
		t.Errorf("reloaded name = %q, want %q", got, "persistent-hub")
	}
	if got := mgr2.GetID(); got != originalID {
		t.Errorf("reloaded ID = %q, want %q (should persist)", got, originalID)
	}
}

func TestGetConfig(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	mgr, _ := NewManager()
	mgr.SetName("config-test")

	cfg := mgr.GetConfig()
	if cfg.Name != "config-test" {
		t.Errorf("GetConfig().Name = %q, want %q", cfg.Name, "config-test")
	}
	if cfg.ID == "" {
		t.Error("GetConfig().ID is empty")
	}
	if cfg.Platform == "" {
		t.Error("GetConfig().Platform is empty")
	}
}

func TestSave_ValidJSON(t *testing.T) {
	tmpDir := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", tmpDir)

	mgr, _ := NewManager()
	mgr.SetName("json-hub")

	configPath := filepath.Join(tmpDir, "capydeploy-hub", "config.json")
	data, err := os.ReadFile(configPath)
	if err != nil {
		t.Fatalf("failed to read config: %v", err)
	}

	var cfg Config
	if err := json.Unmarshal(data, &cfg); err != nil {
		t.Fatalf("config is not valid JSON: %v", err)
	}

	if cfg.Name != "json-hub" {
		t.Errorf("saved name = %q, want %q", cfg.Name, "json-hub")
	}
}
