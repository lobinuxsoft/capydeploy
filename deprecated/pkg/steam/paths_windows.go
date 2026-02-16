//go:build windows

package steam

import (
	"golang.org/x/sys/windows/registry"
)

// getBaseDir returns the Steam base directory on Windows using the registry.
func getBaseDir() (string, error) {
	// Try 64-bit registry first
	key, err := registry.OpenKey(registry.LOCAL_MACHINE, `SOFTWARE\Wow6432Node\Valve\Steam`, registry.QUERY_VALUE)
	if err != nil {
		// Fall back to 32-bit registry
		key, err = registry.OpenKey(registry.LOCAL_MACHINE, `SOFTWARE\Valve\Steam`, registry.QUERY_VALUE)
		if err != nil {
			return "", ErrSteamNotFound
		}
	}
	defer key.Close()

	steamPath, _, err := key.GetStringValue("InstallPath")
	if err != nil {
		return "", ErrSteamNotFound
	}

	return steamPath, nil
}

