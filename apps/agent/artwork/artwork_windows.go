//go:build windows

// Package artwork provides Steam artwork application for the Agent.
package artwork

import (
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	ssmSteam "github.com/shadowblip/steam-shortcut-manager/pkg/steam"
)

// Apply downloads artwork from URLs and saves it to the Steam grid folder.
// On Windows, uses filesystem method only (no CEF API support).
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

	// Use filesystem method directly (no CEF on Windows)
	if err := ssmSteam.SetArtworkFilesystem(uint64(appID), artwork); err != nil {
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

	// Report all provided artwork as applied
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
