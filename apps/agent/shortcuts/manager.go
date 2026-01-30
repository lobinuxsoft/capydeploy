// Package shortcuts provides Steam shortcut management for the Agent.
package shortcuts

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"

	"github.com/lobinuxsoft/capydeploy/apps/agent/artwork"
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/steam"
	"github.com/shadowblip/steam-shortcut-manager/pkg/shortcut"
)

// Manager handles Steam shortcut operations locally on the Agent.
type Manager struct {
	paths *steam.Paths
}

// NewManager creates a new shortcut manager.
func NewManager() (*Manager, error) {
	paths, err := steam.NewPaths()
	if err != nil {
		return nil, fmt.Errorf("failed to detect Steam: %w", err)
	}
	return &Manager{paths: paths}, nil
}

// NewManagerWithPaths creates a manager with custom paths (for testing).
func NewManagerWithPaths(paths *steam.Paths) *Manager {
	return &Manager{paths: paths}
}

// List returns all shortcuts for a user.
func (m *Manager) List(userID string) ([]protocol.ShortcutInfo, error) {
	shortcutsPath := m.paths.ShortcutsPath(userID)

	// Return empty list if file doesn't exist
	if _, err := os.Stat(shortcutsPath); os.IsNotExist(err) {
		return []protocol.ShortcutInfo{}, nil
	}

	shortcuts, err := shortcut.Load(shortcutsPath)
	if err != nil {
		return nil, fmt.Errorf("failed to load shortcuts: %w", err)
	}

	var result []protocol.ShortcutInfo
	for _, sc := range shortcuts.Shortcuts {
		result = append(result, protocol.ShortcutInfo{
			AppID:         uint32(sc.Appid),
			Name:          sc.AppName,
			Exe:           sc.Exe,
			StartDir:      sc.StartDir,
			LaunchOptions: sc.LaunchOptions,
			Tags:          tagsToSlice(sc.Tags),
			LastPlayed:    int64(sc.LastPlayTime),
		})
	}

	return result, nil
}

// Create adds a new shortcut for a user.
func (m *Manager) Create(userID string, cfg protocol.ShortcutConfig) (uint32, error) {
	shortcutsPath := m.paths.ShortcutsPath(userID)

	// Load existing shortcuts or create new
	var shortcuts *shortcut.Shortcuts
	if _, err := os.Stat(shortcutsPath); os.IsNotExist(err) {
		// Ensure config directory exists
		configDir := filepath.Dir(shortcutsPath)
		if err := os.MkdirAll(configDir, 0755); err != nil {
			return 0, fmt.Errorf("failed to create config dir: %w", err)
		}
		shortcuts = shortcut.NewShortcuts()
	} else {
		var err error
		shortcuts, err = shortcut.Load(shortcutsPath)
		if err != nil {
			return 0, fmt.Errorf("failed to load shortcuts: %w", err)
		}
	}

	// Calculate AppID
	appID := shortcut.CalculateAppID(cfg.Exe, cfg.Name)

	// Check if shortcut already exists
	if existing, _ := shortcuts.LookupByID(int64(appID)); existing != nil {
		return 0, fmt.Errorf("shortcut already exists: %s", cfg.Name)
	}

	// Create new shortcut
	sc := shortcut.NewShortcut(cfg.Name, cfg.Exe, shortcut.DefaultShortcut)
	sc.StartDir = cfg.StartDir
	sc.LaunchOptions = cfg.LaunchOptions
	sc.Appid = int64(appID)
	sc.Tags = sliceToTags(cfg.Tags)

	// Add to shortcuts
	if err := shortcuts.Add(sc); err != nil {
		return 0, fmt.Errorf("failed to add shortcut: %w", err)
	}

	// Save
	if err := shortcut.Save(shortcuts, shortcutsPath); err != nil {
		return 0, fmt.Errorf("failed to save shortcuts: %w", err)
	}

	return uint32(appID), nil
}

// CreateWithArtwork creates a shortcut and applies artwork if provided.
func (m *Manager) CreateWithArtwork(userID string, cfg protocol.ShortcutConfig) (uint32, *artwork.ApplyResult, error) {
	appID, err := m.Create(userID, cfg)
	if err != nil {
		return 0, nil, err
	}

	var artResult *artwork.ApplyResult
	if cfg.Artwork != nil {
		artResult, _ = artwork.Apply(userID, appID, cfg.Artwork)
	}

	return appID, artResult, nil
}

// Delete removes a shortcut by AppID or name.
func (m *Manager) Delete(userID string, appID uint32, name string) error {
	shortcutsPath := m.paths.ShortcutsPath(userID)

	shortcuts, err := shortcut.Load(shortcutsPath)
	if err != nil {
		return fmt.Errorf("failed to load shortcuts: %w", err)
	}

	// Find and remove the shortcut
	found := false
	for key, sc := range shortcuts.Shortcuts {
		if (appID > 0 && uint32(sc.Appid) == appID) || (name != "" && sc.AppName == name) {
			delete(shortcuts.Shortcuts, key)
			found = true
			break
		}
	}

	if !found {
		return fmt.Errorf("shortcut not found")
	}

	// Reindex keys to be sequential
	shortcuts = reindexShortcuts(shortcuts)

	// Save
	if err := shortcut.Save(shortcuts, shortcutsPath); err != nil {
		return fmt.Errorf("failed to save shortcuts: %w", err)
	}

	return nil
}

// reindexShortcuts ensures shortcut keys are sequential (0, 1, 2, ...).
func reindexShortcuts(shortcuts *shortcut.Shortcuts) *shortcut.Shortcuts {
	newShortcuts := shortcut.NewShortcuts()
	i := 0
	for _, sc := range shortcuts.Shortcuts {
		newShortcuts.Shortcuts[strconv.Itoa(i)] = sc
		i++
	}
	return newShortcuts
}

// tagsToSlice converts VDF tags map to string slice.
func tagsToSlice(tags map[string]interface{}) []string {
	if tags == nil {
		return nil
	}
	var result []string
	for _, v := range tags {
		if s, ok := v.(string); ok {
			result = append(result, s)
		}
	}
	return result
}

// sliceToTags converts string slice to VDF tags map.
func sliceToTags(tags []string) map[string]interface{} {
	if len(tags) == 0 {
		return nil
	}
	result := make(map[string]interface{})
	for i, tag := range tags {
		result[strconv.Itoa(i)] = tag
	}
	return result
}
