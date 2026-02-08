package main

import (
	"context"
	"encoding/base64"
	"errors"
	"fmt"
	"io"
	"log"
	"mime"
	"os"
	"os/exec"
	"path/filepath"
	goruntime "runtime"
	"strings"
	"sync"
	"time"

	"github.com/wailsapp/wails/v2/pkg/runtime"

	"github.com/lobinuxsoft/capydeploy/apps/hub/auth"
	hubconfig "github.com/lobinuxsoft/capydeploy/apps/hub/config"
	"github.com/lobinuxsoft/capydeploy/apps/hub/modules"
	"github.com/lobinuxsoft/capydeploy/apps/hub/wsclient"
	"github.com/lobinuxsoft/capydeploy/pkg/config"
	"github.com/lobinuxsoft/capydeploy/pkg/discovery"
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/steamgriddb"
	"github.com/lobinuxsoft/capydeploy/pkg/transfer"
	"github.com/lobinuxsoft/capydeploy/pkg/version"
)

// App struct holds the application state
type App struct {
	ctx              context.Context
	connectedAgent   *ConnectedAgent
	discoveryClient  *discovery.Client
	discoveryCancel  context.CancelFunc
	discoveredMu     sync.RWMutex
	discoveredCache  map[string]*discovery.DiscoveredAgent
	mu               sync.RWMutex
	tokenStore       *auth.TokenStore
	configMgr        *hubconfig.Manager
}

// ConnectedAgent represents a connected agent with its client
type ConnectedAgent struct {
	Agent    *discovery.DiscoveredAgent
	Client   modules.PlatformClient
	WSClient *modules.WSClient       // WebSocket client (nil if using HTTP)
	Info     *protocol.AgentInfo     // Full agent info from WS connection (includes capabilities)
}

// ConnectionStatus represents the current connection status
type ConnectionStatus struct {
	Connected             bool       `json:"connected"`
	AgentID               string     `json:"agentId"`
	AgentName             string     `json:"agentName"`
	Platform              string     `json:"platform"`
	Host                  string     `json:"host"`
	Port                  int        `json:"port"`
	IPs                   []string   `json:"ips"`
	SupportedImageFormats []string   `json:"supportedImageFormats"`
	Capabilities          []string   `json:"capabilities"`
}

// DiscoveredAgentInfo represents agent info for the frontend
type DiscoveredAgentInfo struct {
	ID           string   `json:"id"`
	Name         string   `json:"name"`
	Platform     string   `json:"platform"`
	Version      string   `json:"version"`
	Host         string   `json:"host"`
	Port         int      `json:"port"`
	IPs          []string `json:"ips"`
	DiscoveredAt string   `json:"discoveredAt"`
	LastSeen     string   `json:"lastSeen"`
	Online       bool     `json:"online"`
}

// InstalledGame represents a game installed on the remote device
type InstalledGame struct {
	Name   string `json:"name"`
	Path   string `json:"path"`
	Size   string `json:"size"`
	AppID  uint32 `json:"appId,omitempty"`
}

// UploadProgress represents upload progress data
type UploadProgress struct {
	Progress float64 `json:"progress"`
	Status   string  `json:"status"`
	Error    string  `json:"error,omitempty"`
	Done     bool    `json:"done"`
}

// NewApp creates a new App application struct
func NewApp() *App {
	tokenStore, err := auth.NewTokenStore()
	if err != nil {
		log.Printf("Warning: failed to initialize token store: %v", err)
	}

	configMgr, err := hubconfig.NewManager()
	if err != nil {
		log.Printf("Warning: failed to initialize config manager: %v", err)
	}

	return &App{
		discoveryClient: discovery.NewClient(),
		discoveredCache: make(map[string]*discovery.DiscoveredAgent),
		tokenStore:      tokenStore,
		configMgr:       configMgr,
	}
}

// startup is called when the app starts
func (a *App) startup(ctx context.Context) {
	a.ctx = ctx

	log.Printf("CapyDeploy Hub %s starting", version.Full())

	// Start continuous discovery in background
	go a.runDiscovery()
}

// shutdown is called when the app is closing
func (a *App) shutdown(ctx context.Context) {
	// Disconnect from agent
	a.DisconnectAgent()

	// Cancel discovery goroutine context
	if a.discoveryCancel != nil {
		a.discoveryCancel()
	}

	// Stop discovery client
	if a.discoveryClient != nil {
		a.discoveryClient.Close()
	}
}

