package shortcuts

import (
	"encoding/binary"
	"encoding/json"
	"os"
	"path/filepath"
	"runtime"
	"testing"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/steam"
)

// --- Pure function tests ---

func TestExpandPath(t *testing.T) {
	home, err := os.UserHomeDir()
	if err != nil {
		t.Skip("cannot determine home directory")
	}

	tests := []struct {
		name string
		path string
		want string
	}{
		{"absolute path unchanged", "/usr/bin/game", "/usr/bin/game"},
		{"relative path unchanged", "games/test", "games/test"},
		{"tilde expands", "~/Games/test", filepath.Join(home, "Games/test")},
		{"tilde alone not expanded", "~", "~"},
		{"tilde in middle not expanded", "/home/~user/games", "/home/~user/games"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := expandPath(tt.path); got != tt.want {
				t.Errorf("expandPath(%q) = %q, want %q", tt.path, got, tt.want)
			}
		})
	}
}

func TestQuotePath(t *testing.T) {
	tests := []struct {
		name string
		path string
		want string
	}{
		{"simple path", "/usr/bin/game", "/usr/bin/game"},
		{"already quoted", `"/usr/bin/game"`, "/usr/bin/game"},
	}

	if runtime.GOOS == "windows" {
		// On Windows, quotePath adds quotes
		tests = []struct {
			name string
			path string
			want string
		}{
			{"simple path", `C:\Games\test`, `"C:\Games\test"`},
			{"already quoted", `"C:\Games\test"`, `"C:\Games\test"`},
		}
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := quotePath(tt.path); got != tt.want {
				t.Errorf("quotePath(%q) = %q, want %q", tt.path, got, tt.want)
			}
		})
	}
}

func TestUnquotePath(t *testing.T) {
	tests := []struct {
		name string
		path string
		want string
	}{
		{"unquoted path", "/usr/bin/game", "/usr/bin/game"},
		{"quoted path", `"/usr/bin/game"`, "/usr/bin/game"},
		{"single quote not removed", `'/usr/bin/game'`, `'/usr/bin/game'`},
		{"empty string", "", ""},
		{"only quotes", `""`, ""},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := unquotePath(tt.path); got != tt.want {
				t.Errorf("unquotePath(%q) = %q, want %q", tt.path, got, tt.want)
			}
		})
	}
}

func TestIsSubPath(t *testing.T) {
	tests := []struct {
		name   string
		parent string
		child  string
		want   bool
	}{
		{"child inside parent", "/home/user", "/home/user/games/test", true},
		{"child is parent", "/home/user", "/home/user", false},
		{"child outside parent", "/home/user", "/etc/config", false},
		{"sibling directory", "/home/user", "/home/user2/games", false},
		{"root as parent", "/", "/home/user", true},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := isSubPath(tt.parent, tt.child); got != tt.want {
				t.Errorf("isSubPath(%q, %q) = %v, want %v", tt.parent, tt.child, got, tt.want)
			}
		})
	}
}

func TestDeleteGameDirectory_Safety(t *testing.T) {
	home, err := os.UserHomeDir()
	if err != nil {
		t.Skip("cannot determine home directory")
	}

	tests := []struct {
		name    string
		path    string
		wantErr bool
	}{
		{"empty path is no-op", "", false},
		{"root path rejected", "/", true},
		{"home dir rejected", home, true},
		{"top-level home subdir rejected", filepath.Join(home, "Games"), true},
		{"system path rejected", "/etc", true},
		{"nonexistent deep path is no-op", filepath.Join(home, "Games", "nonexistent_test_dir_12345"), false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := deleteGameDirectory(tt.path)
			if (err != nil) != tt.wantErr {
				t.Errorf("deleteGameDirectory(%q) error = %v, wantErr %v", tt.path, err, tt.wantErr)
			}
		})
	}
}

