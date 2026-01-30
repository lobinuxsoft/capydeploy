// Package artwork provides Steam artwork application for the Agent.
package artwork

import (
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
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

// Apply downloads artwork from URLs and saves it to the Steam grid folder.
// Uses Steam's CEF API for animated WebP/GIF support, with filesystem fallback.
func Apply(userID string, appID uint32, cfg *protocol.ArtworkConfig) (*ApplyResult, error) {
	if cfg == nil {
		return &ApplyResult{Applied: []string{}}, nil
	}

	result := &ApplyResult{
		Applied: []string{},
		Failed:  []ArtworkResult{},
	}

	// Convert protocol.ArtworkConfig to steam-shortcut-manager's ArtworkConfig
	artwork := &ssmSteam.ArtworkConfig{
		GridPortrait:  cfg.Grid,   // 600x900
		GridLandscape: cfg.Banner, // 920x430
		HeroImage:     cfg.Hero,   // 1920x620
		LogoImage:     cfg.Logo,
		IconImage:     cfg.Icon,
	}

	// Use steam-shortcut-manager's SetArtwork which tries CEF API first,
	// then falls back to filesystem. CEF API supports animated WebP/GIF.
	if err := ssmSteam.SetArtwork(uint64(appID), artwork); err != nil {
		// If SetArtwork fails completely, report all as failed
		if cfg.Grid != "" {
			result.Failed = append(result.Failed, ArtworkResult{Type: "grid", Error: err.Error()})
		}
		if cfg.Banner != "" {
			result.Failed = append(result.Failed, ArtworkResult{Type: "banner", Error: err.Error()})
		}
		if cfg.Hero != "" {
			result.Failed = append(result.Failed, ArtworkResult{Type: "hero", Error: err.Error()})
		}
		if cfg.Logo != "" {
			result.Failed = append(result.Failed, ArtworkResult{Type: "logo", Error: err.Error()})
		}
		if cfg.Icon != "" {
			result.Failed = append(result.Failed, ArtworkResult{Type: "icon", Error: err.Error()})
		}
		return result, nil
	}

	// SetArtwork succeeded - report all provided artwork as applied
	if cfg.Grid != "" {
		result.Applied = append(result.Applied, "grid")
	}
	if cfg.Banner != "" {
		result.Applied = append(result.Applied, "banner")
	}
	if cfg.Hero != "" {
		result.Applied = append(result.Applied, "hero")
	}
	if cfg.Logo != "" {
		result.Applied = append(result.Applied, "logo")
	}
	if cfg.Icon != "" {
		result.Applied = append(result.Applied, "icon")
	}

	return result, nil
}
