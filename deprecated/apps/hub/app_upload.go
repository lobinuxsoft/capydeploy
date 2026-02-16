package main

import (
	"context"
	"fmt"
	"io"
	"log"
	"os"
	"path/filepath"
	"strings"

	"github.com/wailsapp/wails/v2/pkg/runtime"

	"github.com/lobinuxsoft/capydeploy/apps/hub/modules"
	"github.com/lobinuxsoft/capydeploy/pkg/config"
	"github.com/lobinuxsoft/capydeploy/pkg/discovery"
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/transfer"
)

// UploadGame uploads a game to the connected agent
func (a *App) UploadGame(setupID string) error {
	a.mu.RLock()
	if a.connectedAgent == nil {
		a.mu.RUnlock()
		return fmt.Errorf("no agent connected")
	}
	client := a.connectedAgent.Client
	agentInfo := a.connectedAgent.Agent
	a.mu.RUnlock()

	// Get the game setup
	setups, err := config.GetGameSetups()
	if err != nil {
		return fmt.Errorf("failed to get game setups: %w", err)
	}

	var setup *config.GameSetup
	for _, s := range setups {
		if s.ID == setupID {
			setup = &s
			break
		}
	}

	if setup == nil {
		return fmt.Errorf("game setup not found: %s", setupID)
	}

	// Start upload in goroutine
	go a.performUpload(client, agentInfo, setup)

	return nil
}

func (a *App) performUpload(client modules.PlatformClient, agentInfo *discovery.DiscoveredAgent, setup *config.GameSetup) {
	ctx, cancel := context.WithCancel(a.ctx)
	defer cancel()

	emitProgress := func(progress float64, status string, errMsg string, done bool) {
		runtime.EventsEmit(a.ctx, "upload:progress", UploadProgress{
			Progress: progress,
			Status:   status,
			Error:    errMsg,
			Done:     done,
		})
	}

	// Check if client supports uploads
	uploader, ok := modules.AsFileUploader(client)
	if !ok {
		emitProgress(0, "", "Agent does not support file uploads", true)
		return
	}

	emitProgress(0, "Scanning files...", "", false)

	// Scan local files
	files, totalSize, err := scanFilesForUpload(setup.LocalPath)
	if err != nil {
		emitProgress(0, "", fmt.Sprintf("Failed to scan files: %v", err), true)
		return
	}

	emitProgress(0.05, "Initializing upload...", "", false)

	// Prepare upload config
	uploadConfig := protocol.UploadConfig{
		GameName:      setup.Name,
		InstallPath:   setup.InstallPath,
		Executable:    setup.Executable,
		LaunchOptions: setup.LaunchOptions,
		Tags:          setup.Tags,
	}

	// Initialize upload
	initResp, err := uploader.InitUpload(ctx, uploadConfig, totalSize, files)
	if err != nil {
		emitProgress(0, "", fmt.Sprintf("Failed to initialize upload: %v", err), true)
		return
	}

	uploadID := initResp.UploadID
	chunkSize := initResp.ChunkSize
	if chunkSize == 0 {
		chunkSize = 1024 * 1024 // 1MB default
	}

	emitProgress(0.1, "Uploading files...", "", false)

	// Upload files in chunks
	var uploaded int64
	for _, fileEntry := range files {
		localPath := filepath.Join(setup.LocalPath, fileEntry.RelativePath)

		file, err := os.Open(localPath)
		if err != nil {
			emitProgress(0, "", fmt.Sprintf("Failed to open %s: %v", fileEntry.RelativePath, err), true)
			uploader.CancelUpload(ctx, uploadID)
			return
		}

		var offset int64
		// Check for resume point
		if resumeOffset, hasResume := initResp.ResumeFrom[fileEntry.RelativePath]; hasResume {
			offset = resumeOffset
			file.Seek(offset, 0)
			uploaded += offset
		}

		buf := make([]byte, chunkSize)
		for {
			n, readErr := file.Read(buf)
			if n > 0 {
				chunk := &transfer.Chunk{
					FilePath: fileEntry.RelativePath,
					Offset:   offset,
					Size:     n,
					Data:     buf[:n],
				}

				if err := uploader.UploadChunk(ctx, uploadID, chunk); err != nil {
					file.Close()
					emitProgress(0, "", fmt.Sprintf("Failed to upload chunk: %v", err), true)
					uploader.CancelUpload(ctx, uploadID)
					return
				}

				offset += int64(n)
				uploaded += int64(n)

				// Update progress (10% to 85% for file transfer)
				progress := 0.1 + (float64(uploaded)/float64(totalSize))*0.75
				emitProgress(progress, fmt.Sprintf("Uploading: %s", fileEntry.RelativePath), "", false)
			}

			if readErr == io.EOF {
				break
			}
			if readErr != nil {
				file.Close()
				emitProgress(0, "", fmt.Sprintf("Failed to read %s: %v", fileEntry.RelativePath, readErr), true)
				uploader.CancelUpload(ctx, uploadID)
				return
			}
		}
		file.Close()
	}

	// Send local artwork as binary WS messages BEFORE CompleteUpload.
	// Agents that don't know the AppID yet (Decky) store it as pending
	// and include it in the shortcut creation flow.
	a.sendLocalArtwork(ctx, setup, 0, emitProgress)

	emitProgress(0.85, "Creating shortcut...", "", false)

	// Prepare shortcut config — only include remote (http) artwork URLs.
	// Local (file://) artwork was already sent as binary above.
	artworkCfg := buildRemoteArtworkConfig(setup)

	// Only send the executable filename — the agent knows its own install path
	shortcutCfg := &protocol.ShortcutConfig{
		Name:          setup.Name,
		Exe:           setup.Executable,
		LaunchOptions: setup.LaunchOptions,
		Tags:          parseTags(setup.Tags),
		Artwork:       artworkCfg,
	}

	// Complete upload with shortcut creation
	completeResp, err := uploader.CompleteUpload(ctx, uploadID, true, shortcutCfg)
	if err != nil {
		emitProgress(0, "", fmt.Sprintf("Failed to complete upload: %v", err), true)
		return
	}

	if !completeResp.Success {
		emitProgress(0, "", "Upload completion failed", true)
		return
	}

	emitProgress(1.0, "Upload complete!", "", true)
}

