// Package pathutil provides shared path utilities for the desktop Agent.
package pathutil

import (
	"os"
	"path/filepath"
	"strings"
)

// ExpandHome expands ~ to the user's home directory.
func ExpandHome(path string) string {
	if strings.HasPrefix(path, "~/") {
		home, err := os.UserHomeDir()
		if err == nil {
			return filepath.Join(home, path[2:])
		}
	}
	return path
}
