package auth

import (
	"time"

	"github.com/lobinuxsoft/capydeploy/apps/agent/config"
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
			PairedAt: parseTime(h.PairedAt),
			LastSeen: parseTime(h.LastSeen),
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
		PairedAt: hub.PairedAt.Format(time.RFC3339),
		LastSeen: hub.LastSeen.Format(time.RFC3339),
	}
	return s.cfg.AddAuthorizedHub(cfgHub)
}

// RemoveAuthorizedHub removes a Hub from the authorized list.
func (s *ConfigStorageAdapter) RemoveAuthorizedHub(hubID string) error {
	return s.cfg.RemoveAuthorizedHub(hubID)
}

// UpdateHubLastSeen updates the LastSeen timestamp for a Hub.
func (s *ConfigStorageAdapter) UpdateHubLastSeen(hubID string, lastSeen time.Time) error {
	return s.cfg.UpdateHubLastSeen(hubID, lastSeen.Format(time.RFC3339))
}

// Save persists the storage (no-op, config saves on each change).
func (s *ConfigStorageAdapter) Save() error {
	return nil
}

func parseTime(s string) time.Time {
	t, _ := time.Parse(time.RFC3339, s)
	return t
}
