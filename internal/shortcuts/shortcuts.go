// Package shortcuts provides Steam shortcut management functions
// using the steam-shortcut-manager library directly
package shortcuts

import (
	"fmt"
	"strings"

	"github.com/shadowblip/steam-shortcut-manager/pkg/remote"
	"github.com/shadowblip/steam-shortcut-manager/pkg/shortcut"
	"github.com/shadowblip/steam-shortcut-manager/pkg/steam"
)

// RemoteConfig holds the SSH connection parameters
type RemoteConfig struct {
	Host     string
	Port     int
	User     string
	Password string
	KeyFile  string
}

// AddShortcut adds a Steam shortcut on a remote device
func AddShortcut(cfg *RemoteConfig, name, exe, startDir, launchOpts string, tags []string) error {
	// Create and connect remote client
	client := remote.NewClient(&remote.Config{
		Host:     cfg.Host,
		Port:     cfg.Port,
		User:     cfg.User,
		Password: cfg.Password,
		KeyFile:  cfg.KeyFile,
	})

	if err := client.Connect(); err != nil {
		return fmt.Errorf("failed to connect: %w", err)
	}
	defer client.Close()

	// Set remote clients for library packages
	shortcut.SetRemoteClient(client)
	steam.SetRemoteClient(client)

	// Get all Steam users on the remote device
	users, err := steam.GetRemoteUsers()
	if err != nil {
		return fmt.Errorf("failed to get Steam users: %w", err)
	}

	if len(users) == 0 {
		return fmt.Errorf("no Steam users found on remote device")
	}

	// Add shortcut for all users
	for _, user := range users {
		shortcutsPath, err := steam.GetRemoteShortcutsPath(user)
		if err != nil {
			continue
		}

		// Load existing shortcuts or create new
		var shortcuts *shortcut.Shortcuts
		if steam.RemoteHasShortcuts(user) {
			shortcuts, err = shortcut.Load(shortcutsPath)
			if err != nil {
				return fmt.Errorf("failed to load shortcuts for user %s: %w", user, err)
			}
		} else {
			shortcuts = shortcut.NewShortcuts()
		}

		// Create new shortcut
		newShortcut := shortcut.NewShortcut(name, exe, func(s *shortcut.Shortcut) {
			s.AllowDesktopConfig = 1
			s.AllowOverlay = 1
			s.StartDir = startDir
			s.LaunchOptions = launchOpts
			s.Appid = int64(shortcut.CalculateAppID(exe, name))

			// Add tags
			s.Tags = map[string]interface{}{}
			for i, tag := range tags {
				s.Tags[fmt.Sprintf("%d", i)] = tag
			}
		})

		// Add to shortcuts collection
		if err := shortcuts.Add(newShortcut); err != nil {
			return fmt.Errorf("failed to add shortcut for user %s: %w", user, err)
		}

		// Save shortcuts
		if err := shortcut.Save(shortcuts, shortcutsPath); err != nil {
			return fmt.Errorf("failed to save shortcuts for user %s: %w", user, err)
		}
	}

	return nil
}

// RemoveShortcut removes a Steam shortcut from a remote device
func RemoveShortcut(cfg *RemoteConfig, name string) error {
	// Create and connect remote client
	client := remote.NewClient(&remote.Config{
		Host:     cfg.Host,
		Port:     cfg.Port,
		User:     cfg.User,
		Password: cfg.Password,
		KeyFile:  cfg.KeyFile,
	})

	if err := client.Connect(); err != nil {
		return fmt.Errorf("failed to connect: %w", err)
	}
	defer client.Close()

	// Set remote clients for library packages
	shortcut.SetRemoteClient(client)
	steam.SetRemoteClient(client)

	// Get all Steam users
	users, err := steam.GetRemoteUsers()
	if err != nil {
		return fmt.Errorf("failed to get Steam users: %w", err)
	}

	// Remove shortcut for all users
	for _, user := range users {
		if !steam.RemoteHasShortcuts(user) {
			continue
		}

		shortcutsPath, err := steam.GetRemoteShortcutsPath(user)
		if err != nil {
			continue
		}

		shortcuts, err := shortcut.Load(shortcutsPath)
		if err != nil {
			continue
		}

		// Filter out the shortcut with the given name
		newShortcuts := shortcut.NewShortcuts()
		for _, sc := range shortcuts.Shortcuts {
			if sc.AppName == name {
				continue // Skip the one we're removing
			}
			newShortcuts.Add(&sc)
		}

		// Save the updated shortcuts
		if err := shortcut.Save(newShortcuts, shortcutsPath); err != nil {
			return fmt.Errorf("failed to save shortcuts for user %s: %w", user, err)
		}
	}

	return nil
}

