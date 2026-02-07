package shortcuts

import (
	"os"
	"path/filepath"
	"runtime"
	"testing"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/steam"
	"github.com/shadowblip/steam-shortcut-manager/pkg/shortcut"
)

// newTestShortcuts creates a Shortcuts struct with named shortcuts for testing.
func newTestShortcuts(entries map[string]string) *shortcut.Shortcuts {
	sc := shortcut.NewShortcuts()
	for key, name := range entries {
		sc.Shortcuts[key] = shortcut.Shortcut{AppName: name}
	}
	return sc
}

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

func TestSliceToTags(t *testing.T) {
	tests := []struct {
		name string
		tags []string
		want int
	}{
		{"nil slice", nil, 0},
		{"empty slice", []string{}, 0},
		{"single tag", []string{"RPG"}, 1},
		{"multiple tags", []string{"RPG", "Action", "Indie"}, 3},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := sliceToTags(tt.tags)
			if len(tt.tags) == 0 {
				if got != nil {
					t.Errorf("sliceToTags(%v) = %v, want nil", tt.tags, got)
				}
				return
			}
			if len(got) != tt.want {
				t.Errorf("sliceToTags() len = %d, want %d", len(got), tt.want)
			}
			// Verify values are accessible
			for i, tag := range tt.tags {
				key := string(rune('0') + rune(i))
				if i >= 10 {
					// sliceToTags uses strconv.Itoa, not rune math
					break
				}
				if got[key] != tag {
					t.Errorf("sliceToTags()[%q] = %v, want %q", key, got[key], tag)
				}
			}
		})
	}
}

