// Package shortcuts provides Steam shortcut management for the Agent.
package shortcuts

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"

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

	// Expand paths (~ to home directory) and add quotes for Steam
	exePath := quotePath(expandPath(cfg.Exe))
	startDir := quotePath(expandPath(cfg.StartDir))

	// Calculate AppID using the expanded (but unquoted) path
	appID := shortcut.CalculateAppID(expandPath(cfg.Exe), cfg.Name)

	// Check if shortcut already exists
	if existing, _ := shortcuts.LookupByID(int64(appID)); existing != nil {
		return 0, fmt.Errorf("shortcut already exists: %s", cfg.Name)
	}

	// Create new shortcut
	sc := shortcut.NewShortcut(cfg.Name, exePath, shortcut.DefaultShortcut)
	sc.StartDir = startDir
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

// Delete removes a shortcut by AppID or name, and optionally deletes the game folder.
func (m *Manager) Delete(userID string, appID uint32, name string) error {
	return m.DeleteWithCleanup(userID, appID, name, true)
}

// DeleteWithCleanup removes a shortcut and optionally its game folder.
func (m *Manager) DeleteWithCleanup(userID string, appID uint32, name string, deleteGameFolder bool) error {
	shortcutsPath := m.paths.ShortcutsPath(userID)

	shortcuts, err := shortcut.Load(shortcutsPath)
	if err != nil {
		return fmt.Errorf("failed to load shortcuts: %w", err)
	}

	// Find the shortcut and get its StartDir before removing
	var gameFolderPath string
	found := false
	for key, sc := range shortcuts.Shortcuts {
		if (appID > 0 && uint32(sc.Appid) == appID) || (name != "" && sc.AppName == name) {
			// Get the game folder path (remove quotes if present)
			gameFolderPath = unquotePath(sc.StartDir)
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

	// Delete game folder if requested and path is valid
	if deleteGameFolder && gameFolderPath != "" {
		if err := deleteGameDirectory(gameFolderPath); err != nil {
			// Log but don't fail - shortcut was already removed
			fmt.Printf("Warning: failed to delete game folder %s: %v\n", gameFolderPath, err)
		}
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

// expandPath expands ~ to the user's home directory.
func expandPath(path string) string {
	if strings.HasPrefix(path, "~/") {
		home, err := os.UserHomeDir()
		if err == nil {
			return filepath.Join(home, path[2:])
		}
	}
	return path
}

// quotePath wraps a path in double quotes for Steam if not already quoted.
func quotePath(path string) string {
	if strings.HasPrefix(path, "\"") && strings.HasSuffix(path, "\"") {
		return path
	}
	return "\"" + path + "\""
}

// unquotePath removes surrounding double quotes from a path.
func unquotePath(path string) string {
	if strings.HasPrefix(path, "\"") && strings.HasSuffix(path, "\"") {
		return path[1 : len(path)-1]
	}
	return path
}

// deleteGameDirectory safely removes a game installation directory.
// Only deletes if the path looks like a valid game folder (not system paths).
func deleteGameDirectory(path string) error {
	if path == "" {
		return nil
	}

	// Expand path if it uses ~
	path = expandPath(path)

	// Safety checks - don't delete system paths or root directories
	absPath, err := filepath.Abs(path)
	if err != nil {
		return fmt.Errorf("invalid path: %w", err)
	}

	// Get home directory for validation
	home, err := os.UserHomeDir()
	if err != nil {
		return fmt.Errorf("cannot determine home directory: %w", err)
	}

	// Only allow deletion within user's home directory
	// Use case-insensitive comparison for Windows compatibility
	if !isSubPath(home, absPath) {
		return fmt.Errorf("refusing to delete path outside home directory: %s", absPath)
	}

	// Don't delete the home directory itself or immediate subdirectories like ~/Games
	relPath, err := filepath.Rel(home, absPath)
	if err != nil {
		return fmt.Errorf("cannot determine relative path: %w", err)
	}

	// Must be at least 2 levels deep (e.g., ~/Games/MyGame, not ~/Games)
	parts := strings.Split(relPath, string(filepath.Separator))
	if len(parts) < 2 {
		return fmt.Errorf("refusing to delete top-level directory: %s", absPath)
	}

	// Check if path exists
	info, err := os.Stat(absPath)
	if os.IsNotExist(err) {
		return nil // Already gone, nothing to do
	}
	if err != nil {
		return fmt.Errorf("cannot stat path: %w", err)
	}

	// Must be a directory
	if !info.IsDir() {
		return fmt.Errorf("path is not a directory: %s", absPath)
	}

	// Delete the directory and all its contents
	if err := os.RemoveAll(absPath); err != nil {
		return fmt.Errorf("failed to remove directory: %w", err)
	}

	return nil
}

// isSubPath checks if child is inside parent directory.
// Uses case-insensitive comparison on Windows.
func isSubPath(parent, child string) bool {
	parent = filepath.Clean(parent)
	child = filepath.Clean(child)

	// Ensure parent ends with separator for proper prefix matching
	if !strings.HasSuffix(parent, string(filepath.Separator)) {
		parent = parent + string(filepath.Separator)
	}

	// On Windows, paths are case-insensitive
	if filepath.Separator == '\\' {
		return strings.HasPrefix(strings.ToLower(child), strings.ToLower(parent))
	}

	return strings.HasPrefix(child, parent)
}
