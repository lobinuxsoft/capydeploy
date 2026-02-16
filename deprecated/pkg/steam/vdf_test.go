package steam

import (
	"encoding/binary"
	"os"
	"path/filepath"
	"testing"
)

// buildTestVDF constructs a minimal binary VDF shortcuts file for testing.
func buildTestVDF(shortcuts []testShortcut) []byte {
	var buf []byte

	// Root: \x00 "shortcuts" \x00
	buf = append(buf, vdfTypeObject)
	buf = append(buf, []byte("shortcuts")...)
	buf = append(buf, 0x00)

	for i, sc := range shortcuts {
		// Shortcut entry: \x00 "<index>" \x00
		buf = append(buf, vdfTypeObject)
		buf = append(buf, []byte(string(rune('0'+i)))...)
		buf = append(buf, 0x00)

		// appid (int32)
		buf = append(buf, vdfTypeInt32)
		buf = append(buf, []byte("appid")...)
		buf = append(buf, 0x00)
		b := make([]byte, 4)
		binary.LittleEndian.PutUint32(b, sc.appID)
		buf = append(buf, b...)

		// AppName (string)
		buf = append(buf, vdfTypeString)
		buf = append(buf, []byte("AppName")...)
		buf = append(buf, 0x00)
		buf = append(buf, []byte(sc.name)...)
		buf = append(buf, 0x00)

		// Exe (string)
		buf = append(buf, vdfTypeString)
		buf = append(buf, []byte("Exe")...)
		buf = append(buf, 0x00)
		buf = append(buf, []byte(sc.exe)...)
		buf = append(buf, 0x00)

		// StartDir (string)
		buf = append(buf, vdfTypeString)
		buf = append(buf, []byte("StartDir")...)
		buf = append(buf, 0x00)
		buf = append(buf, []byte(sc.startDir)...)
		buf = append(buf, 0x00)

		// LaunchOptions (string)
		buf = append(buf, vdfTypeString)
		buf = append(buf, []byte("LaunchOptions")...)
		buf = append(buf, 0x00)
		buf = append(buf, []byte(sc.launchOptions)...)
		buf = append(buf, 0x00)

		// LastPlayTime (int32)
		buf = append(buf, vdfTypeInt32)
		buf = append(buf, []byte("LastPlayTime")...)
		buf = append(buf, 0x00)
		b2 := make([]byte, 4)
		binary.LittleEndian.PutUint32(b2, uint32(sc.lastPlayed))
		buf = append(buf, b2...)

		// Tags (nested object)
		if len(sc.tags) > 0 {
			buf = append(buf, vdfTypeObject)
			buf = append(buf, []byte("tags")...)
			buf = append(buf, 0x00)
			for j, tag := range sc.tags {
				buf = append(buf, vdfTypeString)
				buf = append(buf, []byte(string(rune('0'+j)))...)
				buf = append(buf, 0x00)
				buf = append(buf, []byte(tag)...)
				buf = append(buf, 0x00)
			}
			buf = append(buf, vdfTypeEnd) // end tags
		}

		buf = append(buf, vdfTypeEnd) // end shortcut
	}

	buf = append(buf, vdfTypeEnd) // end shortcuts root

	return buf
}

type testShortcut struct {
	appID         uint32
	name          string
	exe           string
	startDir      string
	launchOptions string
	lastPlayed    int64
	tags          []string
}

