//go:build !windows

package steam

import (
	"os"
	"path/filepath"
)

// getBaseDir returns the Steam base directory on Linux/Unix systems.
func getBaseDir() (string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}

	// Primary location: ~/.steam/steam
	steamDir := filepath.Join(home, ".steam", "steam")
	if _, err := os.Stat(steamDir); err == nil {
		return steamDir, nil
	}

	// Fallback: ~/.local/share/Steam
	steamDir = filepath.Join(home, ".local", "share", "Steam")
	if _, err := os.Stat(steamDir); err == nil {
		return steamDir, nil
	}

	// Flatpak location
	steamDir = filepath.Join(home, ".var", "app", "com.valvesoftware.Steam", ".steam", "steam")
	if _, err := os.Stat(steamDir); err == nil {
		return steamDir, nil
	}

	return "", ErrSteamNotFound
}