// ListShortcuts returns all Steam shortcuts from a remote device
func ListShortcuts(cfg *RemoteConfig) ([]ShortcutInfo, error) {
	// Create and connect remote client
	client := remote.NewClient(&remote.Config{
		Host:     cfg.Host,
		Port:     cfg.Port,
		User:     cfg.User,
		Password: cfg.Password,
		KeyFile:  cfg.KeyFile,
	})

	if err := client.Connect(); err != nil {
		return nil, fmt.Errorf("failed to connect: %w", err)
	}
	defer client.Close()

	// Set remote clients for library packages
	shortcut.SetRemoteClient(client)
	steam.SetRemoteClient(client)

	// Get all Steam users
	users, err := steam.GetRemoteUsers()
	if err != nil {
		return nil, fmt.Errorf("failed to get Steam users: %w", err)
	}

	var result []ShortcutInfo

	// Get shortcuts from all users
	for _, user := range users {
		if !steam.RemoteHasShortcuts(user) {
			continue
		}

		shortcutsPath, err := steam.GetRemoteShortcutsPath(user)
		if err != nil {
			continue
		}

		shortcuts, err := shortcut.Load(shortcutsPath)
		if err != nil {
			continue
		}

		for _, sc := range shortcuts.Shortcuts {
			result = append(result, ShortcutInfo{
				Name:          sc.AppName,
				Exe:           sc.Exe,
				StartDir:      sc.StartDir,
				LaunchOptions: sc.LaunchOptions,
				AppID:         sc.Appid,
			})
		}
	}

	return result, nil
}

// ShortcutInfo represents basic shortcut information
type ShortcutInfo struct {
	Name          string
	Exe           string
	StartDir      string
	LaunchOptions string
	AppID         int64
}

// ParseTags parses a comma-separated tag string into a slice
func ParseTags(tagsStr string) []string {
	if tagsStr == "" {
		return nil
	}
	tags := strings.Split(tagsStr, ",")
	result := make([]string, 0, len(tags))
	for _, tag := range tags {
		tag = strings.TrimSpace(tag)
		if tag != "" {
			result = append(result, tag)
		}
	}
	return result
}

// RefreshSteamLibrary performs a soft restart of Steam to reload shortcuts
// In Gaming Mode (Big Picture), Steam will automatically relaunch
func RefreshSteamLibrary(cfg *RemoteConfig) error {
	// Create and connect remote client
	client := remote.NewClient(&remote.Config{
		Host:     cfg.Host,
		Port:     cfg.Port,
		User:     cfg.User,
		Password: cfg.Password,
		KeyFile:  cfg.KeyFile,
	})

	if err := client.Connect(); err != nil {
		return fmt.Errorf("failed to connect: %w", err)
	}
	defer client.Close()

	// Soft restart Steam - in Gaming Mode it will automatically relaunch
	// We use steam -shutdown which gracefully closes Steam
	// On Bazzite/SteamOS Gaming Mode, the session manager will restart Steam automatically
	client.RunCommand(`steam -shutdown >/dev/null 2>&1 || true`)

	return nil
}
