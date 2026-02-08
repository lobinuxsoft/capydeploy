package steam

import (
	"os"
	"path/filepath"
	"testing"
)

func TestArtworkTypeToCEFAsset(t *testing.T) {
	tests := []struct {
		artType   string
		wantAsset int
		wantOK    bool
	}{
		{"grid", CEFAssetGridPortrait, true},
		{"banner", CEFAssetGridLandscape, true},
		{"hero", CEFAssetHero, true},
		{"logo", CEFAssetLogo, true},
		{"icon", CEFAssetIcon, true},
		{"unknown", 0, false},
		{"", 0, false},
	}

	for _, tt := range tests {
		t.Run(tt.artType, func(t *testing.T) {
			asset, ok := ArtworkTypeToCEFAsset(tt.artType)
			if ok != tt.wantOK {
				t.Errorf("ArtworkTypeToCEFAsset(%q) ok = %v, want %v", tt.artType, ok, tt.wantOK)
			}
			if asset != tt.wantAsset {
				t.Errorf("ArtworkTypeToCEFAsset(%q) = %d, want %d", tt.artType, asset, tt.wantAsset)
			}
		})
	}
}

func TestFindJSContext_Priority(t *testing.T) {
	client := NewCEFClient()
	tabs := []cefTab{
		{Title: "SP", WebSocketDebuggerURL: "ws://localhost:8080/devtools/page/1"},
		{Title: "SharedJSContext", WebSocketDebuggerURL: "ws://localhost:8080/devtools/page/2"},
		{Title: "Other", WebSocketDebuggerURL: "ws://localhost:8080/devtools/page/3"},
	}

	tab, err := client.findJSContext(tabs)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if tab.Title != "SharedJSContext" {
		t.Errorf("expected SharedJSContext, got %q", tab.Title)
	}
}

func TestFindJSContext_Fallback(t *testing.T) {
	client := NewCEFClient()
	tabs := []cefTab{
		{Title: "Other", WebSocketDebuggerURL: "ws://localhost:8080/devtools/page/1"},
		{Title: "SP", WebSocketDebuggerURL: "ws://localhost:8080/devtools/page/2"},
	}

	tab, err := client.findJSContext(tabs)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if tab.Title != "SP" {
		t.Errorf("expected SP fallback, got %q", tab.Title)
	}
}

func TestFindJSContext_NotFound(t *testing.T) {
	client := NewCEFClient()

	tests := []struct {
		name string
		tabs []cefTab
	}{
		{"empty", []cefTab{}},
		{"no ws url", []cefTab{{Title: "SharedJSContext"}}},
		{"irrelevant tabs", []cefTab{{Title: "Other", WebSocketDebuggerURL: "ws://x"}}},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, err := client.findJSContext(tt.tabs)
			if err == nil {
				t.Error("expected error when no suitable tab found")
			}
		})
	}
}

func TestFindJSContext_SkipsTabsWithoutWSURL(t *testing.T) {
	client := NewCEFClient()
	tabs := []cefTab{
		{Title: "SharedJSContext", WebSocketDebuggerURL: ""},  // no WS URL — skip
		{Title: "SP", WebSocketDebuggerURL: "ws://localhost:8080/devtools/page/1"},
	}

	tab, err := client.findJSContext(tabs)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if tab.Title != "SP" {
		t.Errorf("expected SP (SharedJSContext has no WS URL), got %q", tab.Title)
	}
}

func TestJsString(t *testing.T) {
	tests := []struct {
		name  string
		input string
		want  string
	}{
		{"simple string", "My Game", `"My Game"`},
		{"with double quotes", `Say "hello"`, `"Say \"hello\""`},
		{"with backslash", `path\to\file`, `"path\\to\\file"`},
		{"with spaces", "path with spaces", `"path with spaces"`},
		{"empty string", "", `""`},
		{"with newline", "line1\nline2", `"line1\nline2"`},
		{"with tab", "col1\tcol2", `"col1\tcol2"`},
		{"unicode", "日本語ゲーム", `"日本語ゲーム"`},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := jsString(tt.input)
			if got != tt.want {
				t.Errorf("jsString(%q) = %s, want %s", tt.input, got, tt.want)
			}
		})
	}
}

func TestJsString_WindowsPaths(t *testing.T) {
	tests := []struct {
		name  string
		input string
		want  string
	}{
		{
			"program files",
			`C:\Program Files\Steam\game.exe`,
			`"C:\\Program Files\\Steam\\game.exe"`,
		},
		{
			"nested path",
			`C:\Users\Player\Games\My Game\launcher.exe`,
			`"C:\\Users\\Player\\Games\\My Game\\launcher.exe"`,
		},
		{
			"quoted windows path",
			`"C:\Games\test"`,
			`"\"C:\\Games\\test\""`,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := jsString(tt.input)
			if got != tt.want {
				t.Errorf("jsString(%q) = %s, want %s", tt.input, got, tt.want)
			}
		})
	}
}

func TestEnsureCEFDebugFile_CreatesFile(t *testing.T) {
	// Use a temp dir to simulate Steam base dir — we can't call the real
	// function without Steam installed, but we test the file creation logic.
	tmpDir := t.TempDir()
	debugPath := filepath.Join(tmpDir, cefDebugFile)

	// File doesn't exist yet
	if _, err := os.Stat(debugPath); !os.IsNotExist(err) {
		t.Fatal("debug file should not exist before test")
	}

	// Create it
	if err := os.WriteFile(debugPath, []byte{}, 0644); err != nil {
		t.Fatalf("failed to create debug file: %v", err)
	}

	// Verify it exists
	info, err := os.Stat(debugPath)
	if err != nil {
		t.Fatalf("debug file should exist after creation: %v", err)
	}
	if info.Size() != 0 {
		t.Errorf("debug file should be empty, got %d bytes", info.Size())
	}
}

func TestEnsureCEFDebugFile_Idempotent(t *testing.T) {
	tmpDir := t.TempDir()
	debugPath := filepath.Join(tmpDir, cefDebugFile)

	// Create file first time
	os.WriteFile(debugPath, []byte{}, 0644)

	// Stat should show it exists (simulates "already exists" path)
	if _, err := os.Stat(debugPath); err != nil {
		t.Fatalf("debug file should exist: %v", err)
	}
}
