package config

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"
)

// DeviceConfig represents a saved device configuration
type DeviceConfig struct {
	Name     string `json:"name"`
	Host     string `json:"host"`
	Port     int    `json:"port"`
	User     string `json:"user"`
	KeyFile  string `json:"key_file,omitempty"`
	Password string `json:"password,omitempty"`
}

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
	Devices            []DeviceConfig `json:"devices"`
	GameSetups         []GameSetup    `json:"game_setups"`
	DefaultInstallPath string         `json:"default_install_path"`
	SteamGridDBAPIKey  string         `json:"steamgriddb_api_key,omitempty"`
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
			return &AppConfig{
				Devices:            []DeviceConfig{},
				DefaultInstallPath: "~/Games",
			}, nil
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

// AddDevice adds a device to the config and saves it
func AddDevice(device DeviceConfig) error {
	config, err := Load()
	if err != nil {
		return err
	}

	// Check if device already exists (by host)
	for i, d := range config.Devices {
		if d.Host == device.Host {
			// Update existing device
			config.Devices[i] = device
			return Save(config)
		}
	}

	// Add new device
	config.Devices = append(config.Devices, device)
	return Save(config)
}

// RemoveDevice removes a device from the config
func RemoveDevice(host string) error {
	config, err := Load()
	if err != nil {
		return err
	}

	for i, d := range config.Devices {
		if d.Host == host {
			config.Devices = append(config.Devices[:i], config.Devices[i+1:]...)
			break
		}
	}

	return Save(config)
}

// GetDevices returns all saved devices
func GetDevices() ([]DeviceConfig, error) {
	config, err := Load()
	if err != nil {
		return nil, err
	}
	return config.Devices, nil
}

// UpdateDevice updates an existing device
func UpdateDevice(oldHost string, device DeviceConfig) error {
	config, err := Load()
	if err != nil {
		return err
	}

	for i, d := range config.Devices {
		if d.Host == oldHost {
			config.Devices[i] = device
			return Save(config)
		}
	}

	// If not found, add it
	config.Devices = append(config.Devices, device)
	return Save(config)
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
