package main

import (
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"

	"github.com/lobinuxsoft/capydeploy/pkg/steamgriddb"
)

// CacheHandler serves cached images via HTTP without base64 encoding.
// This avoids loading entire images into memory, especially important for
// large animated GIFs/WebPs that can be 5-20MB each.
type CacheHandler struct{}

// NewCacheHandler creates a new cache handler.
func NewCacheHandler() *CacheHandler {
	return &CacheHandler{}
}

// ServeHTTP handles requests to /cache/{gameID}/{filename} or /cache/{gameID}/thumbs/{filename}
func (h *CacheHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	// Parse path: /cache/{gameID}/{filename} or /cache/{gameID}/thumbs/{filename}
	path := strings.TrimPrefix(r.URL.Path, "/cache/")
	parts := strings.SplitN(path, "/", 3)
	if len(parts) < 2 {
		http.Error(w, "Invalid cache path", http.StatusBadRequest)
		return
	}

	gameID := parts[0]
	var subdir, filename string

	if len(parts) == 3 && parts[1] == "thumbs" {
		// /cache/{gameID}/thumbs/{filename}
		subdir = "thumbs"
		filename = parts[2]
	} else {
		// /cache/{gameID}/{filename}
		filename = parts[1]
	}

	// Validate filename to prevent path traversal
	if strings.Contains(filename, "..") || strings.Contains(filename, "/") {
		http.Error(w, "Invalid filename", http.StatusBadRequest)
		return
	}

	// Get cache directory
	cacheDir, err := steamgriddb.GetImageCacheDir()
	if err != nil {
		http.Error(w, "Cache not available", http.StatusInternalServerError)
		return
	}

	// Build full path
	var filePath string
	if subdir != "" {
		filePath = filepath.Join(cacheDir, fmt.Sprintf("game_%s", gameID), subdir, filename)
	} else {
		filePath = filepath.Join(cacheDir, fmt.Sprintf("game_%s", gameID), filename)
	}

	// Verify the file exists and is within the cache directory
	absPath, err := filepath.Abs(filePath)
	if err != nil {
		http.Error(w, "Invalid path", http.StatusBadRequest)
		return
	}

	absCacheDir, _ := filepath.Abs(cacheDir)
	if !strings.HasPrefix(absPath, absCacheDir) {
		http.Error(w, "Access denied", http.StatusForbidden)
		return
	}

	// Open and serve the file
	file, err := os.Open(absPath)
	if err != nil {
		if os.IsNotExist(err) {
			http.Error(w, "Not found", http.StatusNotFound)
		} else {
			http.Error(w, "Error reading file", http.StatusInternalServerError)
		}
		return
	}
	defer file.Close()

	// Set content type based on extension
	ext := strings.ToLower(filepath.Ext(filename))
	contentType := "image/jpeg"
	switch ext {
	case ".png":
		contentType = "image/png"
	case ".webp":
		contentType = "image/webp"
	case ".gif":
		contentType = "image/gif"
	}

	w.Header().Set("Content-Type", contentType)
	w.Header().Set("Cache-Control", "public, max-age=31536000") // Cache for 1 year

	// Stream file directly to response (no memory buffering)
	io.Copy(w, file)
}
