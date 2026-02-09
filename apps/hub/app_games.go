package main

import (
	"context"
	"fmt"
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
