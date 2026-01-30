// Package artwork provides Steam artwork application for the Agent.
package artwork

import (
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/steam"
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

// Apply downloads artwork from URLs and saves it to the Steam grid folder.
func Apply(userID string, appID uint32, cfg *protocol.ArtworkConfig) (*ApplyResult, error) {
	if cfg == nil {
		return &ApplyResult{Applied: []string{}}, nil
	}

	paths, err := steam.NewPaths()
	if err != nil {
		return nil, fmt.Errorf("failed to get Steam paths: %w", err)
	}

	// Ensure grid directory exists
	if err := paths.EnsureGridDir(userID); err != nil {
		return nil, fmt.Errorf("failed to create grid directory: %w", err)
	}

	gridDir := paths.GridDir(userID)
	result := &ApplyResult{
		Applied: []string{},
		Failed:  []ArtworkResult{},
	}

	// Apply each artwork type
	artworks := []struct {
		name    string
		url     string
		artType steam.ArtworkType
	}{
		{"grid", cfg.Grid, steam.ArtworkPortrait},
		{"banner", cfg.Banner, steam.ArtworkGrid},
		{"hero", cfg.Hero, steam.ArtworkHero},
		{"logo", cfg.Logo, steam.ArtworkLogo},
		{"icon", cfg.Icon, steam.ArtworkIcon},
	}

	for _, art := range artworks {
		if art.url == "" {
			continue
		}

		if err := downloadAndSave(art.url, gridDir, appID, art.artType); err != nil {
			result.Failed = append(result.Failed, ArtworkResult{
				Type:  art.name,
				Error: err.Error(),
			})
		} else {
			result.Applied = append(result.Applied, art.name)
		}
	}

	return result, nil
}

// downloadAndSave downloads an image from a URL and saves it to the grid folder.
func downloadAndSave(url, gridDir string, appID uint32, artType steam.ArtworkType) error {
	resp, err := http.Get(url)
	if err != nil {
		return fmt.Errorf("download failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("download failed: HTTP %d", resp.StatusCode)
	}

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return fmt.Errorf("failed to read data: %w", err)
	}

	ext := getExtension(resp, url)
	filename := artworkFilename(appID, artType, ext)
	destPath := filepath.Join(gridDir, filename)

	if err := os.WriteFile(destPath, data, 0644); err != nil {
		return fmt.Errorf("failed to save: %w", err)
	}

	return nil
}

// artworkFilename generates the filename for artwork based on type.
func artworkFilename(appID uint32, artType steam.ArtworkType, ext string) string {
	switch artType {
	case steam.ArtworkGrid:
		return fmt.Sprintf("%d%s", appID, ext)
	case steam.ArtworkHero:
		return fmt.Sprintf("%d_hero%s", appID, ext)
	case steam.ArtworkLogo:
		return fmt.Sprintf("%d_logo%s", appID, ext)
	case steam.ArtworkIcon:
		return fmt.Sprintf("%d_icon%s", appID, ext)
	case steam.ArtworkPortrait:
		return fmt.Sprintf("%dp%s", appID, ext)
	default:
		return fmt.Sprintf("%d%s", appID, ext)
	}
}

// getExtension determines file extension from HTTP response or URL.
func getExtension(resp *http.Response, url string) string {
	contentType := resp.Header.Get("Content-Type")

	switch {
	case strings.Contains(contentType, "png"):
		return ".png"
	case strings.Contains(contentType, "jpeg"), strings.Contains(contentType, "jpg"):
		return ".jpg"
	case strings.Contains(contentType, "webp"):
		return ".webp"
	case strings.Contains(contentType, "gif"):
		return ".gif"
	}

	// Fallback to URL extension
	urlPath := url
	if idx := strings.Index(url, "?"); idx != -1 {
		urlPath = url[:idx]
	}
	urlLower := strings.ToLower(urlPath)

	switch {
	case strings.HasSuffix(urlLower, ".webp"):
		return ".webp"
	case strings.HasSuffix(urlLower, ".png"):
		return ".png"
	case strings.HasSuffix(urlLower, ".jpg"), strings.HasSuffix(urlLower, ".jpeg"):
		return ".jpg"
	case strings.HasSuffix(urlLower, ".gif"):
		return ".gif"
	default:
		return ".png"
	}
}
