// Package version provides build-time version information.
package version

import "fmt"

const (
	// Major version component.
	Major = 0
	// Minor version component.
	Minor = 1
	// Patch version component.
	Patch = 0
)

var (
	// Version is the semantic version (major.minor.patch), set at build time via ldflags.
	// For development builds: "0.1.0-dev+abc1234"
	// For release builds: "0.1.0"
	Version = fmt.Sprintf("%d.%d.%d-dev", Major, Minor, Patch)

	// Commit is the git commit hash, set at build time via ldflags.
	Commit = "unknown"

	// BuildDate is the build timestamp, set at build time via ldflags.
	BuildDate = "unknown"
)

// Full returns a formatted string with all version information.
func Full() string {
	return fmt.Sprintf("%s (commit: %s, built: %s)", Version, Commit, BuildDate)
}
