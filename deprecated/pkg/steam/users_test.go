package steam

import (
	"os"
	"path/filepath"
	"testing"
)

func TestGetUsersWithPaths(t *testing.T) {
	tmpDir := t.TempDir()
	paths := NewPathsWithBase(tmpDir)

	// Create userdata directory with some users
	userDataDir := paths.UserDataDir()
	os.MkdirAll(filepath.Join(userDataDir, "12345", "config"), 0755)
	os.MkdirAll(filepath.Join(userDataDir, "67890", "config"), 0755)
	os.MkdirAll(filepath.Join(userDataDir, "invalid_user"), 0755) // Should be ignored

	// Create shortcuts.vdf for first user only
	os.WriteFile(filepath.Join(userDataDir, "12345", "config", "shortcuts.vdf"), []byte{}, 0644)

	users, err := GetUsersWithPaths(paths)
	if err != nil {
		t.Fatalf("GetUsersWithPaths() error = %v", err)
	}

	if len(users) != 2 {
		t.Errorf("GetUsersWithPaths() returned %d users, want 2", len(users))
	}

	// Find user with shortcuts
	var foundWithShortcuts bool
	for _, u := range users {
		if u.ID == "12345" {
			if !u.HasShortcuts {
				t.Error("User 12345 should have HasShortcuts = true")
			}
			foundWithShortcuts = true
		}
		if u.ID == "67890" {
			if u.HasShortcuts {
				t.Error("User 67890 should have HasShortcuts = false")
			}
		}
	}

	if !foundWithShortcuts {
		t.Error("Should have found user with shortcuts")
	}
}

func TestGetUsersWithPaths_NoUserDataDir(t *testing.T) {
	paths := NewPathsWithBase("/nonexistent/path")

	_, err := GetUsersWithPaths(paths)
	if err == nil {
		t.Error("GetUsersWithPaths() should error when userdata dir doesn't exist")
	}
}

func TestGetUsersWithPaths_EmptyUserData(t *testing.T) {
	tmpDir := t.TempDir()
	paths := NewPathsWithBase(tmpDir)

	// Create empty userdata directory
	os.MkdirAll(paths.UserDataDir(), 0755)

	users, err := GetUsersWithPaths(paths)
	if err != nil {
		t.Fatalf("GetUsersWithPaths() error = %v", err)
	}

	if len(users) != 0 {
		t.Errorf("GetUsersWithPaths() returned %d users, want 0", len(users))
	}
}

func TestGetUsersWithPaths_IgnoresFiles(t *testing.T) {
	tmpDir := t.TempDir()
	paths := NewPathsWithBase(tmpDir)

	userDataDir := paths.UserDataDir()
	os.MkdirAll(userDataDir, 0755)

	// Create a file instead of directory
	os.WriteFile(filepath.Join(userDataDir, "12345"), []byte{}, 0644)
	// Create a valid user directory
	os.MkdirAll(filepath.Join(userDataDir, "67890"), 0755)

	users, err := GetUsersWithPaths(paths)
	if err != nil {
		t.Fatalf("GetUsersWithPaths() error = %v", err)
	}

	// Should only find the directory, not the file
	if len(users) != 1 {
		t.Errorf("GetUsersWithPaths() returned %d users, want 1", len(users))
	}
	if len(users) > 0 && users[0].ID != "67890" {
		t.Errorf("User ID = %q, want %q", users[0].ID, "67890")
	}
}

func TestUserIDToUint32(t *testing.T) {
	tests := []struct {
		input   string
		want    uint32
		wantErr bool
	}{
		{"12345", 12345, false},
		{"0", 0, false},
		{"4294967295", 4294967295, false}, // Max uint32
		{"-1", 0, true},
		{"invalid", 0, true},
		{"", 0, true},
		{"4294967296", 0, true}, // Overflow
	}

	for _, tt := range tests {
		t.Run(tt.input, func(t *testing.T) {
			got, err := UserIDToUint32(tt.input)
			if (err != nil) != tt.wantErr {
				t.Errorf("UserIDToUint32(%q) error = %v, wantErr %v", tt.input, err, tt.wantErr)
				return
			}
			if !tt.wantErr && got != tt.want {
				t.Errorf("UserIDToUint32(%q) = %d, want %d", tt.input, got, tt.want)
			}
		})
	}
}

func TestUint32ToUserID(t *testing.T) {
	tests := []struct {
		input uint32
		want  string
	}{
		{12345, "12345"},
		{0, "0"},
		{4294967295, "4294967295"},
	}

	for _, tt := range tests {
		got := Uint32ToUserID(tt.input)
		if got != tt.want {
			t.Errorf("Uint32ToUserID(%d) = %q, want %q", tt.input, got, tt.want)
		}
	}
}

func TestUserIDRoundTrip(t *testing.T) {
	original := uint32(12345678)

	str := Uint32ToUserID(original)
	parsed, err := UserIDToUint32(str)
	if err != nil {
		t.Fatalf("Round trip error: %v", err)
	}

	if parsed != original {
		t.Errorf("Round trip failed: %d -> %q -> %d", original, str, parsed)
	}
}

func TestUser_Fields(t *testing.T) {
	user := User{
		ID:           "12345",
		HasShortcuts: true,
	}

	if user.ID != "12345" {
		t.Errorf("ID = %q, want %q", user.ID, "12345")
	}
	if !user.HasShortcuts {
		t.Error("HasShortcuts should be true")
	}
}