func TestSliceToTagsRoundTrip(t *testing.T) {
	input := []string{"RPG", "Action", "Indie"}
	tags := sliceToTags(input)
	result := tagsToSlice(tags)

	if len(result) != len(input) {
		t.Fatalf("round trip: got %d tags, want %d", len(result), len(input))
	}

	// Since map iteration order is non-deterministic, check all input values are present
	have := make(map[string]bool)
	for _, v := range result {
		have[v] = true
	}
	for _, v := range input {
		if !have[v] {
			t.Errorf("round trip: missing tag %q", v)
		}
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

func TestReindexShortcuts(t *testing.T) {
	// Create a shortcut collection with non-sequential keys
	sc := newTestShortcuts(map[string]string{
		"5":  "GameA",
		"10": "GameB",
		"2":  "GameC",
	})

	result := reindexShortcuts(sc)

	if len(result.Shortcuts) != 3 {
		t.Fatalf("reindexShortcuts() returned %d shortcuts, want 3", len(result.Shortcuts))
	}

	// Keys should be "0", "1", "2"
	for i := 0; i < 3; i++ {
		key := string(rune('0') + rune(i))
		if _, ok := result.Shortcuts[key]; !ok {
			t.Errorf("reindexShortcuts(): missing key %q", key)
		}
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

// --- Manager CRUD tests with VDF library ---

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

func TestManager_CreateAndList(t *testing.T) {
	tmpDir := t.TempDir()
	paths := steam.NewPathsWithBase(tmpDir)
	mgr := NewManagerWithPaths(paths)

	userID := "12345"

	// Ensure config dir exists for the user
	configDir := paths.ConfigDir(userID)
	if err := os.MkdirAll(configDir, 0755); err != nil {
		t.Fatalf("failed to create config dir: %v", err)
	}

	cfg := protocol.ShortcutConfig{
		Name:          "Test Game",
		Exe:           "/usr/bin/test-game",
		StartDir:      "/usr/bin",
		LaunchOptions: "--fullscreen",
		Tags:          []string{"RPG", "Action"},
	}

	appID, err := mgr.Create(userID, cfg)
	if err != nil {
		t.Fatalf("Create() error = %v", err)
	}
	if appID == 0 {
		t.Error("Create() returned appID 0")
	}

	// List should return the shortcut
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
	if list[0].AppID != appID {
		t.Errorf("List()[0].AppID = %d, want %d", list[0].AppID, appID)
	}
}

func TestManager_CreateDuplicate(t *testing.T) {
	tmpDir := t.TempDir()
	paths := steam.NewPathsWithBase(tmpDir)
	mgr := NewManagerWithPaths(paths)

	userID := "12345"
	if err := os.MkdirAll(paths.ConfigDir(userID), 0755); err != nil {
		t.Fatalf("failed to create config dir: %v", err)
	}

	cfg := protocol.ShortcutConfig{
		Name: "Test Game",
		Exe:  "/usr/bin/test-game",
	}

	if _, err := mgr.Create(userID, cfg); err != nil {
		t.Fatalf("first Create() error = %v", err)
	}

	// Second create with same exe+name should fail
	_, err := mgr.Create(userID, cfg)
	if err == nil {
		t.Error("second Create() should return error for duplicate shortcut")
	}
}

func TestManager_Delete(t *testing.T) {
	tmpDir := t.TempDir()
	paths := steam.NewPathsWithBase(tmpDir)
	mgr := NewManagerWithPaths(paths)

	userID := "12345"
	if err := os.MkdirAll(paths.ConfigDir(userID), 0755); err != nil {
		t.Fatalf("failed to create config dir: %v", err)
	}

	cfg := protocol.ShortcutConfig{
		Name:     "Test Game",
		Exe:      "/usr/bin/test-game",
		StartDir: "/usr/bin",
	}

	appID, err := mgr.Create(userID, cfg)
	if err != nil {
		t.Fatalf("Create() error = %v", err)
	}

	// Delete without cleanup to avoid filesystem side effects
	if err := mgr.DeleteWithCleanup(userID, appID, "", false); err != nil {
		t.Fatalf("DeleteWithCleanup() error = %v", err)
	}

	// List should be empty
	list, err := mgr.List(userID)
	if err != nil {
		t.Fatalf("List() error = %v", err)
	}
	if len(list) != 0 {
		t.Errorf("List() returned %d shortcuts after delete, want 0", len(list))
	}
}

func TestManager_DeleteByName(t *testing.T) {
	tmpDir := t.TempDir()
	paths := steam.NewPathsWithBase(tmpDir)
	mgr := NewManagerWithPaths(paths)

	userID := "12345"
	if err := os.MkdirAll(paths.ConfigDir(userID), 0755); err != nil {
		t.Fatalf("failed to create config dir: %v", err)
	}

	cfg := protocol.ShortcutConfig{
		Name:     "My Game",
		Exe:      "/usr/bin/my-game",
		StartDir: "/usr/bin",
	}

	if _, err := mgr.Create(userID, cfg); err != nil {
		t.Fatalf("Create() error = %v", err)
	}

	// Delete by name (appID=0)
	if err := mgr.DeleteWithCleanup(userID, 0, "My Game", false); err != nil {
		t.Fatalf("DeleteWithCleanup() by name error = %v", err)
	}

	list, err := mgr.List(userID)
	if err != nil {
		t.Fatalf("List() error = %v", err)
	}
	if len(list) != 0 {
		t.Errorf("List() returned %d shortcuts after delete by name, want 0", len(list))
	}
}

func TestManager_DeleteNotFound(t *testing.T) {
	tmpDir := t.TempDir()
	paths := steam.NewPathsWithBase(tmpDir)
	mgr := NewManagerWithPaths(paths)

	userID := "12345"
	if err := os.MkdirAll(paths.ConfigDir(userID), 0755); err != nil {
		t.Fatalf("failed to create config dir: %v", err)
	}

	// Create one shortcut so shortcuts.vdf exists
	cfg := protocol.ShortcutConfig{
		Name:     "Existing Game",
		Exe:      "/usr/bin/existing",
		StartDir: "/usr/bin",
	}
	if _, err := mgr.Create(userID, cfg); err != nil {
		t.Fatalf("Create() error = %v", err)
	}

	// Try to delete a non-existent shortcut
	err := mgr.DeleteWithCleanup(userID, 99999, "", false)
	if err == nil {
		t.Error("DeleteWithCleanup() should return error for non-existent shortcut")
	}
}
