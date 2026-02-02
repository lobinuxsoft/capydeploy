package main

import (
	"context"
	"fmt"
	"log"
	"net"
	"sync"

	"github.com/wailsapp/wails/v2/pkg/runtime"

	"github.com/lobinuxsoft/capydeploy/apps/agent/server"
	"github.com/lobinuxsoft/capydeploy/apps/agent/shortcuts"
	"github.com/lobinuxsoft/capydeploy/pkg/discovery"
	"github.com/lobinuxsoft/capydeploy/pkg/steam"
)

// Version is set at build time.
var Version = "dev"

// App struct holds the application state
type App struct {
	ctx    context.Context
	cancel context.CancelFunc

	server    *server.Server
	serverMu  sync.RWMutex
	serverCtx context.Context

	// Agent configuration
	port       int
	name       string
	uploadPath string

	// Connection state
	acceptConnections bool
	connectedHub      *ConnectedHub
	connectionMu      sync.RWMutex
}

// ConnectedHub represents a connected Hub
type ConnectedHub struct {
	Name string `json:"name"`
	IP   string `json:"ip"`
}

// AgentStatus represents the current agent status for the UI
type AgentStatus struct {
	Running           bool          `json:"running"`
	Name              string        `json:"name"`
	Platform          string        `json:"platform"`
	Version           string        `json:"version"`
	Port              int           `json:"port"`
	IPs               []string      `json:"ips"`
	AcceptConnections bool          `json:"acceptConnections"`
	ConnectedHub      *ConnectedHub `json:"connectedHub"`
}

// SteamUserInfo represents a Steam user for the UI
type SteamUserInfo struct {
	ID   string `json:"id"`
	Name string `json:"name"`
}

// ShortcutInfo represents a shortcut for the UI
type ShortcutInfo struct {
	AppID    uint32 `json:"appId"`
	Name     string `json:"name"`
	Exe      string `json:"exe"`
	StartDir string `json:"startDir"`
}

// NewApp creates a new App application struct
func NewApp() *App {
	return &App{
		port:              discovery.DefaultPort,
		name:              discovery.GetHostname(),
		acceptConnections: true,
	}
}

// startup is called when the app starts
func (a *App) startup(ctx context.Context) {
	a.ctx = ctx

	// Start the HTTP server in background
	go a.startServer()
}

// shutdown is called when the app is closing
func (a *App) shutdown(ctx context.Context) {
	if a.cancel != nil {
		a.cancel()
	}
}

// startServer starts the HTTP server for Hub connections
func (a *App) startServer() {
	a.serverMu.Lock()

	serverCtx, cancel := context.WithCancel(context.Background())
	a.serverCtx = serverCtx
	a.cancel = cancel

	cfg := server.Config{
		Port:       a.port,
		Name:       a.name,
		Version:    Version,
		Platform:   discovery.GetPlatform(),
		Verbose:    false,
		UploadPath: a.uploadPath,
		AcceptConnections: func() bool {
			a.connectionMu.RLock()
			defer a.connectionMu.RUnlock()
			return a.acceptConnections
		},
	}

	srv, err := server.New(cfg)
	if err != nil {
		log.Printf("Error creating server: %v", err)
		a.serverMu.Unlock()
		runtime.EventsEmit(a.ctx, "server:error", err.Error())
		return
	}
	a.server = srv
	a.serverMu.Unlock()

	log.Printf("CapyDeploy Agent v%s starting on port %d", Version, a.port)
	log.Printf("Platform: %s, Name: %s", cfg.Platform, cfg.Name)

	runtime.EventsEmit(a.ctx, "server:started", a.GetStatus())

	if err := srv.Run(serverCtx); err != nil && err != context.Canceled {
		log.Printf("Server error: %v", err)
		runtime.EventsEmit(a.ctx, "server:error", err.Error())
	}

	runtime.EventsEmit(a.ctx, "server:stopped", nil)
}

// =============================================================================
// Wails Bindings - Called from frontend
// =============================================================================

// GetStatus returns the current agent status
func (a *App) GetStatus() AgentStatus {
	a.serverMu.RLock()
	running := a.server != nil
	a.serverMu.RUnlock()

	a.connectionMu.RLock()
	connectedHub := a.connectedHub
	acceptConnections := a.acceptConnections
	a.connectionMu.RUnlock()

	return AgentStatus{
		Running:           running,
		Name:              a.name,
		Platform:          discovery.GetPlatform(),
		Version:           Version,
		Port:              a.port,
		IPs:               getLocalIPs(),
		AcceptConnections: acceptConnections,
		ConnectedHub:      connectedHub,
	}
}

// GetSteamUsers returns the list of Steam users
func (a *App) GetSteamUsers() ([]SteamUserInfo, error) {
	users, err := steam.GetUsers()
	if err != nil {
		return nil, fmt.Errorf("failed to get Steam users: %w", err)
	}

	result := make([]SteamUserInfo, len(users))
	for i, u := range users {
		result[i] = SteamUserInfo{
			ID:   u.ID,
			Name: u.ID, // Steam User ID as name (no username available locally)
		}
	}
	return result, nil
}

// GetShortcuts returns shortcuts for a Steam user
func (a *App) GetShortcuts(userID string) ([]ShortcutInfo, error) {
	mgr, err := shortcuts.NewManager()
	if err != nil {
		return nil, fmt.Errorf("failed to create shortcut manager: %w", err)
	}

	list, err := mgr.List(userID)
	if err != nil {
		return nil, fmt.Errorf("failed to list shortcuts: %w", err)
	}

	result := make([]ShortcutInfo, len(list))
	for i, s := range list {
		result[i] = ShortcutInfo{
			AppID:    s.AppID,
			Name:     s.Name,
			Exe:      s.Exe,
			StartDir: s.StartDir,
		}
	}
	return result, nil
}

// SetAcceptConnections enables or disables new connections
func (a *App) SetAcceptConnections(accept bool) {
	a.connectionMu.Lock()
	a.acceptConnections = accept
	a.connectionMu.Unlock()

	runtime.EventsEmit(a.ctx, "status:changed", a.GetStatus())
}

// DisconnectHub disconnects the current Hub
func (a *App) DisconnectHub() {
	a.connectionMu.Lock()
	a.connectedHub = nil
	a.connectionMu.Unlock()

	runtime.EventsEmit(a.ctx, "status:changed", a.GetStatus())
}

// =============================================================================
// Helper functions
// =============================================================================

// getLocalIPs returns the local IP addresses
func getLocalIPs() []string {
	var ips []string

	addrs, err := net.InterfaceAddrs()
	if err != nil {
		return ips
	}

	for _, addr := range addrs {
		if ipnet, ok := addr.(*net.IPNet); ok && !ipnet.IP.IsLoopback() {
			if ipnet.IP.To4() != nil {
				ips = append(ips, ipnet.IP.String())
			}
		}
	}

	return ips
}
