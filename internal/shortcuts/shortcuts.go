// Package shortcuts provides Steam shortcut management functions
// using the steam-shortcut-manager library directly
package shortcuts

import (
	"fmt"
	"io"
	"net/http"
	"path/filepath"
	"strings"

	"github.com/shadowblip/steam-shortcut-manager/pkg/remote"
	"github.com/shadowblip/steam-shortcut-manager/pkg/shortcut"
	"github.com/shadowblip/steam-shortcut-manager/pkg/steam"
)

// ArtworkConfig holds the artwork URLs to download
type ArtworkConfig struct {
	GridPortrait  string // 600x900 portrait grid (e.g. {appid}p.png)
	GridLandscape string // 920x430 landscape grid (e.g. {appid}.png)
	HeroImage     string // 1920x620 hero banner (e.g. {appid}_hero.png)
	LogoImage     string // Logo with transparency (e.g. {appid}_logo.png)
	IconImage     string // Square icon (e.g. {appid}_icon.png)
}

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
	return AddShortcutWithArtwork(cfg, name, exe, startDir, launchOpts, tags, nil)
}

// AddShortcutWithArtwork adds a Steam shortcut with custom artwork on a remote device
func AddShortcutWithArtwork(cfg *RemoteConfig, name, exe, startDir, launchOpts string, tags []string, artwork *ArtworkConfig) error {
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

	// Calculate appID for artwork naming
	appID := shortcut.CalculateAppID(exe, name)
	fmt.Printf("[DEBUG] Calculated AppID for '%s' (exe: %s): %d\n", name, exe, appID)

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
			s.Appid = int64(appID)

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

		// Download and upload artwork if provided
		if artwork != nil {
			// Construct the grid path manually (userdata/USER_ID/config/grid)
			shortcutsDir := filepath.Dir(shortcutsPath) // userdata/USER_ID/config
			gridPath := filepath.Join(shortcutsDir, "grid")
			// Convert to forward slashes for Linux
			gridPath = strings.ReplaceAll(gridPath, "\\", "/")

			fmt.Printf("[DEBUG] Artwork config received:\n")
			fmt.Printf("  GridPortrait (capsule 600x900): %s\n", artwork.GridPortrait)
			fmt.Printf("  GridLandscape (wide 920x430): %s\n", artwork.GridLandscape)
			fmt.Printf("  HeroImage: %s\n", artwork.HeroImage)
			fmt.Printf("  LogoImage: %s\n", artwork.LogoImage)
			fmt.Printf("  IconImage: %s\n", artwork.IconImage)
			fmt.Printf("[DEBUG] AppID: %d, Grid path: %s\n", appID, gridPath)

			// Ensure grid directory exists
			client.RunCommand(fmt.Sprintf("mkdir -p %q", gridPath))

			// Delete any existing artwork for this appID to avoid caching issues
			fmt.Printf("[DEBUG] Cleaning existing artwork for appID %d...\n", appID)
			client.RunCommand(fmt.Sprintf("rm -f %q/%dp.* %q/%d.* %q/%d_hero.* %q/%d_logo.* %q/%d_icon.*",
				gridPath, appID, gridPath, appID, gridPath, appID, gridPath, appID, gridPath, appID))

			// Download and upload each artwork type
			// Steam artwork naming convention:
			// - {appID}p.png = Portrait capsule (600x900) - shown in library grid
			// - {appID}.png = Horizontal/Wide capsule (920x430 or 460x215)
			// - {appID}_hero.png = Hero banner (1920x620)
			// - {appID}_logo.png = Logo with transparency
			// - {appID}_icon.png = Square icon
			if artwork.GridPortrait != "" {
				fmt.Println("[DEBUG] Uploading GridPortrait (capsule) as appID_p...")
				downloadAndUploadArtwork(client, artwork.GridPortrait, gridPath, fmt.Sprintf("%dp", appID))
			}
			if artwork.GridLandscape != "" {
				fmt.Println("[DEBUG] Uploading GridLandscape (wide) as appID...")
				downloadAndUploadArtwork(client, artwork.GridLandscape, gridPath, fmt.Sprintf("%d", appID))
			}
			if artwork.HeroImage != "" {
				fmt.Println("[DEBUG] Uploading HeroImage as appID_hero...")
				downloadAndUploadArtwork(client, artwork.HeroImage, gridPath, fmt.Sprintf("%d_hero", appID))
			}
			if artwork.LogoImage != "" {
				fmt.Println("[DEBUG] Uploading LogoImage as appID_logo...")
				downloadAndUploadArtwork(client, artwork.LogoImage, gridPath, fmt.Sprintf("%d_logo", appID))
			}
			if artwork.IconImage != "" {
				fmt.Println("[DEBUG] Uploading IconImage as appID_icon...")
				downloadAndUploadArtwork(client, artwork.IconImage, gridPath, fmt.Sprintf("%d_icon", appID))
			}
		}
	}

	return nil
}

// downloadAndUploadArtwork downloads an image from URL and uploads it to remote path
func downloadAndUploadArtwork(client *remote.Client, url, remotePath, baseName string) error {
	fmt.Printf("[DEBUG] Downloading artwork: %s -> %s/%s\n", url, remotePath, baseName)

	// Download the image
	resp, err := http.Get(url)
	if err != nil {
		return fmt.Errorf("failed to download artwork: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("failed to download artwork: HTTP %d", resp.StatusCode)
	}

	// Read image data
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("failed to read artwork data: %w", err)
	}

	fmt.Printf("[DEBUG] Downloaded %d bytes, Content-Type: %s\n", len(data), resp.Header.Get("Content-Type"))

	// Determine file extension - ALWAYS prefer Content-Type over URL
	// because URLs may have query parameters that confuse filepath.Ext
	contentType := resp.Header.Get("Content-Type")
	var ext string
	switch {
	case strings.Contains(contentType, "png"):
		ext = ".png"
	case strings.Contains(contentType, "jpeg"), strings.Contains(contentType, "jpg"):
		ext = ".jpg"
	case strings.Contains(contentType, "webp"):
		ext = ".webp"
	case strings.Contains(contentType, "gif"):
		ext = ".gif"
	default:
		// Fallback: try to extract from URL path (without query params)
		urlPath := url
		if idx := strings.Index(url, "?"); idx != -1 {
			urlPath = url[:idx]
		}
		ext = filepath.Ext(urlPath)
		if ext == "" {
			ext = ".png"
		}
	}

	// Upload to remote using WriteFile
	remoteDest := filepath.Join(remotePath, baseName+ext)
	// Convert to forward slashes for Linux
	remoteDest = strings.ReplaceAll(remoteDest, "\\", "/")

	fmt.Printf("[DEBUG] Uploading to: %s\n", remoteDest)

	if err := client.WriteFile(remoteDest, data, 0644); err != nil {
		return fmt.Errorf("failed to upload artwork: %w", err)
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
