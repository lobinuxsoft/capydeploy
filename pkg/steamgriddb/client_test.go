package steamgriddb

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// newTestClient creates a Client pointing at the given test server URL.
func newTestClient(serverURL string) *Client {
	c := NewClient("test-api-key")
	// Override the baseURL by injecting the test server URL into the client's transport.
	// Since Client uses the default httpClient, we redirect by intercepting at transport level.
	c.httpClient = http.Client{
		Transport: &rewriteTransport{
			base:    http.DefaultTransport,
			baseURL: serverURL,
		},
	}
	return c
}

// rewriteTransport rewrites requests to point at a test server.
type rewriteTransport struct {
	base    http.RoundTripper
	baseURL string
}

func (t *rewriteTransport) RoundTrip(req *http.Request) (*http.Response, error) {
	// Replace the scheme+host with the test server, keep the path+query
	u, _ := url.Parse(t.baseURL)
	req.URL.Scheme = u.Scheme
	req.URL.Host = u.Host
	return t.base.RoundTrip(req)
}

func TestSearch(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Verify auth header
		if !strings.HasPrefix(r.Header.Get("Authorization"), "Bearer ") {
			t.Error("missing Bearer token")
		}

		if !strings.Contains(r.URL.Path, "/search/autocomplete/") {
			t.Errorf("unexpected path: %s", r.URL.Path)
		}

		resp := searchResponse{
			apiResponse: apiResponse{Success: true},
			Data: []SearchResult{
				{ID: 1, Name: "Test Game", Types: []string{"steam"}, Verified: true},
				{ID: 2, Name: "Test Game 2", Types: []string{"origin"}},
			},
		}
		json.NewEncoder(w).Encode(resp)
	}))
	defer srv.Close()

	client := newTestClient(srv.URL)
	results, err := client.Search("Test Game")
	if err != nil {
		t.Fatalf("Search() error = %v", err)
	}

	if len(results) != 2 {
		t.Fatalf("Search() returned %d results, want 2", len(results))
	}
	if results[0].Name != "Test Game" {
		t.Errorf("results[0].Name = %q, want %q", results[0].Name, "Test Game")
	}
}

func TestGetGrids(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !strings.Contains(r.URL.Path, "/grids/game/42") {
			t.Errorf("unexpected path: %s", r.URL.Path)
		}

		resp := imageResponse{
			apiResponse: apiResponse{Success: true},
			Data: []ImageData{
				{ID: 100, URL: "https://example.com/grid.png", Width: 920, Height: 430},
			},
		}
		json.NewEncoder(w).Encode(resp)
	}))
	defer srv.Close()

	client := newTestClient(srv.URL)
	grids, err := client.GetGrids(42, nil, 0)
	if err != nil {
		t.Fatalf("GetGrids() error = %v", err)
	}

	if len(grids) != 1 {
		t.Fatalf("GetGrids() returned %d results, want 1", len(grids))
	}
	if grids[0].Width != 920 {
		t.Errorf("grids[0].Width = %d, want 920", grids[0].Width)
	}
}

func TestGetHeroes(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if !strings.Contains(r.URL.Path, "/heroes/game/42") {
			t.Errorf("unexpected path: %s", r.URL.Path)
		}

		resp := imageResponse{
			apiResponse: apiResponse{Success: true},
			Data: []ImageData{
				{ID: 200, URL: "https://example.com/hero.png", Width: 1920, Height: 620},
			},
		}
		json.NewEncoder(w).Encode(resp)
	}))
	defer srv.Close()

	client := newTestClient(srv.URL)
	heroes, err := client.GetHeroes(42, nil, 0)
	if err != nil {
		t.Fatalf("GetHeroes() error = %v", err)
	}

	if len(heroes) != 1 {
		t.Fatalf("GetHeroes() returned %d results, want 1", len(heroes))
	}
}

func TestGetLogos(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		resp := imageResponse{
			apiResponse: apiResponse{Success: true},
			Data: []ImageData{
				{ID: 300, URL: "https://example.com/logo.png"},
			},
		}
		json.NewEncoder(w).Encode(resp)
	}))
	defer srv.Close()

	client := newTestClient(srv.URL)
	logos, err := client.GetLogos(42, nil, 0)
	if err != nil {
		t.Fatalf("GetLogos() error = %v", err)
	}

	if len(logos) != 1 {
		t.Fatalf("GetLogos() returned %d results, want 1", len(logos))
	}
}

