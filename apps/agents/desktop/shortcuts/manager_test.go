package shortcuts

import (
	"encoding/binary"
	"os"
	"path/filepath"
	"runtime"
	"testing"

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

// --- Manager tests ---

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

func TestManager_ListEmpty(t *testing.T) {
	tmpDir := t.TempDir()
	paths := steam.NewPathsWithBase(tmpDir)
	mgr := NewManagerWithPaths(paths)

	// No shortcuts.vdf exists yet — CEF will fail, VDF fallback returns empty
	list, err := mgr.List("12345")
	if err != nil {
		t.Fatalf("List() error = %v", err)
	}
	if len(list) != 0 {
		t.Errorf("List() returned %d shortcuts, want 0", len(list))
	}
}

func TestManager_ListVDFFallback(t *testing.T) {
	tmpDir := t.TempDir()
	paths := steam.NewPathsWithBase(tmpDir)
	mgr := NewManagerWithPaths(paths)
	userID := "12345"

	writeTestVDF(t, paths, userID, "Test Game", "/usr/bin/test-game", 12345)

	// Without CEF, List() falls back to VDF
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
}

func TestManager_List_RequiresCEF(t *testing.T) {
	t.Skip("requires Steam CEF debugger — integration test")
}

func TestManager_Create_RequiresCEF(t *testing.T) {
	t.Skip("requires Steam CEF debugger — integration test")
}

func TestManager_Delete_RequiresCEF(t *testing.T) {
	t.Skip("requires Steam CEF debugger — integration test")
}

func TestManager_DeleteNotFound_RequiresCEF(t *testing.T) {
	t.Skip("requires Steam CEF debugger — integration test")
}
