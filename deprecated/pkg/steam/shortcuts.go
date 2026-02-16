package steam

import (
	"fmt"
	"hash/crc32"
	"os"
	"path/filepath"
	"strings"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

// ShortcutManager handles Steam shortcut operations.
type ShortcutManager struct {
	paths *Paths
}

// NewShortcutManager creates a new ShortcutManager.
func NewShortcutManager() (*ShortcutManager, error) {
	paths, err := NewPaths()
	if err != nil {
		return nil, err
	}
	return &ShortcutManager{paths: paths}, nil
}

// NewShortcutManagerWithPaths creates a ShortcutManager with custom paths.
func NewShortcutManagerWithPaths(paths *Paths) *ShortcutManager {
	return &ShortcutManager{paths: paths}
}

// GetShortcutsPath returns the shortcuts.vdf path for a user.
func (m *ShortcutManager) GetShortcutsPath(userID string) string {
	return m.paths.ShortcutsPath(userID)
}

// GetGridDir returns the grid artwork directory for a user.
func (m *ShortcutManager) GetGridDir(userID string) string {
	return m.paths.GridDir(userID)
}

// EnsureGridDir creates the grid directory if it doesn't exist.
func (m *ShortcutManager) EnsureGridDir(userID string) error {
	return m.paths.EnsureGridDir(userID)
}

// GenerateAppID generates a Steam shortcut app ID from executable path and name.
// This matches Steam's algorithm for non-Steam game shortcuts.
func GenerateAppID(exe, name string) uint32 {
	// Steam's algorithm: CRC32 of (exe + name) | 0x80000000
	key := exe + name
	crc := crc32.ChecksumIEEE([]byte(key))
	// Top bit set to mark as shortcut, bottom bit set
	return (crc | 0x80000000) | 0x02000000
}

// ConvertToShortcutInfo converts a protocol.ShortcutConfig to protocol.ShortcutInfo.
func ConvertToShortcutInfo(cfg protocol.ShortcutConfig) protocol.ShortcutInfo {
	return protocol.ShortcutInfo{
		AppID:         GenerateAppID(cfg.Exe, cfg.Name),
		Name:          cfg.Name,
		Exe:           cfg.Exe,
		StartDir:      cfg.StartDir,
		LaunchOptions: cfg.LaunchOptions,
		Tags:          cfg.Tags,
	}
}

// ArtworkPaths returns all artwork paths for a shortcut.
func (m *ShortcutManager) ArtworkPaths(userID string, appID uint32) map[ArtworkType]string {
	return map[ArtworkType]string{
		ArtworkGrid:     m.paths.ArtworkPath(userID, appID, ArtworkGrid, "png"),
		ArtworkHero:     m.paths.ArtworkPath(userID, appID, ArtworkHero, "png"),
		ArtworkLogo:     m.paths.ArtworkPath(userID, appID, ArtworkLogo, "png"),
		ArtworkIcon:     m.paths.ArtworkPath(userID, appID, ArtworkIcon, "png"),
		ArtworkPortrait: m.paths.ArtworkPath(userID, appID, ArtworkPortrait, "png"),
	}
}

// FindExistingArtwork finds existing artwork files for an appID.
func (m *ShortcutManager) FindExistingArtwork(userID string, appID uint32) (map[ArtworkType]string, error) {
	gridDir := m.paths.GridDir(userID)
	result := make(map[ArtworkType]string)

	extensions := []string{"png", "jpg", "jpeg", "ico"}
	artTypes := map[ArtworkType]string{
		ArtworkGrid:     fmt.Sprintf("%d", appID),
		ArtworkHero:     fmt.Sprintf("%d_hero", appID),
		ArtworkLogo:     fmt.Sprintf("%d_logo", appID),
		ArtworkIcon:     fmt.Sprintf("%d_icon", appID),
		ArtworkPortrait: fmt.Sprintf("%dp", appID),
	}

	for artType, baseName := range artTypes {
		for _, ext := range extensions {
			path := filepath.Join(gridDir, baseName+"."+ext)
			if _, err := os.Stat(path); err == nil {
				result[artType] = path
				break
			}
		}
	}

	return result, nil
}

// SaveArtwork saves artwork data to the appropriate path.
func (m *ShortcutManager) SaveArtwork(userID string, appID uint32, artType ArtworkType, data []byte, ext string) error {
	if err := m.EnsureGridDir(userID); err != nil {
		return fmt.Errorf("failed to create grid dir: %w", err)
	}

	// Clean extension
	ext = strings.TrimPrefix(ext, ".")
	if ext == "" {
		ext = "png"
	}

	path := m.paths.ArtworkPath(userID, appID, artType, ext)
	return os.WriteFile(path, data, 0644)
}

// DeleteArtwork removes all artwork for an appID.
func (m *ShortcutManager) DeleteArtwork(userID string, appID uint32) error {
	existing, err := m.FindExistingArtwork(userID, appID)
	if err != nil {
		return err
	}

	for _, path := range existing {
		if err := os.Remove(path); err != nil && !os.IsNotExist(err) {
			return fmt.Errorf("failed to remove %s: %w", path, err)
		}
	}

	return nil
}