// sendLocalArtwork sends local artwork images to the agent via binary WS messages.
func (a *App) sendLocalArtwork(ctx context.Context, setup *config.GameSetup, appID uint32, emitProgress func(float64, string, string, bool)) {
	a.mu.RLock()
	wsClient := a.connectedAgent.WSClient
	a.mu.RUnlock()

	if wsClient == nil {
		return
	}

	artworkFields := map[string]string{
		"grid":   setup.GridPortrait,
		"banner": setup.GridLandscape,
		"hero":   setup.HeroImage,
		"logo":   setup.LogoImage,
		"icon":   setup.IconImage,
	}

	for artType, path := range artworkFields {
		if !strings.HasPrefix(path, "file://") {
			continue
		}
		localPath := strings.TrimPrefix(path, "file://")

		data, err := os.ReadFile(localPath)
		if err != nil {
			log.Printf("Hub: Failed to read local artwork %s: %v", localPath, err)
			continue
		}

		contentType := detectContentType(localPath)
		if contentType == "" {
			log.Printf("Hub: Unknown content type for artwork: %s", localPath)
			continue
		}

		emitProgress(0.9, fmt.Sprintf("Sending %s artwork...", artType), "", false)

		if err := wsClient.SendArtworkImage(ctx, appID, artType, contentType, data); err != nil {
			log.Printf("Hub: Failed to send artwork %s: %v", artType, err)
		} else {
			log.Printf("Hub: Sent local artwork %s for AppID %d", artType, appID)
		}
	}
}

// scanFilesForUpload scans a directory and returns file entries for upload
func scanFilesForUpload(rootPath string) ([]transfer.FileEntry, int64, error) {
	var files []transfer.FileEntry
	var totalSize int64

	err := filepath.Walk(rootPath, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if info.IsDir() {
			return nil
		}

		relPath, err := filepath.Rel(rootPath, path)
		if err != nil {
			return err
		}
		// Normalize path separators
		relPath = strings.ReplaceAll(relPath, "\\", "/")

		files = append(files, transfer.FileEntry{
			RelativePath: relPath,
			Size:         info.Size(),
		})
		totalSize += info.Size()

		return nil
	})

	return files, totalSize, err
}

// buildRemoteArtworkConfig returns an ArtworkConfig with only remote (http) URLs.
// Local file:// paths are excluded — they are sent as binary WS messages.
func buildRemoteArtworkConfig(setup *config.GameSetup) *protocol.ArtworkConfig {
	cfg := &protocol.ArtworkConfig{}
	hasAny := false

	if strings.HasPrefix(setup.GridPortrait, "http") {
		cfg.Grid = setup.GridPortrait
		hasAny = true
	}
	if strings.HasPrefix(setup.GridLandscape, "http") {
		cfg.Banner = setup.GridLandscape
		hasAny = true
	}
	if strings.HasPrefix(setup.HeroImage, "http") {
		cfg.Hero = setup.HeroImage
		hasAny = true
	}
	if strings.HasPrefix(setup.LogoImage, "http") {
		cfg.Logo = setup.LogoImage
		hasAny = true
	}
	if strings.HasPrefix(setup.IconImage, "http") {
		cfg.Icon = setup.IconImage
		hasAny = true
	}

	if !hasAny {
		return nil
	}
	return cfg
}

// parseTags parses a comma-separated tag string into a slice
func parseTags(tagsStr string) []string {
	if tagsStr == "" {
		return nil
	}
	tags := strings.Split(tagsStr, ",")
	result := make([]string, 0, len(tags))
	for _, tag := range tags {
		tag = strings.TrimSpace(tag)
		if tag != "" {
			result = append(result, tag)
		}
	}
	return result
}
