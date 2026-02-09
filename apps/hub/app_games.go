package main

import (
	"context"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"strings"
	"time"

	"github.com/lobinuxsoft/capydeploy/apps/hub/modules"
)

// GetInstalledGames returns shortcuts from the connected agent
func (a *App) GetInstalledGames(remotePath string) ([]InstalledGame, error) {
	a.mu.RLock()
	if a.connectedAgent == nil {
		a.mu.RUnlock()
		return nil, fmt.Errorf("no agent connected")
	}
	client := a.connectedAgent.Client
	a.mu.RUnlock()

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	// Get Steam users first
	userProvider, ok := modules.AsSteamUserProvider(client)
	if !ok {
		return nil, fmt.Errorf("agent does not support Steam user listing")
	}

	users, err := userProvider.GetSteamUsers(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get Steam users: %w", err)
	}

	if len(users) == 0 {
		return []InstalledGame{}, nil
	}

	// Get shortcuts for first user
	shortcutMgr, ok := modules.AsShortcutManager(client)
	if !ok {
		return nil, fmt.Errorf("agent does not support shortcuts")
	}

	shortcuts, err := shortcutMgr.ListShortcuts(ctx, users[0].ID)
	if err != nil {
		return nil, fmt.Errorf("failed to list shortcuts: %w", err)
	}

	games := make([]InstalledGame, 0, len(shortcuts))
	for _, sc := range shortcuts {
		games = append(games, InstalledGame{
			Name:  sc.Name,
			Path:  sc.StartDir,
			Size:  "N/A", // Agent doesn't provide size info
			AppID: sc.AppID,
		})
	}

	return games, nil
}

// DeleteGame deletes a game from the connected agent.
// The Agent handles everything internally (user detection, file deletion, Steam restart).
func (a *App) DeleteGame(name string, appID uint32) error {
	a.mu.RLock()
	if a.connectedAgent == nil {
		a.mu.RUnlock()
		return fmt.Errorf("no agent connected")
	}
	client := a.connectedAgent.Client
	a.mu.RUnlock()

	// Use longer timeout - Agent needs time for Steam restart
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Minute)
	defer cancel()

	// Use the unified GameManager endpoint - Agent handles everything
	gameMgr, ok := modules.AsGameManager(client)
	if !ok {
		return fmt.Errorf("agent does not support game management")
	}

	if _, err := gameMgr.DeleteGame(ctx, appID); err != nil {
		return fmt.Errorf("failed to delete game: %w", err)
	}

	return nil
}

// UpdateGameArtwork updates artwork for an existing installed game.
// The Hub downloads all images (local or remote) and sends them as binary via WS.
// This way the agent doesn't need internet access. Empty strings are ignored.
func (a *App) UpdateGameArtwork(appID uint32, gridPortrait, gridLandscape, heroImage, logoImage, iconImage string) error {
	a.mu.RLock()
	if a.connectedAgent == nil {
		a.mu.RUnlock()
		return fmt.Errorf("no agent connected")
	}
	wsClient := a.connectedAgent.WSClient
	a.mu.RUnlock()

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	artworkFields := map[string]string{
		"grid":   gridPortrait,
		"banner": gridLandscape,
		"hero":   heroImage,
		"logo":   logoImage,
		"icon":   iconImage,
	}

	for artType, src := range artworkFields {
		if src == "" {
			continue
		}

		var data []byte
		var contentType string
		var err error

		switch {
		case strings.HasPrefix(src, "file://"):
			data, err = os.ReadFile(strings.TrimPrefix(src, "file://"))
			if err != nil {
				log.Printf("Hub: Failed to read local artwork %s: %v", src, err)
				continue
			}
			contentType = detectContentType(strings.TrimPrefix(src, "file://"))

		case strings.HasPrefix(src, "http"):
			data, contentType, err = downloadImage(ctx, src)
			if err != nil {
				log.Printf("Hub: Failed to download artwork %s: %v", src, err)
				continue
			}

		default:
			log.Printf("Hub: Unknown artwork source scheme: %s", src)
			continue
		}

		if contentType == "" {
			log.Printf("Hub: Unknown content type for artwork: %s", src)
			continue
		}

		if err := wsClient.SendArtworkImage(ctx, appID, artType, contentType, data); err != nil {
			log.Printf("Hub: Failed to send artwork %s: %v", artType, err)
		} else {
			log.Printf("Hub: Sent %s artwork for AppID %d", artType, appID)
		}
	}

	return nil
}

// downloadImage downloads an image from a URL and returns its data and content type.
func downloadImage(ctx context.Context, url string) ([]byte, string, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, "", fmt.Errorf("failed to create request: %w", err)
	}

	resp, err := http.DefaultClient.Do(req)
	if err != nil {
		return nil, "", fmt.Errorf("failed to download %s: %w", url, err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, "", fmt.Errorf("download %s returned status %d", url, resp.StatusCode)
	}

	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, "", fmt.Errorf("failed to read response from %s: %w", url, err)
	}

	return data, resp.Header.Get("Content-Type"), nil
}
