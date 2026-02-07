package version

import (
	"fmt"
	"strings"
	"testing"
)

func TestVersionConstants(t *testing.T) {
	// Major, Minor, Patch should be non-negative integers
	if Major < 0 {
		t.Errorf("Major = %d, want >= 0", Major)
	}
	if Minor < 0 {
		t.Errorf("Minor = %d, want >= 0", Minor)
	}
	if Patch < 0 {
		t.Errorf("Patch = %d, want >= 0", Patch)
	}
}

func TestVersionFormat(t *testing.T) {
	// Default (dev) version should contain major.minor.patch
	expected := fmt.Sprintf("%d.%d.%d", Major, Minor, Patch)
	if !strings.Contains(Version, expected) {
		t.Errorf("Version = %q, want to contain %q", Version, expected)
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
