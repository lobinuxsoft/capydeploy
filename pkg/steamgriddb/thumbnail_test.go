package steamgriddb

import (
	"bytes"
	"image"
	"image/color"
	"image/gif"
	"image/jpeg"
	"image/png"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"
)

// createTestPNG creates a minimal valid PNG image in memory.
func createTestPNG(width, height int) []byte {
	img := image.NewRGBA(image.Rect(0, 0, width, height))
	for y := 0; y < height; y++ {
		for x := 0; x < width; x++ {
			img.Set(x, y, color.RGBA{R: 255, G: 0, B: 0, A: 255})
		}
	}
	var buf bytes.Buffer
	png.Encode(&buf, img)
	return buf.Bytes()
}

// createTestJPEG creates a minimal valid JPEG image in memory.
func createTestJPEG(width, height int) []byte {
	img := image.NewRGBA(image.Rect(0, 0, width, height))
	var buf bytes.Buffer
	jpeg.Encode(&buf, img, &jpeg.Options{Quality: 80})
	return buf.Bytes()
}

// createTestGIF creates a minimal valid GIF with 2 frames.
func createTestGIF(width, height int) []byte {
	palette := color.Palette{color.Black, color.White, color.RGBA{R: 255, A: 255}}

	g := &gif.GIF{}
	for i := 0; i < 2; i++ {
		frame := image.NewPaletted(image.Rect(0, 0, width, height), palette)
		for y := 0; y < height; y++ {
			for x := 0; x < width; x++ {
				frame.SetColorIndex(x, y, uint8(i%len(palette)))
			}
		}
		g.Image = append(g.Image, frame)
		g.Delay = append(g.Delay, 10)
	}

	var buf bytes.Buffer
	gif.EncodeAll(&buf, g)
	return buf.Bytes()
}

func TestResizeImage_Downscale(t *testing.T) {
	img := image.NewRGBA(image.Rect(0, 0, 800, 600))

	resized := resizeImage(img, 200, 0)

	bounds := resized.Bounds()
	if bounds.Dx() != 200 {
		t.Errorf("resized width = %d, want 200", bounds.Dx())
	}
	// Proportional: 600 * 200 / 800 = 150
	if bounds.Dy() != 150 {
		t.Errorf("resized height = %d, want 150", bounds.Dy())
	}
}

func TestResizeImage_NoResize(t *testing.T) {
	img := image.NewRGBA(image.Rect(0, 0, 100, 100))

	resized := resizeImage(img, 200, 0)

	// Image is smaller than max, should not resize
	if resized != img {
		t.Error("resizeImage() should return original when no resize needed")
	}
}

func TestResizeImage_MaxHeight(t *testing.T) {
	img := image.NewRGBA(image.Rect(0, 0, 400, 800))

	resized := resizeImage(img, 200, 100)

	bounds := resized.Bounds()
	if bounds.Dy() > 100 {
		t.Errorf("resized height = %d, want <= 100", bounds.Dy())
	}
}

func TestResizeImage_ZeroMax(t *testing.T) {
	img := image.NewRGBA(image.Rect(0, 0, 300, 200))

	resized := resizeImage(img, 0, 0)

	// No constraints, should return original
	if resized != img {
		t.Error("resizeImage(0,0) should return original")
	}
}

func TestDecodeFirstFrame_PNG(t *testing.T) {
	data := createTestPNG(100, 50)

	img, err := decodeFirstFrame(data, "image/png", "test.png")
	if err != nil {
		t.Fatalf("decodeFirstFrame(PNG) error = %v", err)
	}

	bounds := img.Bounds()
	if bounds.Dx() != 100 || bounds.Dy() != 50 {
		t.Errorf("decoded PNG size = %dx%d, want 100x50", bounds.Dx(), bounds.Dy())
	}
}

func TestDecodeFirstFrame_JPEG(t *testing.T) {
	data := createTestJPEG(120, 80)

	img, err := decodeFirstFrame(data, "image/jpeg", "test.jpg")
	if err != nil {
		t.Fatalf("decodeFirstFrame(JPEG) error = %v", err)
	}

	bounds := img.Bounds()
	if bounds.Dx() != 120 || bounds.Dy() != 80 {
		t.Errorf("decoded JPEG size = %dx%d, want 120x80", bounds.Dx(), bounds.Dy())
	}
}

