// Package steamgriddb provides a client for the SteamGridDB API
package steamgriddb

// SearchResult represents a game search result
type SearchResult struct {
	ID       int      `json:"id"`
	Name     string   `json:"name"`
	Types    []string `json:"types"`
	Verified bool     `json:"verified"`
}

// ImageData represents a SteamGridDB image (grid, hero, logo, or icon).
type ImageData struct {
	ID        int    `json:"id"`
	Score     int    `json:"score"`
	Style     string `json:"style"`
	Width     int    `json:"width"`
	Height    int    `json:"height"`
	Nsfw      bool   `json:"nsfw"`
	Humor     bool   `json:"humor"`
	Mime      string `json:"mime"`
	Language  string `json:"language"`
	URL       string `json:"url"`
	Thumb     string `json:"thumb"`
	Lock      bool   `json:"lock"`
	Epilepsy  bool   `json:"epilepsy"`
	Upvotes   int    `json:"upvotes"`
	Downvotes int    `json:"downvotes"`
}

// GridData is an alias for ImageData (grids share the same API schema).
type GridData = ImageData

// ImageFilters represents filters for image queries
type ImageFilters struct {
	Style     string `json:"style"`
	MimeType  string `json:"mimeType"`
	ImageType string `json:"imageType"` // "static", "animated", or "" for all
	Dimension string `json:"dimension"`
	ShowNsfw  bool   `json:"showNsfw"`
	ShowHumor bool   `json:"showHumor"`
}

// API response types
type apiResponse struct {
	Success bool     `json:"success"`
	Errors  []string `json:"errors"`
}

type searchResponse struct {
	apiResponse
	Data []SearchResult `json:"data"`
}

type imageResponse struct {
	apiResponse
	Data []ImageData `json:"data"`
}
