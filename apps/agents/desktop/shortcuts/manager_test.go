package shortcuts

import (
	"os"
	"path/filepath"
	"runtime"
	"testing"

	"github.com/lobinuxsoft/capydeploy/pkg/steam"
	"github.com/shadowblip/steam-shortcut-manager/pkg/shortcut"
)

// --- Pure function tests ---

func TestTagsToSlice(t *testing.T) {
	tests := []struct {
		name string
		tags map[string]interface{}
		want int // expected length (order is non-deterministic from map)
	}{
		{"nil map", nil, 0},
		{"empty map", map[string]interface{}{}, 0},
		{"single tag", map[string]interface{}{"0": "RPG"}, 1},
		{"multiple tags", map[string]interface{}{"0": "RPG", "1": "Action"}, 2},
		{"non-string values ignored", map[string]interface{}{"0": "RPG", "1": 42}, 1},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := tagsToSlice(tt.tags)
			if tt.tags == nil {
				if got != nil {
					t.Errorf("tagsToSlice(nil) = %v, want nil", got)
				}
				return
			}
			if len(got) != tt.want {
				t.Errorf("tagsToSlice() len = %d, want %d", len(got), tt.want)
			}
		})
	}
}

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

// saveTestShortcut writes a shortcut directly to VDF for test setup
// (bypassing Create which now requires CEF).
func saveTestShortcut(t *testing.T, paths *steam.Paths, userID string, name, exe, startDir string, appID int64) {
	t.Helper()
	shortcutsPath := paths.ShortcutsPath(userID)
	configDir := paths.ConfigDir(userID)

	if err := os.MkdirAll(configDir, 0755); err != nil {
		t.Fatalf("failed to create config dir: %v", err)
	}

	var shortcuts *shortcut.Shortcuts
	if _, err := os.Stat(shortcutsPath); os.IsNotExist(err) {
		shortcuts = shortcut.NewShortcuts()
	} else {
		var loadErr error
		shortcuts, loadErr = shortcut.Load(shortcutsPath)
		if loadErr != nil {
			t.Fatalf("failed to load shortcuts: %v", loadErr)
		}
	}

	sc := shortcut.NewShortcut(name, exe, shortcut.DefaultShortcut)
	sc.StartDir = startDir
	sc.Appid = appID
	if err := shortcuts.Add(sc); err != nil {
		t.Fatalf("failed to add test shortcut: %v", err)
	}
	if err := shortcut.Save(shortcuts, shortcutsPath); err != nil {
		t.Fatalf("failed to save test shortcuts: %v", err)
	}
}

func TestManager_ListEmpty(t *testing.T) {
	tmpDir := t.TempDir()
	paths := steam.NewPathsWithBase(tmpDir)
	mgr := NewManagerWithPaths(paths)

	// No shortcuts.vdf exists yet
	list, err := mgr.List("12345")
	if err != nil {
		t.Fatalf("List() error = %v", err)
	}
	if len(list) != 0 {
		t.Errorf("List() returned %d shortcuts, want 0", len(list))
	}
}

func TestManager_ListVDFFallback(t *testing.T) {
	// Without CEF available, List() falls back to reading the VDF file.
	tmpDir := t.TempDir()
	paths := steam.NewPathsWithBase(tmpDir)
	mgr := NewManagerWithPaths(paths)
	userID := "12345"

	saveTestShortcut(t, paths, userID, "Test Game", "/usr/bin/test-game", "/usr/bin", 12345)

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

func TestManager_Create_RequiresCEF(t *testing.T) {
	t.Skip("requires Steam CEF debugger — integration test")
}

func TestManager_Delete_RequiresCEF(t *testing.T) {
	t.Skip("requires Steam CEF debugger — integration test")
}

func TestManager_DeleteNotFound_RequiresCEF(t *testing.T) {
	t.Skip("requires Steam CEF debugger — integration test")
}