// runDiscovery handles mDNS discovery and emits events
func (a *App) runDiscovery() {
	ctx, cancel := context.WithCancel(context.Background())
	a.discoveryCancel = cancel

	// Start continuous discovery
	go a.discoveryClient.StartContinuousDiscovery(ctx, 5*time.Second)

	// Process events
	for event := range a.discoveryClient.Events() {
		a.discoveredMu.Lock()
		switch event.Type {
		case discovery.EventDiscovered:
			a.discoveredCache[event.Agent.Info.ID] = event.Agent
			runtime.EventsEmit(a.ctx, "discovery:agent-found", a.agentToInfo(event.Agent))
		case discovery.EventUpdated:
			a.discoveredCache[event.Agent.Info.ID] = event.Agent
			runtime.EventsEmit(a.ctx, "discovery:agent-updated", a.agentToInfo(event.Agent))
		case discovery.EventLost:
			delete(a.discoveredCache, event.Agent.Info.ID)
			runtime.EventsEmit(a.ctx, "discovery:agent-lost", event.Agent.Info.ID)
		}
		a.discoveredMu.Unlock()
	}
}

// agentToInfo converts a DiscoveredAgent to frontend-friendly info
func (a *App) agentToInfo(agent *discovery.DiscoveredAgent) DiscoveredAgentInfo {
	ips := make([]string, 0, len(agent.IPs))
	for _, ip := range agent.IPs {
		ips = append(ips, ip.String())
	}

	return DiscoveredAgentInfo{
		ID:           agent.Info.ID,
		Name:         agent.Info.Name,
		Platform:     agent.Info.Platform,
		Version:      agent.Info.Version,
		Host:         agent.Host,
		Port:         agent.Port,
		IPs:          ips,
		DiscoveredAt: agent.DiscoveredAt.Format(time.RFC3339),
		LastSeen:     agent.LastSeen.Format(time.RFC3339),
		Online:       !agent.IsStale(30 * time.Second),
	}
}

// =============================================================================
// Agent Discovery & Connection
// =============================================================================

// GetDiscoveredAgents returns all discovered agents
func (a *App) GetDiscoveredAgents() []DiscoveredAgentInfo {
	a.discoveredMu.RLock()
	defer a.discoveredMu.RUnlock()

	agents := make([]DiscoveredAgentInfo, 0, len(a.discoveredCache))
	for _, agent := range a.discoveredCache {
		agents = append(agents, a.agentToInfo(agent))
	}
	return agents
}

// RefreshDiscovery triggers a manual discovery scan
func (a *App) RefreshDiscovery() ([]DiscoveredAgentInfo, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	agents, err := a.discoveryClient.Discover(ctx, 3*time.Second)
	if err != nil {
		return nil, fmt.Errorf("discovery failed: %w", err)
	}

	// Update cache
	a.discoveredMu.Lock()
	for _, agent := range agents {
		a.discoveredCache[agent.Info.ID] = agent
	}
	a.discoveredMu.Unlock()

	return a.GetDiscoveredAgents(), nil
}

