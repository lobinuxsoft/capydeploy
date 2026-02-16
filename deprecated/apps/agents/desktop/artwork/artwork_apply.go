// Package artwork provides Steam artwork application for the Agent.
package artwork

import (
	"log"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

// Apply downloads artwork from URLs and applies each via ApplyFromData
// (CEF API first, filesystem fallback).
func Apply(userID string, appID uint32, cfg *protocol.ArtworkConfig) (*ApplyResult, error) {
	if cfg == nil {
		return &ApplyResult{Applied: []string{}}, nil
	}

	result := &ApplyResult{
		Applied: []string{},
		Failed:  []ArtworkResult{},
	}

	// Map of artwork type → URL from config
	artworks := map[string]string{
		"grid":   cfg.Grid,
		"banner": cfg.Banner,
		"hero":   cfg.Hero,
		"logo":   cfg.Logo,
		"icon":   cfg.Icon,
	}

	for artType, url := range artworks {
		if url == "" {
			continue
		}

		data, contentType, err := downloadURL(url)
		if err != nil {
			log.Printf("[artwork] failed to download %s: %v", artType, err)
			result.Failed = append(result.Failed, ArtworkResult{Type: artType, Error: err.Error()})
			continue
		}

		if err := ApplyFromData(appID, artType, data, contentType); err != nil {
			log.Printf("[artwork] failed to apply %s: %v", artType, err)
			result.Failed = append(result.Failed, ArtworkResult{Type: artType, Error: err.Error()})
			continue
		}

		result.Applied = append(result.Applied, artType)
	}

	return result, nil
}