func TestDeleteGameDirectory_ActualDelete(t *testing.T) {
	home, err := os.UserHomeDir()
	if err != nil {
		t.Skip("cannot determine home directory")
	}

	// Create a temp dir inside home at 2+ levels deep
	tmpBase := filepath.Join(home, ".capydeploy-test")
	gameDir := filepath.Join(tmpBase, "test-game")
	if err := os.MkdirAll(gameDir, 0755); err != nil {
		t.Fatalf("failed to create test dir: %v", err)
	}
	defer os.RemoveAll(tmpBase)

	// Create a file inside
	testFile := filepath.Join(gameDir, "game.exe")
	if err := os.WriteFile(testFile, []byte("test"), 0644); err != nil {
		t.Fatalf("failed to create test file: %v", err)
	}

	if err := deleteGameDirectory(gameDir); err != nil {
		t.Errorf("deleteGameDirectory() error = %v", err)
	}

	if _, err := os.Stat(gameDir); !os.IsNotExist(err) {
		t.Errorf("deleteGameDirectory() did not remove directory")
	}
}

// --- Tracking tests ---

// writeTestVDF writes a minimal binary VDF shortcuts file for testing.
func writeTestVDF(t *testing.T, paths *steam.Paths, userID string, name, exe string, appID uint32) {
	t.Helper()
	configDir := paths.ConfigDir(userID)
	if err := os.MkdirAll(configDir, 0755); err != nil {
		t.Fatalf("failed to create config dir: %v", err)
	}

	var buf []byte
	// Root object
	buf = append(buf, 0x00)
	buf = append(buf, []byte("shortcuts")...)
	buf = append(buf, 0x00)
	// Single shortcut entry
	buf = append(buf, 0x00)
	buf = append(buf, []byte("0")...)
	buf = append(buf, 0x00)
	// appid
	buf = append(buf, 0x02)
	buf = append(buf, []byte("appid")...)
	buf = append(buf, 0x00)
	b := make([]byte, 4)
	binary.LittleEndian.PutUint32(b, appID)
	buf = append(buf, b...)
	// AppName
	buf = append(buf, 0x01)
	buf = append(buf, []byte("AppName")...)
	buf = append(buf, 0x00)
	buf = append(buf, []byte(name)...)
	buf = append(buf, 0x00)
	// Exe
	buf = append(buf, 0x01)
	buf = append(buf, []byte("Exe")...)
	buf = append(buf, 0x00)
	buf = append(buf, []byte(exe)...)
	buf = append(buf, 0x00)
	// End shortcut + end root
	buf = append(buf, 0x08, 0x08)

	if err := os.WriteFile(paths.ShortcutsPath(userID), buf, 0644); err != nil {
		t.Fatalf("failed to write test VDF: %v", err)
	}
}

// writeTestTracking writes a tracked shortcuts JSON file for testing.
func writeTestTracking(t *testing.T, trackingPath string, shortcuts []protocol.ShortcutInfo) {
	t.Helper()
	data, err := json.MarshalIndent(shortcuts, "", "  ")
	if err != nil {
		t.Fatalf("failed to marshal tracking data: %v", err)
	}
	if err := os.WriteFile(trackingPath, data, 0600); err != nil {
		t.Fatalf("failed to write tracking file: %v", err)
	}
}

func TestManager_ListEmpty(t *testing.T) {
	tmpDir := t.TempDir()
	paths := steam.NewPathsWithBase(tmpDir)
	trackingPath := filepath.Join(tmpDir, "tracked.json")
	mgr := NewManagerWithPaths(paths, trackingPath)

	// No VDF and no tracking file — seeds empty, returns empty
	list, err := mgr.List("12345")
	if err != nil {
		t.Fatalf("List() error = %v", err)
	}
	if len(list) != 0 {
		t.Errorf("List() returned %d shortcuts, want 0", len(list))
	}

	// Tracking file should have been created (empty array)
	if _, err := os.Stat(trackingPath); os.IsNotExist(err) {
		t.Error("tracking file should have been created after first List()")
	}
}

