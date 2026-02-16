// Package steam provides local Steam integration for shortcuts and artwork management.
package steam

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
)

// Common errors for Steam operations.
var (
	ErrSteamNotFound   = errors.New("steam installation not found")
	ErrUserNotFound    = errors.New("steam user not found")
	ErrImageNotFound   = errors.New("image not found")
	ErrShortcutsNotFound = errors.New("shortcuts.vdf not found")
)

// Paths provides access to Steam directory paths.
type Paths struct {
	baseDir string
}

// NewPaths creates a new Paths instance with auto-detected Steam directory.
func NewPaths() (*Paths, error) {
	baseDir, err := getBaseDir()
	if err != nil {
		return nil, err
	}
	return &Paths{baseDir: baseDir}, nil
}

// NewPathsWithBase creates a new Paths instance with a custom base directory.
func NewPathsWithBase(baseDir string) *Paths {
	return &Paths{baseDir: baseDir}
}

// BaseDir returns the Steam base directory.
func (p *Paths) BaseDir() string {
	return p.baseDir
}

// UserDataDir returns the userdata directory.
func (p *Paths) UserDataDir() string {
	return filepath.Join(p.baseDir, "userdata")
}

// UserDir returns the directory for a specific user.
func (p *Paths) UserDir(userID string) string {
	return filepath.Join(p.UserDataDir(), userID)
}

// ConfigDir returns the config directory for a user.
func (p *Paths) ConfigDir(userID string) string {
	return filepath.Join(p.UserDir(userID), "config")
}

// ShortcutsPath returns the path to shortcuts.vdf for a user.
func (p *Paths) ShortcutsPath(userID string) string {
	return filepath.Join(p.ConfigDir(userID), "shortcuts.vdf")
}

// GridDir returns the grid artwork directory for a user.
func (p *Paths) GridDir(userID string) string {
	return filepath.Join(p.ConfigDir(userID), "grid")
}

// HasShortcuts returns true if the user has a shortcuts.vdf file.
func (p *Paths) HasShortcuts(userID string) bool {
	_, err := os.Stat(p.ShortcutsPath(userID))
	return err == nil
}

// EnsureGridDir creates the grid directory if it doesn't exist.
func (p *Paths) EnsureGridDir(userID string) error {
	return os.MkdirAll(p.GridDir(userID), 0755)
}

// ArtworkPath returns the path for a specific artwork type.
func (p *Paths) ArtworkPath(userID string, appID uint32, artType ArtworkType, ext string) string {
	return filepath.Join(p.GridDir(userID), artworkFilename(appID, artType, ext))
}

// ArtworkType represents the type of Steam artwork.
type ArtworkType int

const (
	ArtworkGrid   ArtworkType = iota // 460x215 horizontal banner
	ArtworkHero                      // 1920x620 header
	ArtworkLogo                      // transparent logo
	ArtworkIcon                      // square icon
	ArtworkPortrait                  // 600x900 vertical grid
)

// artworkFilename generates the filename for artwork based on type.
func artworkFilename(appID uint32, artType ArtworkType, ext string) string {
	switch artType {
	case ArtworkGrid:
		return formatFilename(appID, "", ext)
	case ArtworkHero:
		return formatFilename(appID, "_hero", ext)
	case ArtworkLogo:
		return formatFilename(appID, "_logo", ext)
	case ArtworkIcon:
		return formatFilename(appID, "_icon", ext)
	case ArtworkPortrait:
		return formatFilename(appID, "p", ext)
	default:
		return formatFilename(appID, "", ext)
	}
}

func formatFilename(appID uint32, suffix, ext string) string {
	if ext == "" {
		ext = "png"
	}
	return fmt.Sprintf("%d%s.%s", appID, suffix, ext)
}