func TestGetIcons(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		resp := imageResponse{
			apiResponse: apiResponse{Success: true},
			Data: []ImageData{
				{ID: 400, URL: "https://example.com/icon.png"},
			},
		}
		json.NewEncoder(w).Encode(resp)
	}))
	defer srv.Close()

	client := newTestClient(srv.URL)
	icons, err := client.GetIcons(42, nil, 0)
	if err != nil {
		t.Fatalf("GetIcons() error = %v", err)
	}

	if len(icons) != 1 {
		t.Fatalf("GetIcons() returned %d results, want 1", len(icons))
	}
}

func TestSearch_APIError(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusUnauthorized)
		w.Write([]byte(`{"success":false,"errors":["Unauthorized"]}`))
	}))
	defer srv.Close()

	client := newTestClient(srv.URL)
	_, err := client.Search("test")
	if err == nil {
		t.Error("Search() should return error on 401")
	}
	if !strings.Contains(err.Error(), "401") {
		t.Errorf("error should mention status code: %v", err)
	}
}

func TestBuildParams(t *testing.T) {
	tests := []struct {
		name    string
		filters *ImageFilters
		page    int
		want    map[string]string // expected param key→value pairs
		notWant []string          // keys that should NOT be present
	}{
		{
			name:    "nil filters, no page",
			filters: nil,
			page:    0,
			want:    map[string]string{},
		},
		{
			name:    "page only",
			filters: nil,
			page:    2,
			want:    map[string]string{"page": "2"},
		},
		{
			name: "style filter",
			filters: &ImageFilters{
				Style: "alternate",
			},
			page: 0,
			want: map[string]string{"styles": "alternate", "nsfw": "false", "humor": "false"},
		},
		{
			name: "all styles ignored",
			filters: &ImageFilters{
				Style: "All Styles",
			},
			page:    0,
			notWant: []string{"styles"},
		},
		{
			name: "static animation type",
			filters: &ImageFilters{
				ImageType: "static",
			},
			page: 0,
			want: map[string]string{"types": "static"},
		},
		{
			name: "animated frontend label",
			filters: &ImageFilters{
				ImageType: "Animated Only",
			},
			page: 0,
			want: map[string]string{"types": "animated"},
		},
		{
			name: "nsfw and humor enabled",
			filters: &ImageFilters{
				ShowNsfw:  true,
				ShowHumor: true,
			},
			page: 0,
			want: map[string]string{"nsfw": "any", "humor": "any"},
		},
		{
			name: "mime type filter",
			filters: &ImageFilters{
				MimeType: "image/png",
			},
			page: 0,
			want: map[string]string{"mimes": "image/png"},
		},
		{
			name: "dimension filter",
			filters: &ImageFilters{
				Dimension: "600x900",
			},
			page: 0,
			want: map[string]string{"dimensions": "600x900"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := buildParams(tt.filters, tt.page)

			for key, wantVal := range tt.want {
				if gotVal := got.Get(key); gotVal != wantVal {
					t.Errorf("buildParams()[%q] = %q, want %q", key, gotVal, wantVal)
				}
			}

			for _, key := range tt.notWant {
				if got.Has(key) {
					t.Errorf("buildParams() should not have key %q", key)
				}
			}
		})
	}
}

// --- Cache tests ---

func TestSaveAndGetCachedImage(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	gameID := 12345
	imageURL := "https://example.com/image.png"
	testData := []byte("fake-png-data")

	// Save
	if err := SaveImageToCache(gameID, imageURL, testData, "image/png"); err != nil {
		t.Fatalf("SaveImageToCache() error = %v", err)
	}

	// Get
	data, contentType, err := GetCachedImage(gameID, imageURL)
	if err != nil {
		t.Fatalf("GetCachedImage() error = %v", err)
	}
	if string(data) != string(testData) {
		t.Errorf("GetCachedImage() data mismatch")
	}
	if contentType != "image/png" {
		t.Errorf("GetCachedImage() contentType = %q, want %q", contentType, "image/png")
	}
}

