package auth

import (
	"os"
	"path/filepath"
	"testing"
)

func TestNewTokenStore(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	store, err := NewTokenStore()
	if err != nil {
		t.Fatalf("NewTokenStore() error = %v", err)
	}

	if store.GetHubID() == "" {
		t.Error("GetHubID() returned empty string")
	}
}

func TestGetHubID_Stable(t *testing.T) {
	tmpDir := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", tmpDir)

	store1, _ := NewTokenStore()
	id1 := store1.GetHubID()

	// Reload
	store2, _ := NewTokenStore()
	id2 := store2.GetHubID()

	if id1 != id2 {
		t.Errorf("Hub IDs should persist: %q != %q", id1, id2)
	}
}

func TestSaveToken(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	store, _ := NewTokenStore()

	if err := store.SaveToken("agent-1", "my-secret-token"); err != nil {
		t.Fatalf("SaveToken() error = %v", err)
	}

	if got := store.GetToken("agent-1"); got != "my-secret-token" {
		t.Errorf("GetToken() = %q, want %q", got, "my-secret-token")
	}
}

func TestGetToken_NotFound(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	store, _ := NewTokenStore()

	if got := store.GetToken("nonexistent"); got != "" {
		t.Errorf("GetToken() for unknown agent = %q, want empty", got)
	}
}

func TestRemoveToken(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	store, _ := NewTokenStore()
	store.SaveToken("agent-1", "token-1")

	if err := store.RemoveToken("agent-1"); err != nil {
		t.Fatalf("RemoveToken() error = %v", err)
	}

	if got := store.GetToken("agent-1"); got != "" {
		t.Errorf("GetToken() after remove = %q, want empty", got)
	}
}

func TestHasToken(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	store, _ := NewTokenStore()

	if store.HasToken("agent-1") {
		t.Error("HasToken() = true before saving")
	}

	store.SaveToken("agent-1", "token")

	if !store.HasToken("agent-1") {
		t.Error("HasToken() = false after saving")
	}
}

func TestPersistence(t *testing.T) {
	tmpDir := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", tmpDir)

	store1, _ := NewTokenStore()
	store1.SaveToken("agent-1", "token-persist")
	store1.SaveToken("agent-2", "token-persist-2")

	// Reload from disk
	store2, _ := NewTokenStore()

	if got := store2.GetToken("agent-1"); got != "token-persist" {
		t.Errorf("persisted token = %q, want %q", got, "token-persist")
	}
	if got := store2.GetToken("agent-2"); got != "token-persist-2" {
		t.Errorf("persisted token 2 = %q, want %q", got, "token-persist-2")
	}
}

func TestSave_FilePermissions(t *testing.T) {
	tmpDir := t.TempDir()
	t.Setenv("XDG_CONFIG_HOME", tmpDir)

	store, _ := NewTokenStore()
	store.SaveToken("agent-1", "token")

	tokensPath := filepath.Join(tmpDir, "capydeploy-hub", "tokens.json")
	info, err := os.Stat(tokensPath)
	if err != nil {
		t.Fatalf("tokens file not found: %v", err)
	}

	perm := info.Mode().Perm()
	if perm != 0600 {
		t.Errorf("tokens file permissions = %o, want 0600", perm)
	}
}
