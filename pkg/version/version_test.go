package version

import (
	"fmt"
	"strings"
	"testing"
)

func TestVersionDefault(t *testing.T) {
	// Without ldflags injection, Version defaults to "dev"
	if Version == "" {
		t.Error("Version should not be empty")
	}
}

func TestFull(t *testing.T) {
	full := Full()

	if full == "" {
		t.Fatal("Full() returned empty string")
	}

	// Should contain the version string
	if !strings.Contains(full, Version) {
		t.Errorf("Full() = %q, want to contain Version %q", full, Version)
	}

	// Should contain the commit hash
	if !strings.Contains(full, Commit) {
		t.Errorf("Full() = %q, want to contain Commit %q", full, Commit)
	}

	// Should contain the build date
	if !strings.Contains(full, BuildDate) {
		t.Errorf("Full() = %q, want to contain BuildDate %q", full, BuildDate)
	}
}

func TestFullFormat(t *testing.T) {
	// Verify the exact format: "version (commit: hash, built: date)"
	expected := fmt.Sprintf("%s (commit: %s, built: %s)", Version, Commit, BuildDate)
	if got := Full(); got != expected {
		t.Errorf("Full() = %q, want %q", got, expected)
	}
}

func TestGetInfo(t *testing.T) {
	info := GetInfo()

	if info.Version != Version {
		t.Errorf("GetInfo().Version = %q, want %q", info.Version, Version)
	}
	if info.Commit != Commit {
		t.Errorf("GetInfo().Commit = %q, want %q", info.Commit, Commit)
	}
	if info.BuildDate != BuildDate {
		t.Errorf("GetInfo().BuildDate = %q, want %q", info.BuildDate, BuildDate)
	}
}
