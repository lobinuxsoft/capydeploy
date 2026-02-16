package auth

import (
	"time"

	"github.com/lobinuxsoft/capydeploy/apps/agents/desktop/config"
)

// ConfigStorageAdapter adapts config.Manager to implement auth.Storage.
type ConfigStorageAdapter struct {
	cfg *config.Manager
}

// NewConfigStorage creates a new storage adapter from a config manager.
func NewConfigStorage(cfg *config.Manager) *ConfigStorageAdapter {
	return &ConfigStorageAdapter{cfg: cfg}
}

// GetAuthorizedHubs returns the list of authorized Hubs.
func (s *ConfigStorageAdapter) GetAuthorizedHubs() []AuthorizedHub {
	cfgHubs := s.cfg.GetAuthorizedHubs()
	hubs := make([]AuthorizedHub, len(cfgHubs))
	for i, h := range cfgHubs {
		hubs[i] = AuthorizedHub{
			ID:       h.ID,
			Name:     h.Name,
			Platform: h.Platform,
			Token:    h.Token,
			PairedAt: h.PairedAt,
			LastSeen: h.LastSeen,
		}
	}
	return hubs
}

// AddAuthorizedHub adds a Hub to the authorized list.
func (s *ConfigStorageAdapter) AddAuthorizedHub(hub AuthorizedHub) error {
	cfgHub := config.AuthorizedHub{
		ID:       hub.ID,
		Name:     hub.Name,
		Platform: hub.Platform,
		Token:    hub.Token,
		PairedAt: hub.PairedAt,
		LastSeen: hub.LastSeen,
	}
	return s.cfg.AddAuthorizedHub(cfgHub)
}

// RemoveAuthorizedHub removes a Hub from the authorized list.
func (s *ConfigStorageAdapter) RemoveAuthorizedHub(hubID string) error {
	return s.cfg.RemoveAuthorizedHub(hubID)
}

// UpdateHubLastSeen updates the LastSeen timestamp for a Hub.
func (s *ConfigStorageAdapter) UpdateHubLastSeen(hubID string, lastSeen time.Time) error {
	return s.cfg.UpdateHubLastSeen(hubID, lastSeen)
}

// Save persists the storage (no-op, config saves on each change).
func (s *ConfigStorageAdapter) Save() error {
	return nil
}
