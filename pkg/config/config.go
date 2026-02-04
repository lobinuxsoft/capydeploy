package config

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"
)

// GameSetup represents a saved game installation setup
type GameSetup struct {
	ID            string `json:"id"`
	Name          string `json:"name"`
	LocalPath     string `json:"local_path"`
	Executable    string `json:"executable"`
	LaunchOptions string `json:"launch_options,omitempty"`
	Tags          string `json:"tags,omitempty"`
	InstallPath   string `json:"install_path"`
	// SteamGridDB artwork
	GridDBGameID   int    `json:"griddb_game_id,omitempty"`
	GridPortrait   string `json:"grid_portrait,omitempty"`   // 600x900 portrait grid
	GridLandscape  string `json:"grid_landscape,omitempty"`  // 920x430 landscape grid
	HeroImage      string `json:"hero_image,omitempty"`      // 1920x620 hero banner
	LogoImage      string `json:"logo_image,omitempty"`      // Logo with transparency
	IconImage      string `json:"icon_image,omitempty"`      // Square icon
}

// AppConfig represents the application configuration
type AppConfig struct {
	GameSetups        []GameSetup `json:"game_setups"`
	SteamGridDBAPIKey string      `json:"steamgriddb_api_key,omitempty"`
	ImageCacheEnabled bool        `json:"image_cache_enabled"`
}

// GetConfigPath returns the path to the config file
func GetConfigPath() (string, error) {
	configDir, err := os.UserConfigDir()
	if err != nil {
		// Fallback to home directory
		home, err := os.UserHomeDir()
		if err != nil {
			return "", err
		}
		configDir = home
	}

	appConfigDir := filepath.Join(configDir, "capydeploy")
	if err := os.MkdirAll(appConfigDir, 0755); err != nil {
		return "", err
	}

	return filepath.Join(appConfigDir, "config.json"), nil
}

// Load loads the configuration from disk
func Load() (*AppConfig, error) {
	configPath, err := GetConfigPath()
	if err != nil {
		return nil, err
	}

	data, err := os.ReadFile(configPath)
	if err != nil {
		if os.IsNotExist(err) {
			// Return default config if file doesn't exist
			return &AppConfig{ImageCacheEnabled: true}, nil
		}
		return nil, err
	}

	var config AppConfig
	if err := json.Unmarshal(data, &config); err != nil {
		return nil, err
	}

	return &config, nil
}

// Save saves the configuration to disk
func Save(config *AppConfig) error {
	configPath, err := GetConfigPath()
	if err != nil {
		return err
	}

	data, err := json.MarshalIndent(config, "", "  ")
	if err != nil {
		return err
	}

	return os.WriteFile(configPath, data, 0600)
}

// AddGameSetup adds a game setup to the config
func AddGameSetup(setup GameSetup) error {
	config, err := Load()
	if err != nil {
		return err
	}

	// Generate ID if not set
	if setup.ID == "" {
		setup.ID = fmt.Sprintf("game_%d", time.Now().UnixNano())
	}

	// Check if setup already exists (by ID)
	for i, s := range config.GameSetups {
		if s.ID == setup.ID {
			config.GameSetups[i] = setup
			return Save(config)
		}
	}

	config.GameSetups = append(config.GameSetups, setup)
	return Save(config)
}

// UpdateGameSetup updates an existing game setup
func UpdateGameSetup(id string, setup GameSetup) error {
	config, err := Load()
	if err != nil {
		return err
	}

	for i, s := range config.GameSetups {
		if s.ID == id {
			setup.ID = id // Keep the same ID
			config.GameSetups[i] = setup
			return Save(config)
		}
	}

	return fmt.Errorf("game setup not found: %s", id)
}

// RemoveGameSetup removes a game setup from the config
func RemoveGameSetup(id string) error {
	config, err := Load()
	if err != nil {
		return err
	}

	for i, s := range config.GameSetups {
		if s.ID == id {
			config.GameSetups = append(config.GameSetups[:i], config.GameSetups[i+1:]...)
			return Save(config)
		}
	}

	return nil
}

// GetGameSetups returns all saved game setups
func GetGameSetups() ([]GameSetup, error) {
	config, err := Load()
	if err != nil {
		return nil, err
	}
	return config.GameSetups, nil
}

// GetSteamGridDBAPIKey returns the SteamGridDB API key
func GetSteamGridDBAPIKey() (string, error) {
	config, err := Load()
	if err != nil {
		return "", err
	}
	return config.SteamGridDBAPIKey, nil
}

// SetSteamGridDBAPIKey saves the SteamGridDB API key
func SetSteamGridDBAPIKey(apiKey string) error {
	config, err := Load()
	if err != nil {
		return err
	}
	config.SteamGridDBAPIKey = apiKey
	return Save(config)
}

// GetImageCacheEnabled returns whether image caching is enabled
func GetImageCacheEnabled() (bool, error) {
	config, err := Load()
	if err != nil {
		return true, err // Default to enabled on error
	}
	return config.ImageCacheEnabled, nil
}

// SetImageCacheEnabled enables or disables image caching
func SetImageCacheEnabled(enabled bool) error {
	config, err := Load()
	if err != nil {
		return err
	}
	config.ImageCacheEnabled = enabled
	return Save(config)
}