// ConnectAgent connects to an agent by ID using WebSocket
func (a *App) ConnectAgent(agentID string) error {
	// Find agent in cache
	a.discoveredMu.RLock()
	agent, ok := a.discoveredCache[agentID]
	a.discoveredMu.RUnlock()

	if !ok {
		return fmt.Errorf("agent not found: %s", agentID)
	}

	// Disconnect existing connection
	a.DisconnectAgent()

	// Create WebSocket client with auth
	var wsClient *modules.WSClient
	var err error

	// Get hub name from config (fallback to default)
	hubName := "CapyDeploy Hub"
	hubPlatform := ""
	if a.configMgr != nil {
		hubName = a.configMgr.GetName()
		hubPlatform = a.configMgr.GetPlatform()
	}

	if a.tokenStore != nil {
		wsClient, err = modules.WSClientFromAgentWithAuth(
			agent,
			hubName,
			version.Version,
			a.tokenStore.GetHubID(),
			a.tokenStore.GetToken,
			a.tokenStore.SaveToken,
		)
	} else {
		wsClient, err = modules.WSClientFromAgent(agent, hubName, version.Version)
	}

	// Set hub platform for agent to store
	if wsClient != nil && hubPlatform != "" {
		wsClient.SetPlatform(hubPlatform)
	}
	if err != nil {
		return fmt.Errorf("failed to create WS client: %w", err)
	}

	// Set callbacks for push events
	wsClient.SetCallbacks(
		func() {
			// On disconnect
			a.mu.Lock()
			a.connectedAgent = nil
			a.mu.Unlock()
			runtime.EventsEmit(a.ctx, "connection:changed", a.GetConnectionStatus())
		},
		func(event protocol.UploadProgressEvent) {
			// On upload progress
			runtime.EventsEmit(a.ctx, "upload:progress", UploadProgress{
				Progress: event.Percentage,
				Status:   fmt.Sprintf("Uploading: %s", event.CurrentFile),
				Done:     false,
			})
		},
		func(event protocol.OperationEvent) {
			// On operation event
			runtime.EventsEmit(a.ctx, "operation:event", event)
		},
	)

	// Set pairing callback
	wsClient.SetPairingCallback(func(agentID string) {
		runtime.EventsEmit(a.ctx, "pairing:required", agentID)
	})

	// Connect via WebSocket
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	if err := wsClient.Connect(ctx); err != nil {
		// Check if pairing is required
		if errors.Is(err, wsclient.ErrPairingRequired) {
			// Store the client for pairing completion
			a.mu.Lock()
			a.connectedAgent = &ConnectedAgent{
				Agent:    agent,
				Client:   wsClient,
				WSClient: wsClient,
				Info:     nil, // Not yet authenticated
			}
			a.mu.Unlock()

			// Emit pairing required event
			runtime.EventsEmit(a.ctx, "pairing:required", agentID)
			return nil // Not an error, waiting for pairing
		}
		return fmt.Errorf("failed to connect: %w", err)
	}

	// Get full agent info (includes capabilities)
	agentInfo, err := wsClient.GetInfo(ctx)
	if err != nil {
		wsClient.Close()
		return fmt.Errorf("failed to get agent info: %w", err)
	}

	a.mu.Lock()
	a.connectedAgent = &ConnectedAgent{
		Agent:    agent,
		Client:   wsClient,
		WSClient: wsClient,
		Info:     agentInfo,
	}
	a.mu.Unlock()

	// Emit connection status change
	runtime.EventsEmit(a.ctx, "connection:changed", a.GetConnectionStatus())

	return nil
}

// DisconnectAgent disconnects from the current agent
func (a *App) DisconnectAgent() {
	a.mu.Lock()
	if a.connectedAgent != nil && a.connectedAgent.WSClient != nil {
		a.connectedAgent.WSClient.Close()
	}
	a.connectedAgent = nil
	a.mu.Unlock()

	// Emit connection status change
	runtime.EventsEmit(a.ctx, "connection:changed", a.GetConnectionStatus())
}

// ConfirmPairing confirms a pairing with the connected agent using the provided code.
func (a *App) ConfirmPairing(code string) error {
	a.mu.RLock()
	if a.connectedAgent == nil || a.connectedAgent.WSClient == nil {
		a.mu.RUnlock()
		return fmt.Errorf("no agent connection pending pairing")
	}
	wsClient := a.connectedAgent.WSClient
	agent := a.connectedAgent.Agent
	a.mu.RUnlock()

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	// Confirm pairing
	if err := wsClient.ConfirmPairing(ctx, code); err != nil {
		return fmt.Errorf("pairing failed: %w", err)
	}

	// Now get agent info
	agentInfo, err := wsClient.GetInfo(ctx)
	if err != nil {
		return fmt.Errorf("failed to get agent info after pairing: %w", err)
	}

	a.mu.Lock()
	a.connectedAgent = &ConnectedAgent{
		Agent:    agent,
		Client:   wsClient,
		WSClient: wsClient,
		Info:     agentInfo,
	}
	a.mu.Unlock()

	// Emit connection status change
	runtime.EventsEmit(a.ctx, "connection:changed", a.GetConnectionStatus())

	return nil
}

// CancelPairing cancels a pending pairing and disconnects.
func (a *App) CancelPairing() {
	a.DisconnectAgent()
}

