// Package shortcuts provides Steam shortcut management for the Agent.
package shortcuts

import (
	"context"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"time"

	"github.com/lobinuxsoft/capydeploy/apps/agents/desktop/artwork"
	agentSteam "github.com/lobinuxsoft/capydeploy/apps/agents/desktop/steam"
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
// Tries CEF API first (instant, reflects live state), falls back to VDF file.
func (m *Manager) List(userID string) ([]protocol.ShortcutInfo, error) {
	result, err := m.listViaCEF()
	if err == nil {
		return result, nil
	}
	log.Printf("[shortcuts] CEF list failed, falling back to VDF: %v", err)
	return m.listViaVDF(userID)
}

// listViaCEF retrieves shortcuts from Steam's CEF API.
func (m *Manager) listViaCEF() ([]protocol.ShortcutInfo, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	client := agentSteam.NewCEFClient()
	cefShortcuts, err := client.GetAllShortcuts(ctx)
	if err != nil {
		return nil, err
	}

	result := make([]protocol.ShortcutInfo, 0, len(cefShortcuts))
	for _, sc := range cefShortcuts {
		result = append(result, agentSteam.CEFShortcutToInfo(sc))
	}

	return result, nil
}

// listViaVDF reads shortcuts from the VDF file on disk.
func (m *Manager) listViaVDF(userID string) ([]protocol.ShortcutInfo, error) {
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

// Create adds a new shortcut via CEF API (instant, no Steam restart needed).
// The userID parameter is kept for signature compatibility but is not used
// for creation — CEF handles persistence internally.
func (m *Manager) Create(userID string, cfg protocol.ShortcutConfig) (uint32, error) {
	exePath := expandPath(cfg.Exe)
	startDir := expandPath(cfg.StartDir)

	// On Windows, Steam expects quoted paths
	if runtime.GOOS == "windows" {
		exePath = quotePath(exePath)
		startDir = quotePath(startDir)
	}

	if err := agentSteam.EnsureCEFReady(); err != nil {
		return 0, fmt.Errorf("CEF not available: %w", err)
	}

	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()

	client := agentSteam.NewCEFClient()
	appID, err := client.AddShortcut(ctx, cfg.Name, exePath, startDir, cfg.LaunchOptions)
	if err != nil {
		return 0, fmt.Errorf("failed to create shortcut via CEF: %w", err)
	}

	// AddShortcut ignores the name and uses the executable filename,
	// so we must rename it afterwards.
	if err := client.SetShortcutName(ctx, appID, cfg.Name); err != nil {
		fmt.Printf("Warning: failed to set shortcut name: %v\n", err)
	}

	// On Linux, automatically set Proton as the compatibility tool
	if runtime.GOOS == "linux" {
		if err := client.SpecifyCompatTool(ctx, appID, "proton_experimental"); err != nil {
			log.Printf("[shortcuts] warning: failed to set Proton for appID %d: %v", appID, err)
		}
	}

	return appID, nil
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

// Delete removes a shortcut by AppID via CEF API, and optionally deletes the game folder.
func (m *Manager) Delete(userID string, appID uint32) error {
	return m.DeleteWithCleanup(userID, appID, true)
}

// DeleteWithCleanup removes a shortcut via CEF API and optionally its game folder.
func (m *Manager) DeleteWithCleanup(userID string, appID uint32, deleteGameFolder bool) error {
	// Look up StartDir before deleting (needed to know which game folder to remove)
	var gameFolderPath string
	shortcuts, err := m.List(userID)
	if err == nil {
		for _, sc := range shortcuts {
			if sc.AppID == appID {
				gameFolderPath = unquotePath(sc.StartDir)
				break
			}
		}
	}

	// Remove shortcut via CEF (instant, no Steam restart needed)
	if err := agentSteam.EnsureCEFReady(); err != nil {
		return fmt.Errorf("CEF not available: %w", err)
	}

	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()

	client := agentSteam.NewCEFClient()
	if err := client.RemoveShortcut(ctx, appID); err != nil {
		return fmt.Errorf("failed to remove shortcut via CEF: %w", err)
	}

	// Delete game folder if requested and path is valid
	if deleteGameFolder && gameFolderPath != "" {
		if err := deleteGameDirectory(gameFolderPath); err != nil {
			fmt.Printf("Warning: failed to delete game folder %s: %v\n", gameFolderPath, err)
		}
	}

	// Delete artwork from grid folder (best-effort cleanup of local files)
	if err := m.deleteArtwork(userID, appID); err != nil {
		fmt.Printf("Warning: failed to delete artwork: %v\n", err)
	}

	return nil
}

// deleteArtwork removes all artwork files for an appID from the grid folder.
func (m *Manager) deleteArtwork(userID string, appID uint32) error {
	gridDir := m.paths.GridDir(userID)

	// All possible artwork file patterns
	patterns := []string{
		fmt.Sprintf("%d.*", appID),        // landscape grid
		fmt.Sprintf("%dp.*", appID),       // portrait grid
		fmt.Sprintf("%d_hero.*", appID),   // hero
		fmt.Sprintf("%d_logo.*", appID),   // logo
		fmt.Sprintf("%d_icon.*", appID),   // icon
	}

	for _, pattern := range patterns {
		matches, err := filepath.Glob(filepath.Join(gridDir, pattern))
		if err != nil {
			continue
		}
		for _, path := range matches {
			if err := os.Remove(path); err != nil && !os.IsNotExist(err) {
				fmt.Printf("Warning: failed to remove %s: %v\n", path, err)
			}
		}
	}

	return nil
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

// quotePath wraps a path in double quotes for Steam on Windows.
// Linux shortcuts must NOT have quotes around paths.
func quotePath(path string) string {
	if runtime.GOOS != "windows" {
		return strings.Trim(path, "\"")
	}
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