func TestDecodeFirstFrame_GIF(t *testing.T) {
	data := createTestGIF(60, 40)

	img, err := decodeFirstFrame(data, "image/gif", "test.gif")
	if err != nil {
		t.Fatalf("decodeFirstFrame(GIF) error = %v", err)
	}

	bounds := img.Bounds()
	if bounds.Dx() != 60 || bounds.Dy() != 40 {
		t.Errorf("decoded GIF first frame size = %dx%d, want 60x40", bounds.Dx(), bounds.Dy())
	}
}

func TestDecodeFirstFrame_GIF_EmptyFrames(t *testing.T) {
	// GIF with no frames
	g := &gif.GIF{}
	var buf bytes.Buffer
	gif.EncodeAll(&buf, g)

	_, err := decodeFirstFrame(buf.Bytes(), "image/gif", "empty.gif")
	if err == nil {
		t.Error("decodeFirstFrame() should error on empty GIF")
	}
}

func TestGetStaticThumbnail(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	pngData := createTestPNG(400, 300)

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "image/png")
		w.Write(pngData)
	}))
	defer srv.Close()

	cfg := DefaultThumbnailConfig()
	path, err := GetStaticThumbnail(1, srv.URL+"/image.png", cfg)
	if err != nil {
		t.Fatalf("GetStaticThumbnail() error = %v", err)
	}

	if path == "" {
		t.Fatal("GetStaticThumbnail() returned empty path")
	}

	// File should exist
	info, err := os.Stat(path)
	if err != nil {
		t.Fatalf("thumbnail file not found: %v", err)
	}
	if info.Size() == 0 {
		t.Error("thumbnail file is empty")
	}
}

func TestGetStaticThumbnail_Cached(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	callCount := 0
	pngData := createTestPNG(200, 150)

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		callCount++
		w.Header().Set("Content-Type", "image/png")
		w.Write(pngData)
	}))
	defer srv.Close()

	cfg := DefaultThumbnailConfig()
	url := srv.URL + "/cached.png"

	// First call downloads
	path1, err := GetStaticThumbnail(1, url, cfg)
	if err != nil {
		t.Fatalf("first call error = %v", err)
	}

	// Second call should use cache (no HTTP request)
	path2, err := GetStaticThumbnail(1, url, cfg)
	if err != nil {
		t.Fatalf("second call error = %v", err)
	}

	if path1 != path2 {
		t.Errorf("cached path mismatch: %q != %q", path1, path2)
	}

	if callCount != 1 {
		t.Errorf("HTTP called %d times, want 1 (cached)", callCount)
	}
}

func TestGetStaticThumbnail_InvalidInput(t *testing.T) {
	cfg := DefaultThumbnailConfig()

	// Invalid gameID
	_, err := GetStaticThumbnail(0, "https://example.com/img.png", cfg)
	if err == nil {
		t.Error("should error on gameID 0")
	}

	// Empty URL
	_, err = GetStaticThumbnail(1, "", cfg)
	if err == nil {
		t.Error("should error on empty URL")
	}
}

func TestDefaultThumbnailConfig(t *testing.T) {
	cfg := DefaultThumbnailConfig()

	if cfg.MaxWidth != 200 {
		t.Errorf("MaxWidth = %d, want 200", cfg.MaxWidth)
	}
	if cfg.MaxHeight != 0 {
		t.Errorf("MaxHeight = %d, want 0 (proportional)", cfg.MaxHeight)
	}
	if cfg.Quality != 70 {
		t.Errorf("Quality = %d, want 70", cfg.Quality)
	}
}

func TestGetCachedThumbnailPath_NotCached(t *testing.T) {
	t.Setenv("XDG_CONFIG_HOME", t.TempDir())

	path := GetCachedThumbnailPath(1, "https://example.com/nonexistent.png")
	if path != "" {
		t.Errorf("GetCachedThumbnailPath() = %q, want empty for non-cached", path)
	}
}
