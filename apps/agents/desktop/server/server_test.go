package server

import (
	"path/filepath"
	"testing"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/transfer"
)

func TestNew_Defaults(t *testing.T) {
	tmpDir := t.TempDir()

	srv, err := New(Config{
		Name:       "test-agent",
		Platform:   "linux",
		Version:    "0.1.0",
		UploadPath: tmpDir,
	})
	if err != nil {
		t.Fatalf("New() error = %v", err)
	}

	if srv.id == "" {
		t.Error("New() server ID is empty")
	}

	if srv.cfg.Name != "test-agent" {
		t.Errorf("server name = %q, want %q", srv.cfg.Name, "test-agent")
	}
}

func TestNew_StableID(t *testing.T) {
	tmpDir := t.TempDir()

	srv1, _ := New(Config{Name: "agent-a", Platform: "linux", UploadPath: tmpDir})
	srv2, _ := New(Config{Name: "agent-a", Platform: "linux", UploadPath: tmpDir})
	srv3, _ := New(Config{Name: "agent-b", Platform: "linux", UploadPath: tmpDir})

	if srv1.id != srv2.id {
		t.Errorf("same config should produce same ID: %q != %q", srv1.id, srv2.id)
	}
	if srv1.id == srv3.id {
		t.Errorf("different names should produce different IDs: %q == %q", srv1.id, srv3.id)
	}
}

func TestGetInfo(t *testing.T) {
	tmpDir := t.TempDir()

	srv, _ := New(Config{
		Name:       "test-agent",
		Platform:   "linux",
		Version:    "1.2.3",
		UploadPath: tmpDir,
	})

	info := srv.GetInfo()

	if info.Name != "test-agent" {
		t.Errorf("info.Name = %q, want %q", info.Name, "test-agent")
	}
	if info.Platform != "linux" {
		t.Errorf("info.Platform = %q, want %q", info.Platform, "linux")
	}
	if info.Version != "1.2.3" {
		t.Errorf("info.Version = %q, want %q", info.Version, "1.2.3")
	}
	if !info.AcceptConnections {
		t.Error("info.AcceptConnections = false, want true (default)")
	}
	if len(info.SupportedImageFormats) == 0 {
		t.Error("info.SupportedImageFormats is empty")
	}
}

func TestGetInfo_AcceptConnectionsCallback(t *testing.T) {
	tmpDir := t.TempDir()

	srv, _ := New(Config{
		Name:              "test-agent",
		Platform:          "linux",
		UploadPath:        tmpDir,
		AcceptConnections: func() bool { return false },
	})

	info := srv.GetInfo()
	if info.AcceptConnections {
		t.Error("info.AcceptConnections = true, want false")
	}
}

func TestGetUploadPath(t *testing.T) {
	tmpDir := t.TempDir()

	srv, _ := New(Config{
		Name:       "test",
		Platform:   "linux",
		UploadPath: tmpDir,
	})

	got := srv.GetUploadPath("MyGame", "")
	want := filepath.Join(tmpDir, "MyGame")
	if got != want {
		t.Errorf("GetUploadPath() = %q, want %q", got, want)
	}
}

func TestGetUploadPath_WithCallback(t *testing.T) {
	tmpDir := t.TempDir()
	customPath := filepath.Join(tmpDir, "custom")

	srv, _ := New(Config{
		Name:           "test",
		Platform:       "linux",
		UploadPath:     tmpDir,
		GetInstallPath: func() string { return customPath },
	})

	got := srv.GetUploadPath("MyGame", "")
	want := filepath.Join(customPath, "MyGame")
	if got != want {
		t.Errorf("GetUploadPath() = %q, want %q", got, want)
	}
}

func TestCreateAndGetUpload(t *testing.T) {
	tmpDir := t.TempDir()

	srv, _ := New(Config{
		Name:       "test",
		Platform:   "linux",
		UploadPath: tmpDir,
	})

	cfg := protocol.UploadConfig{
		GameName: "Test Game",
	}
	files := []transfer.FileEntry{
		{RelativePath: "game.exe", Size: 1024},
	}

	session := srv.CreateUpload(cfg, 1024, files)
	if session == nil {
		t.Fatal("CreateUpload() returned nil")
	}
	if session.ID == "" {
		t.Error("session.ID is empty")
	}

	// Get the upload
	got, ok := srv.GetUpload(session.ID)
	if !ok {
		t.Fatal("GetUpload() not found")
	}
	if got.ID != session.ID {
		t.Errorf("GetUpload().ID = %q, want %q", got.ID, session.ID)
	}
}

func TestGetUpload_NotFound(t *testing.T) {
	tmpDir := t.TempDir()

	srv, _ := New(Config{
		Name:       "test",
		Platform:   "linux",
		UploadPath: tmpDir,
	})

	_, ok := srv.GetUpload("nonexistent")
	if ok {
		t.Error("GetUpload() found a nonexistent upload")
	}
}

func TestDeleteUpload(t *testing.T) {
	tmpDir := t.TempDir()

	srv, _ := New(Config{
		Name:       "test",
		Platform:   "linux",
		UploadPath: tmpDir,
	})

	session := srv.CreateUpload(protocol.UploadConfig{GameName: "Game"}, 100, nil)

	srv.DeleteUpload(session.ID)

	_, ok := srv.GetUpload(session.ID)
	if ok {
		t.Error("GetUpload() still found upload after DeleteUpload()")
	}
}

func TestNotifyShortcutChange(t *testing.T) {
	tmpDir := t.TempDir()
	called := false

	srv, _ := New(Config{
		Name:             "test",
		Platform:         "linux",
		UploadPath:       tmpDir,
		OnShortcutChange: func() { called = true },
	})

	srv.NotifyShortcutChange()

	if !called {
		t.Error("OnShortcutChange callback was not called")
	}
}

func TestNotifyShortcutChange_NilCallback(t *testing.T) {
	tmpDir := t.TempDir()

	srv, _ := New(Config{
		Name:       "test",
		Platform:   "linux",
		UploadPath: tmpDir,
	})

	// Should not panic
	srv.NotifyShortcutChange()
}

func TestNotifyOperation(t *testing.T) {
	tmpDir := t.TempDir()
	var received OperationEvent

	srv, _ := New(Config{
		Name:       "test",
		Platform:   "linux",
		UploadPath: tmpDir,
		OnOperation: func(event OperationEvent) {
			received = event
		},
	})

	srv.NotifyOperation("install", "progress", "Test Game", 50.0, "downloading")

	if received.Type != "install" {
		t.Errorf("event.Type = %q, want %q", received.Type, "install")
	}
	if received.Status != "progress" {
		t.Errorf("event.Status = %q, want %q", received.Status, "progress")
	}
	if received.GameName != "Test Game" {
		t.Errorf("event.GameName = %q, want %q", received.GameName, "Test Game")
	}
	if received.Progress != 50.0 {
		t.Errorf("event.Progress = %f, want %f", received.Progress, 50.0)
	}
}

func TestNotifyOperation_NilCallback(t *testing.T) {
	tmpDir := t.TempDir()

	srv, _ := New(Config{
		Name:       "test",
		Platform:   "linux",
		UploadPath: tmpDir,
	})

	// Should not panic
	srv.NotifyOperation("install", "start", "Game", 0, "")
}
