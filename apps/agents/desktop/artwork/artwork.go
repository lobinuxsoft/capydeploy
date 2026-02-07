// Package artwork provides Steam artwork application for the Agent.
package artwork

import (
	"fmt"
	"os"
	"path/filepath"

	ssmSteam "github.com/shadowblip/steam-shortcut-manager/pkg/steam"
)

// ArtworkResult contains the result of applying a single artwork type.
type ArtworkResult struct {
	Type  string `json:"type"`
	Error string `json:"error,omitempty"`
}

// ApplyResult contains the result of applying artwork.
type ApplyResult struct {
	Applied []string        `json:"applied"`
	Failed  []ArtworkResult `json:"failed,omitempty"`
}

// ApplyFromData writes raw image bytes to the Steam grid directory.
// artworkType: "grid", "banner", "hero", "logo", "icon".
// contentType: "image/png", "image/jpeg", "image/webp".
func ApplyFromData(appID uint32, artworkType string, data []byte, contentType string) error {
	ext := extFromContentType(contentType)
	if ext == "" {
		return fmt.Errorf("unsupported content type: %s", contentType)
	}

	suffix, ok := artworkSuffix(artworkType)
	if !ok {
		return fmt.Errorf("unknown artwork type: %s", artworkType)
	}

	// Get Steam users to find grid directory
	users, err := ssmSteam.GetUsers()
	if err != nil {
		return fmt.Errorf("failed to get steam users: %w", err)
	}
	if len(users) == 0 {
		return fmt.Errorf("no steam users found")
	}

	gridDir, err := ssmSteam.GetImagesDir(users[0])
	if err != nil {
		return fmt.Errorf("failed to get grid directory: %w", err)
	}

	if err := os.MkdirAll(gridDir, 0755); err != nil {
		return fmt.Errorf("failed to create grid directory: %w", err)
	}

	// Remove existing files with different extensions to avoid conflicts
	removeExistingArtwork(gridDir, appID, suffix)

	filename := fmt.Sprintf("%d%s.%s", appID, suffix, ext)
	destPath := filepath.Join(gridDir, filename)

	if err := os.WriteFile(destPath, data, 0644); err != nil {
		return fmt.Errorf("failed to write artwork: %w", err)
	}

	return nil
}

// artworkSuffix returns the Steam grid filename suffix for each artwork type.
func artworkSuffix(artworkType string) (string, bool) {
	switch artworkType {
	case "grid":
		return "p", true
	case "banner":
		return "", true
	case "hero":
		return "_hero", true
	case "logo":
		return "_logo", true
	case "icon":
		return "_icon", true
	default:
		return "", false
	}
}

// extFromContentType returns the file extension for a content type.
func extFromContentType(contentType string) string {
	switch contentType {
	case "image/png":
		return "png"
	case "image/jpeg":
		return "jpg"
	case "image/webp":
		return "webp"
	default:
		return ""
	}
}

// removeExistingArtwork removes previous artwork files with any extension.
func removeExistingArtwork(gridDir string, appID uint32, suffix string) {
	base := fmt.Sprintf("%d%s", appID, suffix)
	for _, ext := range []string{"png", "jpg", "jpeg", "webp", "ico"} {
		path := filepath.Join(gridDir, fmt.Sprintf("%s.%s", base, ext))
		os.Remove(path)
	}
}