// GetConnectionStatus returns the current connection status
func (a *App) GetConnectionStatus() ConnectionStatus {
	a.mu.RLock()
	defer a.mu.RUnlock()

	if a.connectedAgent == nil {
		return ConnectionStatus{Connected: false}
	}

	agent := a.connectedAgent.Agent
	info := a.connectedAgent.Info

	ips := make([]string, 0, len(agent.IPs))
	for _, ip := range agent.IPs {
		ips = append(ips, ip.String())
	}

	// Convert capabilities to strings for JSON serialization
	var capabilities []string
	if info != nil {
		capabilities = make([]string, len(info.Capabilities))
		for i, cap := range info.Capabilities {
			capabilities[i] = string(cap)
		}
	}

	// Use formats from agent info if available, otherwise fall back to platform-based
	var supportedFormats []string
	if info != nil && len(info.SupportedImageFormats) > 0 {
		supportedFormats = info.SupportedImageFormats
	} else {
		supportedFormats = modules.GetSupportedImageFormats(agent.Info.Platform)
	}

	return ConnectionStatus{
		Connected:             true,
		AgentID:               agent.Info.ID,
		AgentName:             agent.Info.Name,
		Platform:              agent.Info.Platform,
		Host:                  agent.Host,
		Port:                  agent.Port,
		IPs:                   ips,
		SupportedImageFormats: supportedFormats,
		Capabilities:          capabilities,
	}
}

// GetAgentInstallPath returns the install path from the connected agent
func (a *App) GetAgentInstallPath() (string, error) {
	a.mu.RLock()
	if a.connectedAgent == nil {
		a.mu.RUnlock()
		return "", fmt.Errorf("no agent connected")
	}
	client := a.connectedAgent.Client
	a.mu.RUnlock()

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	config, err := client.GetConfig(ctx)
	if err != nil {
		return "", fmt.Errorf("failed to get agent config: %w", err)
	}

	return config.InstallPath, nil
}

// =============================================================================
// Game Setup Management
// =============================================================================

// GetGameSetups returns all saved game setups
func (a *App) GetGameSetups() ([]config.GameSetup, error) {
	return config.GetGameSetups()
}

// AddGameSetup adds a new game setup
func (a *App) AddGameSetup(setup config.GameSetup) error {
	return config.AddGameSetup(setup)
}

// UpdateGameSetup updates an existing game setup
func (a *App) UpdateGameSetup(id string, setup config.GameSetup) error {
	return config.UpdateGameSetup(id, setup)
}

// RemoveGameSetup removes a game setup
func (a *App) RemoveGameSetup(id string) error {
	return config.RemoveGameSetup(id)
}

// SelectFolder opens a folder selection dialog
func (a *App) SelectFolder() (string, error) {
	return runtime.OpenDirectoryDialog(a.ctx, runtime.OpenDialogOptions{
		Title: "Select Game Folder",
	})
}

// ArtworkFileResult contains the result of selecting a local artwork file.
type ArtworkFileResult struct {
	Path        string `json:"path"`
	DataURI     string `json:"dataURI"`
	ContentType string `json:"contentType"`
	Size        int64  `json:"size"`
}

// maxArtworkSize is the maximum allowed artwork file size (50MB).
// Animated WebP files for Steam artwork can be 20-30MB.
const maxArtworkSize = 50 * 1024 * 1024

// SelectArtworkFile opens a file dialog to select a local artwork image.
func (a *App) SelectArtworkFile() (*ArtworkFileResult, error) {
	path, err := runtime.OpenFileDialog(a.ctx, runtime.OpenDialogOptions{
		Title: "Select Artwork Image",
		Filters: []runtime.FileFilter{
			{DisplayName: "Images", Pattern: "*.png;*.jpg;*.jpeg;*.webp"},
		},
	})
	if err != nil {
		return nil, err
	}
	if path == "" {
		return nil, nil // User cancelled
	}

	return readArtworkFile(path)
}

// GetArtworkPreview returns a data URI for the given artwork file path.
func (a *App) GetArtworkPreview(path string) (string, error) {
	result, err := readArtworkFile(path)
	if err != nil {
		return "", err
	}
	return result.DataURI, nil
}

// readArtworkFile reads and validates a local artwork file.
func readArtworkFile(path string) (*ArtworkFileResult, error) {
	info, err := os.Stat(path)
	if err != nil {
		return nil, fmt.Errorf("failed to stat file: %w", err)
	}

	if info.Size() > maxArtworkSize {
		return nil, fmt.Errorf("file too large: %d bytes (max %d)", info.Size(), maxArtworkSize)
	}

	contentType := detectContentType(path)
	if contentType == "" {
		return nil, fmt.Errorf("unsupported image format: %s", filepath.Ext(path))
	}

	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read file: %w", err)
	}

	dataURI := fmt.Sprintf("data:%s;base64,%s", contentType, base64.StdEncoding.EncodeToString(data))

	return &ArtworkFileResult{
		Path:        path,
		DataURI:     dataURI,
		ContentType: contentType,
		Size:        info.Size(),
	}, nil
}

