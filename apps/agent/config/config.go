// Package config provides persistent configuration for the Agent.
package config

import (
	"encoding/json"
	"os"
	"path/filepath"
	"sync"

	"github.com/lobinuxsoft/capydeploy/pkg/discovery"
)

// AuthorizedHub represents a Hub that has been paired with this Agent.
type AuthorizedHub struct {
	ID       string `json:"id"`
	Name     string `json:"name"`
	Token    string `json:"token"`
	PairedAt string `json:"pairedAt"`
	LastSeen string `json:"lastSeen"`
}

// Config holds the agent configuration.
type Config struct {
	Name           string          `json:"name"`
	InstallPath    string          `json:"installPath"`
	AuthorizedHubs []AuthorizedHub `json:"authorizedHubs,omitempty"`
}

// Manager handles loading and saving configuration.
type Manager struct {
	mu       sync.RWMutex
	config   Config
	filePath string
}

// NewManager creates a new configuration manager.
func NewManager() (*Manager, error) {
	configDir, err := os.UserConfigDir()
	if err != nil {
		return nil, err
	}

	dir := filepath.Join(configDir, "capydeploy-agent")
	if err := os.MkdirAll(dir, 0755); err != nil {
		return nil, err
	}

	m := &Manager{
		filePath: filepath.Join(dir, "config.json"),
		config: Config{
			Name:        discovery.GetHostname(), // Default to hostname
			InstallPath: "~/Games",               // Default install path
		},
	}

	// Load existing config if present
	m.load()

	return m, nil
}

// load reads config from disk.
func (m *Manager) load() {
	data, err := os.ReadFile(m.filePath)
	if err != nil {
		return // Use defaults
	}

	var cfg Config
	if err := json.Unmarshal(data, &cfg); err != nil {
		return // Use defaults
	}

	// Only use loaded values if they're not empty
	if cfg.Name != "" {
		m.config.Name = cfg.Name
	}
	if cfg.InstallPath != "" {
		m.config.InstallPath = cfg.InstallPath
	}
	if len(cfg.AuthorizedHubs) > 0 {
		m.config.AuthorizedHubs = cfg.AuthorizedHubs
	}
}

// Save writes config to disk.
func (m *Manager) Save() error {
	m.mu.RLock()
	defer m.mu.RUnlock()

	data, err := json.MarshalIndent(m.config, "", "  ")
	if err != nil {
		return err
	}

	return os.WriteFile(m.filePath, data, 0600)
}

// GetName returns the agent name.
func (m *Manager) GetName() string {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.config.Name
}

// SetName sets the agent name and saves config.
func (m *Manager) SetName(name string) error {
	m.mu.Lock()
	m.config.Name = name
	m.mu.Unlock()

	return m.Save()
}

// GetConfig returns a copy of the current config.
func (m *Manager) GetConfig() Config {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.config
}

// GetInstallPath returns the install path.
func (m *Manager) GetInstallPath() string {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.config.InstallPath
}

// SetInstallPath sets the install path and saves config.
func (m *Manager) SetInstallPath(path string) error {
	m.mu.Lock()
	m.config.InstallPath = path
	m.mu.Unlock()

	return m.Save()
}

// GetAuthorizedHubs returns the list of authorized Hubs.
func (m *Manager) GetAuthorizedHubs() []AuthorizedHub {
	m.mu.RLock()
	defer m.mu.RUnlock()
	// Return a copy to prevent external modification
	hubs := make([]AuthorizedHub, len(m.config.AuthorizedHubs))
	copy(hubs, m.config.AuthorizedHubs)
	return hubs
}

// AddAuthorizedHub adds a Hub to the authorized list.
func (m *Manager) AddAuthorizedHub(hub AuthorizedHub) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	// Check if already exists and update
	for i, h := range m.config.AuthorizedHubs {
		if h.ID == hub.ID {
			m.config.AuthorizedHubs[i] = hub
			return m.saveUnlocked()
		}
	}

	// Add new hub
	m.config.AuthorizedHubs = append(m.config.AuthorizedHubs, hub)
	return m.saveUnlocked()
}

// RemoveAuthorizedHub removes a Hub from the authorized list.
func (m *Manager) RemoveAuthorizedHub(hubID string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	for i, h := range m.config.AuthorizedHubs {
		if h.ID == hubID {
			m.config.AuthorizedHubs = append(m.config.AuthorizedHubs[:i], m.config.AuthorizedHubs[i+1:]...)
			return m.saveUnlocked()
		}
	}
	return nil // Not found is not an error
}

// UpdateHubLastSeen updates the LastSeen timestamp for a Hub.
func (m *Manager) UpdateHubLastSeen(hubID string, lastSeen string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	for i, h := range m.config.AuthorizedHubs {
		if h.ID == hubID {
			m.config.AuthorizedHubs[i].LastSeen = lastSeen
			return m.saveUnlocked()
		}
	}
	return nil
}

// saveUnlocked writes config to disk (must hold lock).
func (m *Manager) saveUnlocked() error {
	data, err := json.MarshalIndent(m.config, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(m.filePath, data, 0600)
}