func TestManager_ListSeedsFromVDF(t *testing.T) {
	tmpDir := t.TempDir()
	paths := steam.NewPathsWithBase(tmpDir)
	trackingPath := filepath.Join(tmpDir, "tracked.json")
	userID := "12345"

	writeTestVDF(t, paths, userID, "Test Game", "/usr/bin/test-game", 12345)

	mgr := NewManagerWithPaths(paths, trackingPath)

	// First List() should seed from VDF
	list, err := mgr.List(userID)
	if err != nil {
		t.Fatalf("List() error = %v", err)
	}
	if len(list) != 1 {
		t.Fatalf("List() returned %d shortcuts, want 1", len(list))
	}
	if list[0].Name != "Test Game" {
		t.Errorf("List()[0].Name = %q, want %q", list[0].Name, "Test Game")
	}

	// Tracking file should now exist with the seeded data
	data, err := os.ReadFile(trackingPath)
	if err != nil {
		t.Fatalf("failed to read tracking file: %v", err)
	}
	var tracked []protocol.ShortcutInfo
	if err := json.Unmarshal(data, &tracked); err != nil {
		t.Fatalf("failed to parse tracking file: %v", err)
	}
	if len(tracked) != 1 || tracked[0].Name != "Test Game" {
		t.Errorf("tracking file has unexpected content: %+v", tracked)
	}
}

func TestManager_ListUsesTrackingOverVDF(t *testing.T) {
	tmpDir := t.TempDir()
	paths := steam.NewPathsWithBase(tmpDir)
	trackingPath := filepath.Join(tmpDir, "tracked.json")
	userID := "12345"

	// Write VDF with one game
	writeTestVDF(t, paths, userID, "VDF Game", "/vdf/game", 111)

	// Write tracking with a DIFFERENT game
	writeTestTracking(t, trackingPath, []protocol.ShortcutInfo{
		{AppID: 222, Name: "Tracked Game", Exe: "/tracked/game"},
	})

	mgr := NewManagerWithPaths(paths, trackingPath)

	// List() should return tracked data, NOT VDF data
	list, err := mgr.List(userID)
	if err != nil {
		t.Fatalf("List() error = %v", err)
	}
	if len(list) != 1 {
		t.Fatalf("List() returned %d shortcuts, want 1", len(list))
	}
	if list[0].Name != "Tracked Game" {
		t.Errorf("List()[0].Name = %q, want %q (tracking should take priority over VDF)", list[0].Name, "Tracked Game")
	}
}

func TestManager_TrackingPersistence(t *testing.T) {
	tmpDir := t.TempDir()
	paths := steam.NewPathsWithBase(tmpDir)
	trackingPath := filepath.Join(tmpDir, "tracked.json")

	// Write tracked data
	shortcuts := []protocol.ShortcutInfo{
		{AppID: 100, Name: "Game A", Exe: "/a", StartDir: "/dir-a"},
		{AppID: 200, Name: "Game B", Exe: "/b", StartDir: "/dir-b"},
	}
	writeTestTracking(t, trackingPath, shortcuts)

	// Create a new manager — should load persisted data
	mgr := NewManagerWithPaths(paths, trackingPath)
	list, err := mgr.List("99999")
	if err != nil {
		t.Fatalf("List() error = %v", err)
	}
	if len(list) != 2 {
		t.Fatalf("List() returned %d shortcuts, want 2", len(list))
	}
	if list[0].AppID != 100 || list[1].AppID != 200 {
		t.Errorf("unexpected shortcuts: %+v", list)
	}
}

func TestManager_ListReturnsCopy(t *testing.T) {
	tmpDir := t.TempDir()
	paths := steam.NewPathsWithBase(tmpDir)
	trackingPath := filepath.Join(tmpDir, "tracked.json")

	writeTestTracking(t, trackingPath, []protocol.ShortcutInfo{
		{AppID: 42, Name: "Original"},
	})

	mgr := NewManagerWithPaths(paths, trackingPath)
	list, _ := mgr.List("12345")

	// Mutating the returned slice should not affect internal state
	list[0].Name = "Mutated"
	list2, _ := mgr.List("12345")
	if list2[0].Name != "Original" {
		t.Errorf("List() returned reference to internal slice (mutation propagated)")
	}
}

// --- CEF-dependent tests (skipped in CI) ---

func TestManager_Create_RequiresCEF(t *testing.T) {
	t.Skip("requires Steam CEF debugger — integration test")
}

func TestManager_Delete_RequiresCEF(t *testing.T) {
	t.Skip("requires Steam CEF debugger — integration test")
}