// detectContentType returns the MIME type based on file extension.
func detectContentType(path string) string {
	ext := strings.ToLower(filepath.Ext(path))
	switch ext {
	case ".png":
		return "image/png"
	case ".jpg", ".jpeg":
		return "image/jpeg"
	case ".webp":
		return "image/webp"
	default:
		// Fallback to mime package
		ct := mime.TypeByExtension(ext)
		if strings.HasPrefix(ct, "image/") {
			return ct
		}
		return ""
	}
}

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
		emitProgress(0, "", fmt.Sprintf("Upload failed: %s", completeResp.Error), true)
		return
	}

	emitProgress(1.0, "Upload complete!", "", true)
}

// =============================================================================
// Installed Games Management
// =============================================================================

// GetInstalledGames returns shortcuts from the connected agent
func (a *App) GetInstalledGames(remotePath string) ([]InstalledGame, error) {
	a.mu.RLock()
	if a.connectedAgent == nil {
		a.mu.RUnlock()
		return nil, fmt.Errorf("no agent connected")
	}
	client := a.connectedAgent.Client
	a.mu.RUnlock()

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	// Get Steam users first
	userProvider, ok := modules.AsSteamUserProvider(client)
	if !ok {
		return nil, fmt.Errorf("agent does not support Steam user listing")
	}

	users, err := userProvider.GetSteamUsers(ctx)
	if err != nil {
		return nil, fmt.Errorf("failed to get Steam users: %w", err)
	}

	if len(users) == 0 {
		return []InstalledGame{}, nil
	}

	// Get shortcuts for first user
	shortcutMgr, ok := modules.AsShortcutManager(client)
	if !ok {
		return nil, fmt.Errorf("agent does not support shortcuts")
	}

	shortcuts, err := shortcutMgr.ListShortcuts(ctx, users[0].ID)
	if err != nil {
		return nil, fmt.Errorf("failed to list shortcuts: %w", err)
	}

	games := make([]InstalledGame, 0, len(shortcuts))
	for _, sc := range shortcuts {
		games = append(games, InstalledGame{
			Name:  sc.Name,
			Path:  sc.StartDir,
			Size:  "N/A", // Agent doesn't provide size info
			AppID: sc.AppID,
		})
	}

	return games, nil
}

// DeleteGame deletes a game from the connected agent.
// The Agent handles everything internally (user detection, file deletion, Steam restart).
func (a *App) DeleteGame(name string, appID uint32) error {
	a.mu.RLock()
	if a.connectedAgent == nil {
		a.mu.RUnlock()
		return fmt.Errorf("no agent connected")
	}
	client := a.connectedAgent.Client
	a.mu.RUnlock()

	// Use longer timeout - Agent needs time for Steam restart
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Minute)
	defer cancel()

	// Use the unified GameManager endpoint - Agent handles everything
	gameMgr, ok := modules.AsGameManager(client)
	if !ok {
		return fmt.Errorf("agent does not support game management")
	}

	if _, err := gameMgr.DeleteGame(ctx, appID); err != nil {
		return fmt.Errorf("failed to delete game: %w", err)
	}

	return nil
}

// =============================================================================
// Settings
// =============================================================================

// GetSteamGridDBAPIKey returns the SteamGridDB API key
func (a *App) GetSteamGridDBAPIKey() (string, error) {
	return config.GetSteamGridDBAPIKey()
}

// SetSteamGridDBAPIKey saves the SteamGridDB API key
func (a *App) SetSteamGridDBAPIKey(apiKey string) error {
	return config.SetSteamGridDBAPIKey(apiKey)
}

// GetCacheSize returns the size of the image cache
func (a *App) GetCacheSize() (int64, error) {
	return steamgriddb.GetCacheSize()
}

// ClearImageCache clears the image cache
func (a *App) ClearImageCache() error {
	return steamgriddb.ClearImageCache()
}

// GetImageCacheEnabled returns whether image caching is enabled
func (a *App) GetImageCacheEnabled() (bool, error) {
	return config.GetImageCacheEnabled()
}

// SetImageCacheEnabled enables or disables image caching
// When disabled, automatically clears the cache
func (a *App) SetImageCacheEnabled(enabled bool) error {
	if err := config.SetImageCacheEnabled(enabled); err != nil {
		return err
	}
	// Clear cache when disabling
	if !enabled {
		return steamgriddb.ClearImageCache()
	}
	return nil
}

