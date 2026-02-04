package steamgriddb

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"strconv"
	"strings"
)

const baseURL = "https://www.steamgriddb.com/api/v2"

// Client is a SteamGridDB API client
type Client struct {
	apiKey     string
	httpClient http.Client
}

// NewClient creates a new SteamGridDB client
func NewClient(apiKey string) *Client {
	return &Client{apiKey: apiKey}
}

func (c *Client) get(endpoint string, params url.Values) ([]byte, error) {
	reqURL := baseURL + endpoint
	if len(params) > 0 {
		reqURL += "?" + params.Encode()
	}

	req, err := http.NewRequest("GET", reqURL, nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Authorization", "Bearer "+c.apiKey)

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	body, _ := io.ReadAll(resp.Body)

	if resp.StatusCode != 200 {
		return nil, fmt.Errorf("API error %d: %s", resp.StatusCode, string(body))
	}

	return body, nil
}

// Search searches for games by name
func (c *Client) Search(term string) ([]SearchResult, error) {
	body, err := c.get("/search/autocomplete/"+url.PathEscape(term), nil)
	if err != nil {
		return nil, err
	}

	var resp searchResponse
	if err := json.Unmarshal(body, &resp); err != nil {
		return nil, err
	}

	return resp.Data, nil
}

// GetGrids returns grid images for a game
func (c *Client) GetGrids(gameID int, filters *ImageFilters, page int) ([]GridData, error) {
	params := buildParams(filters, page)
	body, err := c.get(fmt.Sprintf("/grids/game/%d", gameID), params)
	if err != nil {
		return nil, err
	}

	var resp gridResponse
	if err := json.Unmarshal(body, &resp); err != nil {
		return nil, err
	}

	// Debug: log first result
	if len(resp.Data) > 0 {
		fmt.Printf("[DEBUG] First grid: URL=%s, Thumb=%s, %dx%d\n",
			resp.Data[0].URL, resp.Data[0].Thumb, resp.Data[0].Width, resp.Data[0].Height)
	}

	return resp.Data, nil
}

// GetHeroes returns hero images for a game
func (c *Client) GetHeroes(gameID int, filters *ImageFilters, page int) ([]ImageData, error) {
	params := buildParams(filters, page)
	body, err := c.get(fmt.Sprintf("/heroes/game/%d", gameID), params)
	if err != nil {
		return nil, err
	}

	var resp imageResponse
	if err := json.Unmarshal(body, &resp); err != nil {
		return nil, err
	}

	return resp.Data, nil
}

// GetLogos returns logo images for a game
func (c *Client) GetLogos(gameID int, filters *ImageFilters, page int) ([]ImageData, error) {
	params := buildParams(filters, page)
	body, err := c.get(fmt.Sprintf("/logos/game/%d", gameID), params)
	if err != nil {
		return nil, err
	}

	var resp imageResponse
	if err := json.Unmarshal(body, &resp); err != nil {
		return nil, err
	}

	return resp.Data, nil
}

// GetIcons returns icon images for a game
func (c *Client) GetIcons(gameID int, filters *ImageFilters, page int) ([]ImageData, error) {
	params := buildParams(filters, page)
	body, err := c.get(fmt.Sprintf("/icons/game/%d", gameID), params)
	if err != nil {
		return nil, err
	}

	var resp imageResponse
	if err := json.Unmarshal(body, &resp); err != nil {
		return nil, err
	}

	return resp.Data, nil
}

func buildParams(filters *ImageFilters, page int) url.Values {
	params := url.Values{}

	if filters != nil {
		if filters.Style != "" && filters.Style != "All Styles" {
			params.Set("styles", filters.Style)
		}
		if filters.MimeType != "" && filters.MimeType != "All Formats" {
			params.Set("mimes", filters.MimeType)
		}
		// Map frontend animation filter values to API values
		switch filters.ImageType {
		case "static", "Static Only":
			params.Set("types", "static")
		case "animated", "Animated Only":
			params.Set("types", "animated")
		}
		if filters.Dimension != "" && filters.Dimension != "All Sizes" {
			params.Set("dimensions", filters.Dimension)
		}
		if filters.ShowNsfw {
			params.Set("nsfw", "any")
		} else {
			params.Set("nsfw", "false")
		}
		if filters.ShowHumor {
			params.Set("humor", "any")
		} else {
			params.Set("humor", "false")
		}
	}

	if page > 0 {
		params.Set("page", strconv.Itoa(page))
	}

	return params
}

// GetImageCacheDir returns the path to the image cache directory
func GetImageCacheDir() (string, error) {
	configDir, err := os.UserConfigDir()
	if err != nil {
		home, err := os.UserHomeDir()
		if err != nil {
			return "", err
		}
		configDir = home
	}
	cacheDir := filepath.Join(configDir, "capydeploy", "cache", "images")
	if err := os.MkdirAll(cacheDir, 0755); err != nil {
		return "", err
	}
	return cacheDir, nil
}

// GetGameCacheDir returns the cache directory for a specific game
func GetGameCacheDir(gameID int) (string, error) {
	baseDir, err := GetImageCacheDir()
	if err != nil {
		return "", err
	}
	gameDir := filepath.Join(baseDir, fmt.Sprintf("game_%d", gameID))
	if err := os.MkdirAll(gameDir, 0755); err != nil {
		return "", err
	}
	return gameDir, nil
}

// hashURL creates a safe filename from a URL
func hashURL(url string) string {
	h := sha256.Sum256([]byte(url))
	return hex.EncodeToString(h[:16]) // Use first 16 bytes (32 hex chars)
}

// GetCachedImage returns the cached image data if it exists
func GetCachedImage(gameID int, imageURL string) ([]byte, string, error) {
	gameDir, err := GetGameCacheDir(gameID)
	if err != nil {
		return nil, "", err
	}

	// Find file with matching hash prefix
	hash := hashURL(imageURL)
	entries, err := os.ReadDir(gameDir)
	if err != nil {
		return nil, "", err
	}

	for _, entry := range entries {
		if strings.HasPrefix(entry.Name(), hash) {
			filePath := filepath.Join(gameDir, entry.Name())
			data, err := os.ReadFile(filePath)
			if err != nil {
				return nil, "", err
			}
			// Extract content type from extension
			ext := filepath.Ext(entry.Name())
			contentType := "image/jpeg"
			switch ext {
			case ".png":
				contentType = "image/png"
			case ".webp":
				contentType = "image/webp"
			case ".gif":
				contentType = "image/gif"
			}
			return data, contentType, nil
		}
	}

	return nil, "", os.ErrNotExist
}

// SaveImageToCache saves image data to the cache
func SaveImageToCache(gameID int, imageURL string, data []byte, contentType string) error {
	gameDir, err := GetGameCacheDir(gameID)
	if err != nil {
		return err
	}

	hash := hashURL(imageURL)
	ext := ".jpg"
	switch contentType {
	case "image/png":
		ext = ".png"
	case "image/webp":
		ext = ".webp"
	case "image/gif":
		ext = ".gif"
	}

	filePath := filepath.Join(gameDir, hash+ext)
	return os.WriteFile(filePath, data, 0644)
}

// GetCachedImagePath returns the file path of a cached image if it exists
func GetCachedImagePath(gameID int, imageURL string) (string, error) {
	gameDir, err := GetGameCacheDir(gameID)
	if err != nil {
		return "", err
	}

	hash := hashURL(imageURL)
	entries, err := os.ReadDir(gameDir)
	if err != nil {
		return "", err
	}

	for _, entry := range entries {
		if strings.HasPrefix(entry.Name(), hash) {
			return filepath.Join(gameDir, entry.Name()), nil
		}
	}

	return "", os.ErrNotExist
}

// ClearImageCache clears the image cache (including all game subdirectories)
func ClearImageCache() error {
	cacheDir, err := GetImageCacheDir()
	if err != nil {
		return err
	}

	entries, err := os.ReadDir(cacheDir)
	if err != nil {
		return err
	}

	for _, entry := range entries {
		path := filepath.Join(cacheDir, entry.Name())
		if entry.IsDir() {
			os.RemoveAll(path)
		} else {
			os.Remove(path)
		}
	}

	return nil
}

// GetCacheSize returns the total size of the image cache (including subdirectories)
func GetCacheSize() (int64, error) {
	cacheDir, err := GetImageCacheDir()
	if err != nil {
		return 0, err
	}

	var size int64
	err = filepath.Walk(cacheDir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return nil // Skip errors
		}
		if !info.IsDir() {
			size += info.Size()
		}
		return nil
	})

	return size, err
}
