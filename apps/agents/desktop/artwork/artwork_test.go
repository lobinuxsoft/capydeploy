package artwork

import (
	"fmt"
	"os"
	"path/filepath"
	"testing"
)

func TestArtworkSuffix(t *testing.T) {
	tests := []struct {
		artType string
		suffix  string
		ok      bool
	}{
		{"grid", "p", true},
		{"banner", "", true},
		{"hero", "_hero", true},
		{"logo", "_logo", true},
		{"icon", "_icon", true},
		{"unknown", "", false},
		{"", "", false},
	}

	for _, tt := range tests {
		t.Run(tt.artType, func(t *testing.T) {
			suffix, ok := artworkSuffix(tt.artType)
			if ok != tt.ok {
				t.Errorf("artworkSuffix(%q) ok = %v, want %v", tt.artType, ok, tt.ok)
			}
			if suffix != tt.suffix {
				t.Errorf("artworkSuffix(%q) = %q, want %q", tt.artType, suffix, tt.suffix)
			}
		})
	}
}

func TestExtFromContentType(t *testing.T) {
	tests := []struct {
		contentType string
		ext         string
	}{
		{"image/png", "png"},
		{"image/jpeg", "jpg"},
		{"image/webp", "webp"},
		{"image/gif", ""},
		{"text/plain", ""},
		{"", ""},
	}

	for _, tt := range tests {
		t.Run(tt.contentType, func(t *testing.T) {
			ext := extFromContentType(tt.contentType)
			if ext != tt.ext {
				t.Errorf("extFromContentType(%q) = %q, want %q", tt.contentType, ext, tt.ext)
			}
		})
	}
}

func TestRemoveExistingArtwork(t *testing.T) {
	tmpDir := t.TempDir()

	// Create dummy files with different extensions
	appID := uint32(123456789)
	suffix := "p" // grid portrait
	base := "123456789p"

	for _, ext := range []string{"png", "jpg", "webp"} {
		path := filepath.Join(tmpDir, base+"."+ext)
		if err := os.WriteFile(path, []byte("dummy"), 0644); err != nil {
			t.Fatalf("failed to create test file: %v", err)
		}
	}

	// Also create a file that should NOT be removed (different appID)
	otherFile := filepath.Join(tmpDir, "999999999p.png")
	if err := os.WriteFile(otherFile, []byte("keep"), 0644); err != nil {
		t.Fatalf("failed to create other file: %v", err)
	}

	removeExistingArtwork(tmpDir, appID, suffix)

	// Verify target files are removed
	for _, ext := range []string{"png", "jpg", "webp"} {
		path := filepath.Join(tmpDir, base+"."+ext)
		if _, err := os.Stat(path); !os.IsNotExist(err) {
			t.Errorf("file %s should have been removed", path)
		}
	}

	// Verify other file is untouched
	if _, err := os.Stat(otherFile); err != nil {
		t.Errorf("file %s should not have been removed: %v", otherFile, err)
	}
}

func TestApplyFromData_InvalidContentType(t *testing.T) {
	err := ApplyFromData(123, "grid", []byte("data"), "image/gif")
	if err == nil {
		t.Error("expected error for unsupported content type")
	}
}

func TestApplyFromData_InvalidArtworkType(t *testing.T) {
	err := ApplyFromData(123, "nonexistent", []byte("data"), "image/png")
	if err == nil {
		t.Error("expected error for unknown artwork type")
	}
}

func TestApplyViaCEF_NoSteam(t *testing.T) {
	// Verify applyViaCEF does not panic or hang regardless of Steam state.
	// If Steam CEF is not running, it should return an error.
	// If Steam is running, it may succeed — either way, no panic is the goal.
	_ = applyViaCEF(123456, "grid", []byte("fake-image-data"))
}

func TestApplyViaCEF_InvalidType(t *testing.T) {
	err := applyViaCEF(123456, "invalid", []byte("data"))
	if err == nil {
		t.Error("expected error for invalid artwork type")
	}
}

func TestFilenameConventions(t *testing.T) {
	// Verify the filename pattern matches Steam's convention
	tests := []struct {
		artType     string
		appID       uint32
		contentType string
		wantFile    string
	}{
		{"grid", 123456, "image/png", "123456p.png"},
		{"banner", 123456, "image/jpeg", "123456.jpg"},
		{"hero", 123456, "image/webp", "123456_hero.webp"},
		{"logo", 123456, "image/png", "123456_logo.png"},
		{"icon", 123456, "image/jpeg", "123456_icon.jpg"},
	}

	for _, tt := range tests {
		t.Run(tt.artType, func(t *testing.T) {
			suffix, ok := artworkSuffix(tt.artType)
			if !ok {
				t.Fatalf("artworkSuffix(%q) failed", tt.artType)
			}
			ext := extFromContentType(tt.contentType)
			if ext == "" {
				t.Fatalf("extFromContentType(%q) failed", tt.contentType)
			}

			filename := fmt.Sprintf("%d%s.%s", tt.appID, suffix, ext)
			if filename != tt.wantFile {
				t.Errorf("got %q, want %q", filename, tt.wantFile)
			}
		})
	}
}