// OpenCacheFolder opens the cache folder in the file explorer
func (a *App) OpenCacheFolder() error {
	cacheDir, err := steamgriddb.GetImageCacheDir()
	if err != nil {
		return err
	}

	var cmd *exec.Cmd
	switch goruntime.GOOS {
	case "windows":
		cmd = exec.Command("explorer", cacheDir)
	case "darwin":
		cmd = exec.Command("open", cacheDir)
	default: // linux and others
		cmd = exec.Command("xdg-open", cacheDir)
	}

	return cmd.Start()
}

// =============================================================================
// SteamGridDB
// =============================================================================

// SearchGames searches for games on SteamGridDB
func (a *App) SearchGames(query string) ([]steamgriddb.SearchResult, error) {
	apiKey, err := config.GetSteamGridDBAPIKey()
	if err != nil || apiKey == "" {
		return nil, fmt.Errorf("SteamGridDB API key not configured")
	}

	client := steamgriddb.NewClient(apiKey)
	return client.Search(query)
}

// GetGrids returns grid images for a game
func (a *App) GetGrids(gameID int, filters steamgriddb.ImageFilters, page int) ([]steamgriddb.GridData, error) {
	apiKey, err := config.GetSteamGridDBAPIKey()
	if err != nil || apiKey == "" {
		return nil, fmt.Errorf("SteamGridDB API key not configured")
	}

	client := steamgriddb.NewClient(apiKey)
	return client.GetGrids(gameID, &filters, page)
}

// GetHeroes returns hero images for a game
func (a *App) GetHeroes(gameID int, filters steamgriddb.ImageFilters, page int) ([]steamgriddb.ImageData, error) {
	apiKey, err := config.GetSteamGridDBAPIKey()
	if err != nil || apiKey == "" {
		return nil, fmt.Errorf("SteamGridDB API key not configured")
	}

	client := steamgriddb.NewClient(apiKey)
	return client.GetHeroes(gameID, &filters, page)
}

// GetLogos returns logo images for a game
func (a *App) GetLogos(gameID int, filters steamgriddb.ImageFilters, page int) ([]steamgriddb.ImageData, error) {
	apiKey, err := config.GetSteamGridDBAPIKey()
	if err != nil || apiKey == "" {
		return nil, fmt.Errorf("SteamGridDB API key not configured")
	}

	client := steamgriddb.NewClient(apiKey)
	return client.GetLogos(gameID, &filters, page)
}

// GetIcons returns icon images for a game
func (a *App) GetIcons(gameID int, filters steamgriddb.ImageFilters, page int) ([]steamgriddb.ImageData, error) {
	apiKey, err := config.GetSteamGridDBAPIKey()
	if err != nil || apiKey == "" {
		return nil, fmt.Errorf("SteamGridDB API key not configured")
	}

	client := steamgriddb.NewClient(apiKey)
	return client.GetIcons(gameID, &filters, page)
}

// =============================================================================
// Version
// =============================================================================

// VersionInfo represents version information for the frontend.
type VersionInfo struct {
	Version   string `json:"version"`
	Commit    string `json:"commit"`
	BuildDate string `json:"buildDate"`
}

// GetVersion returns the current version information.
func (a *App) GetVersion() VersionInfo {
	return VersionInfo{
		Version:   version.Version,
		Commit:    version.Commit,
		BuildDate: version.BuildDate,
	}
}

// =============================================================================
// Hub Identity
// =============================================================================

// HubInfo represents the Hub's identity information.
type HubInfo struct {
	ID       string `json:"id"`
	Name     string `json:"name"`
	Platform string `json:"platform"`
}

// GetHubInfo returns the Hub's identity information.
func (a *App) GetHubInfo() HubInfo {
	if a.configMgr == nil {
		return HubInfo{
			ID:       "",
			Name:     "CapyDeploy Hub",
			Platform: goruntime.GOOS,
		}
	}
	return HubInfo{
		ID:       a.configMgr.GetID(),
		Name:     a.configMgr.GetName(),
		Platform: a.configMgr.GetPlatform(),
	}
}

// GetHubName returns the Hub's display name.
func (a *App) GetHubName() string {
	if a.configMgr == nil {
		return "CapyDeploy Hub"
	}
	return a.configMgr.GetName()
}

// SetHubName sets the Hub's display name.
func (a *App) SetHubName(name string) error {
	if a.configMgr == nil {
		return fmt.Errorf("config manager not initialized")
	}
	return a.configMgr.SetName(name)
}

// =============================================================================
// Helper functions
// =============================================================================

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

