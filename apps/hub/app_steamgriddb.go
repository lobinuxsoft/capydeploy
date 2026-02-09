package main

import (
	"encoding/base64"
	"fmt"
	"mime"
	"os"
	"path/filepath"
	"strings"

	"github.com/wailsapp/wails/v2/pkg/runtime"

	"github.com/lobinuxsoft/capydeploy/pkg/config"
	"github.com/lobinuxsoft/capydeploy/pkg/steamgriddb"
)

// steamgriddbClient creates a SteamGridDB client using the configured API key.
func (a *App) steamgriddbClient() (*steamgriddb.Client, error) {
	apiKey, err := config.GetSteamGridDBAPIKey()
	if err != nil || apiKey == "" {
		return nil, fmt.Errorf("SteamGridDB API key not configured")
	}
	return steamgriddb.NewClient(apiKey), nil
}

// SearchGames searches for games on SteamGridDB
func (a *App) SearchGames(query string) ([]steamgriddb.SearchResult, error) {
	client, err := a.steamgriddbClient()
	if err != nil {
		return nil, err
	}
	return client.Search(query)
}

// GetGrids returns grid images for a game
func (a *App) GetGrids(gameID int, filters steamgriddb.ImageFilters, page int) ([]steamgriddb.GridData, error) {
	client, err := a.steamgriddbClient()
	if err != nil {
		return nil, err
	}
	return client.GetGrids(gameID, &filters, page)
}

// GetHeroes returns hero images for a game
func (a *App) GetHeroes(gameID int, filters steamgriddb.ImageFilters, page int) ([]steamgriddb.ImageData, error) {
	client, err := a.steamgriddbClient()
	if err != nil {
		return nil, err
	}
	return client.GetHeroes(gameID, &filters, page)
}

// GetLogos returns logo images for a game
func (a *App) GetLogos(gameID int, filters steamgriddb.ImageFilters, page int) ([]steamgriddb.ImageData, error) {
	client, err := a.steamgriddbClient()
	if err != nil {
		return nil, err
	}
	return client.GetLogos(gameID, &filters, page)
}

// GetIcons returns icon images for a game
func (a *App) GetIcons(gameID int, filters steamgriddb.ImageFilters, page int) ([]steamgriddb.ImageData, error) {
	client, err := a.steamgriddbClient()
	if err != nil {
		return nil, err
	}
	return client.GetIcons(gameID, &filters, page)
}

// SelectArtworkFile opens a file dialog to select a local artwork image.
func (a *App) SelectArtworkFile() (*ArtworkFileResult, error) {
	path, err := runtime.OpenFileDialog(a.ctx, runtime.OpenDialogOptions{
		Title: "Select Artwork Image",
		Filters: []runtime.FileFilter{
			{DisplayName: "Images", Pattern: "*.png;*.jpg;*.jpeg;*.webp"},
		},
	})
	if err != nil {
		return nil, err
	}
	if path == "" {
		return nil, nil // User cancelled
	}

	return readArtworkFile(path)
}

// GetArtworkPreview returns a data URI for the given artwork file path.
func (a *App) GetArtworkPreview(path string) (string, error) {
	result, err := readArtworkFile(path)
	if err != nil {
		return "", err
	}
	return result.DataURI, nil
}

// readArtworkFile reads and validates a local artwork file.
func readArtworkFile(path string) (*ArtworkFileResult, error) {
	info, err := os.Stat(path)
	if err != nil {
		return nil, fmt.Errorf("failed to stat file: %w", err)
	}

	if info.Size() > maxArtworkSize {
		return nil, fmt.Errorf("file too large: %d bytes (max %d)", info.Size(), maxArtworkSize)
	}

	contentType := detectContentType(path)
	if contentType == "" {
		return nil, fmt.Errorf("unsupported image format: %s", filepath.Ext(path))
	}

	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read file: %w", err)
	}

	dataURI := fmt.Sprintf("data:%s;base64,%s", contentType, base64.StdEncoding.EncodeToString(data))

	return &ArtworkFileResult{
		Path:        path,
		DataURI:     dataURI,
		ContentType: contentType,
		Size:        info.Size(),
	}, nil
}

// detectContentType returns the MIME type based on file extension.
func detectContentType(path string) string {
	ext := strings.ToLower(filepath.Ext(path))
	switch ext {
	case ".png":
		return "image/png"
	case ".jpg", ".jpeg":
		return "image/jpeg"
	case ".webp":
		return "image/webp"
	default:
		// Fallback to mime package
		ct := mime.TypeByExtension(ext)
		if strings.HasPrefix(ct, "image/") {
			return ct
		}
		return ""
	}
}
