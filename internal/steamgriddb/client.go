package steamgriddb

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"strconv"
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
	cacheDir := filepath.Join(configDir, "bazzite-devkit", "cache", "images")
	if err := os.MkdirAll(cacheDir, 0755); err != nil {
		return "", err
	}
	return cacheDir, nil
}

// ClearImageCache clears the image cache
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
		os.Remove(filepath.Join(cacheDir, entry.Name()))
	}

	return nil
}

// GetCacheSize returns the total size of the image cache
func GetCacheSize() (int64, error) {
	cacheDir, err := GetImageCacheDir()
	if err != nil {
		return 0, err
	}

	var size int64
	entries, err := os.ReadDir(cacheDir)
	if err != nil {
		return 0, err
	}

	for _, entry := range entries {
		info, err := entry.Info()
		if err != nil {
			continue
		}
		size += info.Size()
	}

	return size, nil
}