func TestGetCachedImage_NotFound(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	_, _, err := GetCachedImage(99999, "https://example.com/nonexistent.png")
	if err == nil {
		t.Error("GetCachedImage() should return error for non-cached image")
	}
}

func TestClearImageCache(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	// Save some test data
	if err := SaveImageToCache(1, "https://example.com/a.png", []byte("aaa"), "image/png"); err != nil {
		t.Fatalf("SaveImageToCache() error = %v", err)
	}
	if err := SaveImageToCache(2, "https://example.com/b.jpg", []byte("bbb"), "image/jpeg"); err != nil {
		t.Fatalf("SaveImageToCache() error = %v", err)
	}

	if err := ClearImageCache(); err != nil {
		t.Fatalf("ClearImageCache() error = %v", err)
	}

	// Cache should be empty
	size, err := GetCacheSize()
	if err != nil {
		t.Fatalf("GetCacheSize() error = %v", err)
	}
	if size != 0 {
		t.Errorf("GetCacheSize() after clear = %d, want 0", size)
	}
}

func TestGetCacheSize(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	// Empty cache should be 0
	size, err := GetCacheSize()
	if err != nil {
		t.Fatalf("GetCacheSize() error = %v", err)
	}
	if size != 0 {
		t.Errorf("GetCacheSize() empty cache = %d, want 0", size)
	}

	// Save some data
	testData := []byte("some-image-data-here")
	if err := SaveImageToCache(1, "https://example.com/test.png", testData, "image/png"); err != nil {
		t.Fatalf("SaveImageToCache() error = %v", err)
	}

	size, err = GetCacheSize()
	if err != nil {
		t.Fatalf("GetCacheSize() error = %v", err)
	}
	if size != int64(len(testData)) {
		t.Errorf("GetCacheSize() = %d, want %d", size, len(testData))
	}
}

func TestSaveImageToCache_ContentTypes(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	tests := []struct {
		contentType string
		expectedExt string
	}{
		{"image/png", ".png"},
		{"image/jpeg", ".jpg"},
		{"image/webp", ".webp"},
		{"image/gif", ".gif"},
		{"image/unknown", ".jpg"}, // default
	}

	for _, tt := range tests {
		t.Run(tt.contentType, func(t *testing.T) {
			url := "https://example.com/" + tt.contentType
			if err := SaveImageToCache(1, url, []byte("test"), tt.contentType); err != nil {
				t.Fatalf("SaveImageToCache() error = %v", err)
			}

			// Verify file was saved with correct extension
			path, err := GetCachedImagePath(1, url)
			if err != nil {
				t.Fatalf("GetCachedImagePath() error = %v", err)
			}
			ext := filepath.Ext(path)
			if ext != tt.expectedExt {
				t.Errorf("saved file ext = %q, want %q", ext, tt.expectedExt)
			}
		})
	}
}

func TestHashURL(t *testing.T) {
	// Same URL should produce same hash
	h1 := hashURL("https://example.com/image.png")
	h2 := hashURL("https://example.com/image.png")
	if h1 != h2 {
		t.Errorf("hashURL() not deterministic: %q != %q", h1, h2)
	}

	// Different URLs should produce different hashes
	h3 := hashURL("https://example.com/other.png")
	if h1 == h3 {
		t.Errorf("hashURL() collision: %q == %q", h1, h3)
	}

	// Hash should be 32 hex chars (16 bytes)
	if len(h1) != 32 {
		t.Errorf("hashURL() length = %d, want 32", len(h1))
	}
}

func TestGetGameCacheDir(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	dir, err := GetGameCacheDir(42)
	if err != nil {
		t.Fatalf("GetGameCacheDir() error = %v", err)
	}

	if !strings.Contains(dir, "game_42") {
		t.Errorf("GetGameCacheDir() = %q, should contain 'game_42'", dir)
	}

	// Directory should exist
	info, err := os.Stat(dir)
	if err != nil {
		t.Fatalf("GetGameCacheDir() created dir does not exist: %v", err)
	}
	if !info.IsDir() {
		t.Error("GetGameCacheDir() did not create a directory")
	}
}
