package main

import (
	"fmt"
	"os/exec"
	goruntime "runtime"

	"github.com/wailsapp/wails/v2/pkg/runtime"

	"github.com/lobinuxsoft/capydeploy/pkg/config"
	"github.com/lobinuxsoft/capydeploy/pkg/steamgriddb"
	"github.com/lobinuxsoft/capydeploy/pkg/version"
)

// GetSteamGridDBAPIKey returns the SteamGridDB API key
func (a *App) GetSteamGridDBAPIKey() (string, error) {
	return config.GetSteamGridDBAPIKey()
}

// SetSteamGridDBAPIKey saves the SteamGridDB API key
func (a *App) SetSteamGridDBAPIKey(apiKey string) error {
	return config.SetSteamGridDBAPIKey(apiKey)
}

// GetCacheSize returns the size of the image cache
func (a *App) GetCacheSize() (int64, error) {
	return steamgriddb.GetCacheSize()
}

// ClearImageCache clears the image cache
func (a *App) ClearImageCache() error {
	return steamgriddb.ClearImageCache()
}

// GetImageCacheEnabled returns whether image caching is enabled
func (a *App) GetImageCacheEnabled() (bool, error) {
	return config.GetImageCacheEnabled()
}

// SetImageCacheEnabled enables or disables image caching
// When disabled, automatically clears the cache
func (a *App) SetImageCacheEnabled(enabled bool) error {
	if err := config.SetImageCacheEnabled(enabled); err != nil {
		return err
	}
	// Clear cache when disabling
	if !enabled {
		return steamgriddb.ClearImageCache()
	}
	return nil
}

// OpenCacheFolder opens the cache folder in the file explorer
func (a *App) OpenCacheFolder() error {
	cacheDir, err := steamgriddb.GetImageCacheDir()
	if err != nil {
		return err
	}

	var cmd *exec.Cmd
	switch goruntime.GOOS {
	case "windows":
		cmd = exec.Command("explorer", cacheDir)
	case "darwin":
		cmd = exec.Command("open", cacheDir)
	default: // linux and others
		cmd = exec.Command("xdg-open", cacheDir)
	}

	return cmd.Start()
}

// GetVersion returns the current version information.
func (a *App) GetVersion() version.Info {
	return version.GetInfo()
}

// GetHubInfo returns the Hub's identity information.
func (a *App) GetHubInfo() HubInfo {
	if a.configMgr == nil {
		return HubInfo{
			ID:       "",
			Name:     "CapyDeploy Hub",
			Platform: goruntime.GOOS,
		}
	}
	return HubInfo{
		ID:       a.configMgr.GetID(),
		Name:     a.configMgr.GetName(),
		Platform: a.configMgr.GetPlatform(),
	}
}

// GetHubName returns the Hub's display name.
func (a *App) GetHubName() string {
	if a.configMgr == nil {
		return "CapyDeploy Hub"
	}
	return a.configMgr.GetName()
}

// SetHubName sets the Hub's display name.
func (a *App) SetHubName(name string) error {
	if a.configMgr == nil {
		return fmt.Errorf("config manager not initialized")
	}
	return a.configMgr.SetName(name)
}

// GetGameSetups returns all saved game setups
func (a *App) GetGameSetups() ([]config.GameSetup, error) {
	return config.GetGameSetups()
}

// AddGameSetup adds a new game setup
func (a *App) AddGameSetup(setup config.GameSetup) error {
	return config.AddGameSetup(setup)
}

// UpdateGameSetup updates an existing game setup
func (a *App) UpdateGameSetup(id string, setup config.GameSetup) error {
	return config.UpdateGameSetup(id, setup)
}

// RemoveGameSetup removes a game setup
func (a *App) RemoveGameSetup(id string) error {
	return config.RemoveGameSetup(id)
}

// SelectFolder opens a folder selection dialog
func (a *App) SelectFolder() (string, error) {
	return runtime.OpenDirectoryDialog(a.ctx, runtime.OpenDialogOptions{
		Title: "Select Game Folder",
	})
}
