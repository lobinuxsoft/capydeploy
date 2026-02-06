package steamgriddb

import (
	"bytes"
	"fmt"
	"image"
	"image/gif"
	"image/jpeg"
	"image/png"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"

	"golang.org/x/image/draw"
)

// ThumbnailConfig defines thumbnail generation parameters
type ThumbnailConfig struct {
	MaxWidth  int // Maximum width in pixels
	MaxHeight int // Maximum height in pixels (0 = proportional)
	Quality   int // JPEG quality (1-100)
}

// DefaultThumbnailConfig returns sensible defaults for grid thumbnails
func DefaultThumbnailConfig() ThumbnailConfig {
	return ThumbnailConfig{
		MaxWidth:  200,
		MaxHeight: 0, // Proportional
		Quality:   70,
	}
}

// GetThumbnailCacheDir returns the directory for static thumbnails
func GetThumbnailCacheDir(gameID int) (string, error) {
	baseDir, err := GetImageCacheDir()
	if err != nil {
		return "", err
	}
	thumbDir := filepath.Join(baseDir, fmt.Sprintf("game_%d", gameID), "thumbs")
	if err := os.MkdirAll(thumbDir, 0755); err != nil {
		return "", err
	}
	return thumbDir, nil
}

// GetStaticThumbnail returns the path to a static thumbnail, generating it if needed.
// For animated images (GIF/WebP), it extracts the first frame.
// All thumbnails are saved as compressed JPEG.
func GetStaticThumbnail(gameID int, imageURL string, cfg ThumbnailConfig) (string, error) {
	if gameID <= 0 || imageURL == "" {
		return "", fmt.Errorf("invalid gameID or imageURL")
	}

	thumbDir, err := GetThumbnailCacheDir(gameID)
	if err != nil {
		return "", err
	}

	// Check if thumbnail already exists
	hash := hashURL(imageURL)
	thumbPath := filepath.Join(thumbDir, hash+".jpg")
	if _, err := os.Stat(thumbPath); err == nil {
		return thumbPath, nil
	}

	// Download the image
	resp, err := http.Get(imageURL)
	if err != nil {
		return "", fmt.Errorf("failed to fetch image: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("HTTP error: %d", resp.StatusCode)
	}

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("failed to read image: %w", err)
	}

	// Detect content type
	contentType := resp.Header.Get("Content-Type")
	if contentType == "" {
		contentType = http.DetectContentType(data)
	}

	// Decode image (first frame if animated)
	img, err := decodeFirstFrame(data, contentType, imageURL)
	if err != nil {
		return "", fmt.Errorf("failed to decode image: %w", err)
	}

	// Resize if needed
	img = resizeImage(img, cfg.MaxWidth, cfg.MaxHeight)

	// Encode as JPEG
	var buf bytes.Buffer
	if err := jpeg.Encode(&buf, img, &jpeg.Options{Quality: cfg.Quality}); err != nil {
		return "", fmt.Errorf("failed to encode thumbnail: %w", err)
	}

	// Save to cache
	if err := os.WriteFile(thumbPath, buf.Bytes(), 0644); err != nil {
		return "", fmt.Errorf("failed to save thumbnail: %w", err)
	}

	return thumbPath, nil
}

// decodeFirstFrame decodes an image, extracting only the first frame for animated formats
func decodeFirstFrame(data []byte, contentType, url string) (image.Image, error) {
	reader := bytes.NewReader(data)

	// Handle GIF - extract first frame
	if strings.Contains(contentType, "gif") || strings.HasSuffix(strings.ToLower(url), ".gif") {
		g, err := gif.DecodeAll(reader)
		if err != nil {
			return nil, err
		}
		if len(g.Image) == 0 {
			return nil, fmt.Errorf("empty GIF")
		}
		// Return first frame
		return g.Image[0], nil
	}

	// Handle PNG
	if strings.Contains(contentType, "png") || strings.HasSuffix(strings.ToLower(url), ".png") {
		return png.Decode(reader)
	}

	// Handle JPEG
	if strings.Contains(contentType, "jpeg") || strings.Contains(contentType, "jpg") {
		return jpeg.Decode(reader)
	}

	// Handle WebP - try standard image.Decode (works for static WebP)
	// Note: For animated WebP, this will only get the first frame or fail
	// We'll need golang.org/x/image/webp for proper support
	if strings.Contains(contentType, "webp") || strings.HasSuffix(strings.ToLower(url), ".webp") {
		// Try generic decode - may work for static WebP
		reader.Seek(0, 0)
		img, _, err := image.Decode(reader)
		if err != nil {
			// WebP not supported by standard library, return placeholder error
			return nil, fmt.Errorf("WebP decode not supported: %w", err)
		}
		return img, nil
	}

	// Try generic decode for other formats
	reader.Seek(0, 0)
	img, _, err := image.Decode(reader)
	return img, err
}

// resizeImage resizes an image to fit within maxWidth x maxHeight while preserving aspect ratio
func resizeImage(img image.Image, maxWidth, maxHeight int) image.Image {
	bounds := img.Bounds()
	origWidth := bounds.Dx()
	origHeight := bounds.Dy()

	// Calculate new dimensions
	newWidth := origWidth
	newHeight := origHeight

	if maxWidth > 0 && origWidth > maxWidth {
		newWidth = maxWidth
		newHeight = (origHeight * maxWidth) / origWidth
	}

	if maxHeight > 0 && newHeight > maxHeight {
		newHeight = maxHeight
		newWidth = (origWidth * maxHeight) / origHeight
	}

	// No resize needed
	if newWidth == origWidth && newHeight == origHeight {
		return img
	}

	// Create new image and resize using high-quality interpolation
	dst := image.NewRGBA(image.Rect(0, 0, newWidth, newHeight))
	draw.CatmullRom.Scale(dst, dst.Bounds(), img, bounds, draw.Over, nil)

	return dst
}

// GetCachedThumbnailPath returns the path to an existing thumbnail, or empty string if not cached
func GetCachedThumbnailPath(gameID int, imageURL string) string {
	thumbDir, err := GetThumbnailCacheDir(gameID)
	if err != nil {
		return ""
	}

	hash := hashURL(imageURL)
	thumbPath := filepath.Join(thumbDir, hash+".jpg")
	if _, err := os.Stat(thumbPath); err == nil {
		return thumbPath
	}
	return ""
}
