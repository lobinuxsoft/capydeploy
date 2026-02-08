// Package artwork provides Steam artwork application for the Agent.
package artwork

import (
	"context"
	"encoding/base64"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"time"

	agentSteam "github.com/lobinuxsoft/capydeploy/apps/agents/desktop/steam"
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

// ApplyFromData applies raw image bytes as Steam artwork.
// Tries CEF API first (instant, no restart needed), falls back to filesystem.
// artworkType: "grid", "banner", "hero", "logo", "icon".
// contentType: "image/png", "image/jpeg", "image/webp".
func ApplyFromData(appID uint32, artworkType string, data []byte, contentType string) error {
	ext := extFromContentType(contentType)
	if ext == "" {
		return fmt.Errorf("unsupported content type: %s", contentType)
	}

	if _, ok := artworkSuffix(artworkType); !ok {
		return fmt.Errorf("unknown artwork type: %s", artworkType)
	}

	// Try CEF API first — applies instantly without Steam restart
	if err := applyViaCEF(appID, artworkType, data); err != nil {
		log.Printf("[artwork] CEF failed for %s (appID %d), falling back to filesystem: %v", artworkType, appID, err)
	} else {
		return nil
	}

	// Fallback: write to filesystem (requires Steam restart to take effect)
	return applyViaFilesystem(appID, artworkType, data, ext)
}

// applyViaCEF applies artwork using Steam's CEF API (SetCustomArtworkForApp).
// Ensures CEF is ready (creates debug file + restarts Steam if needed) before operating.
func applyViaCEF(appID uint32, artworkType string, data []byte) error {
	assetType, ok := agentSteam.ArtworkTypeToCEFAsset(artworkType)
	if !ok {
		return fmt.Errorf("no CEF asset mapping for type: %s", artworkType)
	}

	// Ensure CEF debugger is available (creates file + restarts Steam if needed)
	if err := agentSteam.EnsureCEFReady(); err != nil {
		return fmt.Errorf("CEF not ready: %w", err)
	}

	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()

	base64Data := base64.StdEncoding.EncodeToString(data)
	client := agentSteam.NewCEFClient()

	// Clear before set (same pattern as Decky agent).
	// Both Clear and Set await their Promises via evaluateAsync,
	// so no artificial delay is needed between them.
	if err := client.ClearCustomArtwork(ctx, appID, assetType); err != nil {
		return fmt.Errorf("failed to clear artwork: %w", err)
	}

	if err := client.SetCustomArtwork(ctx, appID, base64Data, assetType); err != nil {
		return fmt.Errorf("failed to set artwork: %w", err)
	}

	return nil
}

// applyViaFilesystem writes artwork to Steam's grid directory.
func applyViaFilesystem(appID uint32, artworkType string, data []byte, ext string) error {
	suffix, _ := artworkSuffix(artworkType)

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
