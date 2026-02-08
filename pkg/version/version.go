// Package version provides build-time version information.
package version

import "fmt"

var (
	// Version is the semantic version, set at build time via ldflags.
	// Source of truth: VERSION file at repo root.
	Version = "dev"

	// Commit is the git commit hash, set at build time via ldflags.
	Commit = "unknown"

	// BuildDate is the build timestamp, set at build time via ldflags.
	BuildDate = "unknown"
)

// Full returns a formatted string with all version information.
func Full() string {
	return fmt.Sprintf("%s (commit: %s, built: %s)", Version, Commit, BuildDate)
}

// Info represents version information for frontend consumption.
type Info struct {
	Version   string `json:"version"`
	Commit    string `json:"commit"`
	BuildDate string `json:"buildDate"`
}

// GetInfo returns the current version information.
func GetInfo() Info {
	return Info{
		Version:   Version,
		Commit:    Commit,
		BuildDate: BuildDate,
	}
}
