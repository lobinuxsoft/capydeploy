// Package config provides persistent configuration for the Hub.
package config

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"os"
	"path/filepath"
	"runtime"
	"sync"
	"time"

	"github.com/lobinuxsoft/capydeploy/pkg/discovery"
)

// Config holds the hub configuration.
type Config struct {
	ID       string `json:"id"`
	Name     string `json:"name"`
	Platform string `json:"platform"`
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

	dir := filepath.Join(configDir, "capydeploy-hub")
	if err := os.MkdirAll(dir, 0755); err != nil {
		return nil, err
	}

	hostname := discovery.GetHostname()
	platform := detectPlatform()

	m := &Manager{
		filePath: filepath.Join(dir, "config.json"),
		config: Config{
			ID:       generateID(hostname, platform),
			Name:     hostname,
			Platform: platform,
		},
	}

	// Load existing config if present
	m.load()

	return m, nil
}

// generateID creates a stable ID based on hostname and platform.
func generateID(hostname, platform string) string {
	data := hostname + "-" + platform + "-hub"
	hash := sha256.Sum256([]byte(data))
	return hex.EncodeToString(hash[:])[:8]
}

// detectPlatform returns the current platform.
func detectPlatform() string {
	return runtime.GOOS
}

// load reads config from disk.
func (m *Manager) load() {
	data, err := os.ReadFile(m.filePath)
	if err != nil {
		// First run - save defaults
		m.Save()
		return
	}

	var cfg Config
	if err := json.Unmarshal(data, &cfg); err != nil {
		return
	}

	// Preserve ID if it exists (persistent identity)
	if cfg.ID != "" {
		m.config.ID = cfg.ID
	}
	if cfg.Name != "" {
		m.config.Name = cfg.Name
	}
	// Platform is always detected, not loaded
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

// GetID returns the hub ID.
func (m *Manager) GetID() string {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.config.ID
}

// GetName returns the hub name.
func (m *Manager) GetName() string {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.config.Name
}

// SetName sets the hub name and saves config.
func (m *Manager) SetName(name string) error {
	m.mu.Lock()
	m.config.Name = name
	m.mu.Unlock()

	return m.Save()
}

// GetPlatform returns the hub platform.
func (m *Manager) GetPlatform() string {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.config.Platform
}

// GetConfig returns a copy of the current config.
func (m *Manager) GetConfig() Config {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.config
}

// GenerateNewID generates a new unique ID (for troubleshooting).
func (m *Manager) GenerateNewID() error {
	m.mu.Lock()
	data := m.config.Name + "-" + m.config.Platform + "-" + time.Now().String()
	hash := sha256.Sum256([]byte(data))
	m.config.ID = hex.EncodeToString(hash[:])[:8]
	m.mu.Unlock()

	return m.Save()
}