func TestParseShortcutsVDF_SingleEntry(t *testing.T) {
	data := buildTestVDF([]testShortcut{
		{
			appID:         12345,
			name:          "Test Game",
			exe:           "/usr/bin/test-game",
			startDir:      "/usr/bin",
			launchOptions: "-fullscreen",
			lastPlayed:    1700000000,
			tags:          []string{"RPG", "Action"},
		},
	})

	shortcuts, err := parseShortcutsVDF(data)
	if err != nil {
		t.Fatalf("parseShortcutsVDF() error = %v", err)
	}

	if len(shortcuts) != 1 {
		t.Fatalf("expected 1 shortcut, got %d", len(shortcuts))
	}

	sc := shortcuts[0]
	if sc.AppID != 12345 {
		t.Errorf("AppID = %d, want 12345", sc.AppID)
	}
	if sc.Name != "Test Game" {
		t.Errorf("Name = %q, want %q", sc.Name, "Test Game")
	}
	if sc.Exe != "/usr/bin/test-game" {
		t.Errorf("Exe = %q, want %q", sc.Exe, "/usr/bin/test-game")
	}
	if sc.StartDir != "/usr/bin" {
		t.Errorf("StartDir = %q, want %q", sc.StartDir, "/usr/bin")
	}
	if sc.LaunchOptions != "-fullscreen" {
		t.Errorf("LaunchOptions = %q, want %q", sc.LaunchOptions, "-fullscreen")
	}
	if sc.LastPlayed != 1700000000 {
		t.Errorf("LastPlayed = %d, want 1700000000", sc.LastPlayed)
	}
	if len(sc.Tags) != 2 || sc.Tags[0] != "RPG" || sc.Tags[1] != "Action" {
		t.Errorf("Tags = %v, want [RPG, Action]", sc.Tags)
	}
}

func TestParseShortcutsVDF_MultipleEntries(t *testing.T) {
	data := buildTestVDF([]testShortcut{
		{appID: 111, name: "Game A", exe: "/a"},
		{appID: 222, name: "Game B", exe: "/b"},
		{appID: 333, name: "Game C", exe: "/c"},
	})

	shortcuts, err := parseShortcutsVDF(data)
	if err != nil {
		t.Fatalf("parseShortcutsVDF() error = %v", err)
	}

	if len(shortcuts) != 3 {
		t.Fatalf("expected 3 shortcuts, got %d", len(shortcuts))
	}

	if shortcuts[0].Name != "Game A" || shortcuts[1].Name != "Game B" || shortcuts[2].Name != "Game C" {
		t.Errorf("unexpected shortcut names: %q, %q, %q", shortcuts[0].Name, shortcuts[1].Name, shortcuts[2].Name)
	}
}

func TestParseShortcutsVDF_EmptyFile(t *testing.T) {
	// Root with no entries
	data := []byte{vdfTypeObject}
	data = append(data, []byte("shortcuts")...)
	data = append(data, 0x00)
	data = append(data, vdfTypeEnd)

	shortcuts, err := parseShortcutsVDF(data)
	if err != nil {
		t.Fatalf("parseShortcutsVDF() error = %v", err)
	}

	if len(shortcuts) != 0 {
		t.Errorf("expected 0 shortcuts, got %d", len(shortcuts))
	}
}

func TestParseShortcutsVDF_NoTags(t *testing.T) {
	data := buildTestVDF([]testShortcut{
		{appID: 42, name: "No Tags Game", exe: "/game"},
	})

	shortcuts, err := parseShortcutsVDF(data)
	if err != nil {
		t.Fatalf("parseShortcutsVDF() error = %v", err)
	}

	if len(shortcuts) != 1 {
		t.Fatalf("expected 1 shortcut, got %d", len(shortcuts))
	}

	if shortcuts[0].Tags != nil {
		t.Errorf("Tags should be nil for shortcut without tags, got %v", shortcuts[0].Tags)
	}
}

func TestLoadShortcutsVDF_File(t *testing.T) {
	data := buildTestVDF([]testShortcut{
		{appID: 999, name: "File Test", exe: "/test"},
	})

	tmpDir := t.TempDir()
	path := filepath.Join(tmpDir, "shortcuts.vdf")
	if err := os.WriteFile(path, data, 0644); err != nil {
		t.Fatalf("failed to write test file: %v", err)
	}

	shortcuts, err := LoadShortcutsVDF(path)
	if err != nil {
		t.Fatalf("LoadShortcutsVDF() error = %v", err)
	}

	if len(shortcuts) != 1 || shortcuts[0].AppID != 999 {
		t.Errorf("unexpected result: %+v", shortcuts)
	}
}

func TestLoadShortcutsVDF_FileNotFound(t *testing.T) {
	_, err := LoadShortcutsVDF("/nonexistent/path/shortcuts.vdf")
	if err == nil {
		t.Error("LoadShortcutsVDF() should error on missing file")
	}
}

func TestParseShortcutsVDF_TooSmall(t *testing.T) {
	_, err := parseShortcutsVDF([]byte{0x00})
	if err == nil {
		t.Error("should error on data too small")
	}
}
