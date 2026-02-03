package main

import (
	"context"
	"fmt"
	"log"
	"net"
	"sync"

	"github.com/wailsapp/wails/v2/pkg/runtime"

	"github.com/lobinuxsoft/capydeploy/apps/agent/config"
	"github.com/lobinuxsoft/capydeploy/apps/agent/firewall"
	"github.com/lobinuxsoft/capydeploy/apps/agent/server"
	"github.com/lobinuxsoft/capydeploy/apps/agent/shortcuts"
	"github.com/lobinuxsoft/capydeploy/apps/agent/tray"
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
	configMgr  *config.Manager
	port       int
	uploadPath string

	// Connection state
	acceptConnections bool
	connectedHub      *ConnectedHub
	connectionMu      sync.RWMutex

	// System tray
	noTray bool
	tray   *tray.Tray
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
	cfgMgr, err := config.NewManager()
	if err != nil {
		log.Printf("Warning: failed to load config: %v", err)
	}

	return &App{
		configMgr:         cfgMgr,
		port:              discovery.DefaultPort,
		acceptConnections: true,
	}
}

// getName returns the configured agent name
func (a *App) getName() string {
	if a.configMgr != nil {
		return a.configMgr.GetName()
	}
	return discovery.GetHostname()
}

// startup is called when the app starts
func (a *App) startup(ctx context.Context) {
	a.ctx = ctx

	// Ensure firewall rules exist (Windows only, requires admin first time)
	if err := firewall.EnsureRules(a.port); err != nil {
		log.Printf("Warning: could not configure firewall: %v", err)
		log.Printf("You may need to run the Agent as Administrator once, or manually allow port %d", a.port)
	}

	// Start the HTTP server in background
	go a.startServer()

	// Start system tray if enabled
	if !a.noTray {
		go a.startTray()
	}
}

// shutdown is called when the app is closing
func (a *App) shutdown(ctx context.Context) {
	if a.tray != nil {
		a.tray.Quit()
	}
	if a.cancel != nil {
		a.cancel()
	}

	// Remove firewall rules on shutdown (Windows only)
	if err := firewall.RemoveRules(); err != nil {
		log.Printf("Warning: failed to remove firewall rules: %v", err)
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
		Name:       a.getName(),
		Version:    Version,
		Platform:   discovery.GetPlatform(),
		Verbose:    false,
		UploadPath: a.uploadPath,
		AcceptConnections: func() bool {
			a.connectionMu.RLock()
			defer a.connectionMu.RUnlock()
			return a.acceptConnections
		},
		GetInstallPath: func() string {
			if a.configMgr != nil {
				return a.configMgr.GetInstallPath()
			}
			return "~/Games"
		},
		OnShortcutChange: func() {
			runtime.EventsEmit(a.ctx, "shortcuts:changed", nil)
		},
		OnOperation: func(event server.OperationEvent) {
			runtime.EventsEmit(a.ctx, "operation", event)
		},
		OnHubConnect: func(hubName string) {
			a.connectionMu.Lock()
			a.connectedHub = &ConnectedHub{Name: hubName}
			a.connectionMu.Unlock()
			log.Printf("Hub connected: %s", hubName)
			runtime.EventsEmit(a.ctx, "status:changed", a.GetStatus())
			a.updateTrayStatus()
		},
		OnHubDisconnect: func() {
			a.connectionMu.Lock()
			a.connectedHub = nil
			a.connectionMu.Unlock()
			log.Printf("Hub disconnected")
			runtime.EventsEmit(a.ctx, "status:changed", a.GetStatus())
			a.updateTrayStatus()
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
		Name:              a.getName(),
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
	a.updateTrayStatus()
}

// DisconnectHub disconnects the current Hub
func (a *App) DisconnectHub() {
	a.connectionMu.Lock()
	a.connectedHub = nil
	a.connectionMu.Unlock()

	runtime.EventsEmit(a.ctx, "status:changed", a.GetStatus())
	a.updateTrayStatus()
}

// SetName changes the agent name and restarts the server
func (a *App) SetName(name string) error {
	if a.configMgr == nil {
		return fmt.Errorf("configuration not available")
	}

	if name == "" {
		return fmt.Errorf("name cannot be empty")
	}

	// Save new name
	if err := a.configMgr.SetName(name); err != nil {
		return fmt.Errorf("failed to save name: %w", err)
	}

	log.Printf("Agent name changed to: %s", name)

	// Restart server to update mDNS with new name
	a.restartServer()

	return nil
}

// GetInstallPath returns the current install path
func (a *App) GetInstallPath() string {
	if a.configMgr != nil {
		return a.configMgr.GetInstallPath()
	}
	return "~/Games"
}

// SetInstallPath changes the install path
func (a *App) SetInstallPath(path string) error {
	if a.configMgr == nil {
		return fmt.Errorf("configuration not available")
	}

	if path == "" {
		return fmt.Errorf("path cannot be empty")
	}

	if err := a.configMgr.SetInstallPath(path); err != nil {
		return fmt.Errorf("failed to save install path: %w", err)
	}

	log.Printf("Install path changed to: %s", path)
	runtime.EventsEmit(a.ctx, "status:changed", a.GetStatus())

	return nil
}

// SelectInstallPath opens a folder selection dialog and returns the selected path
func (a *App) SelectInstallPath() (string, error) {
	path, err := runtime.OpenDirectoryDialog(a.ctx, runtime.OpenDialogOptions{
		Title: "Select Install Path",
	})
	if err != nil {
		return "", err
	}
	if path == "" {
		return "", nil // User cancelled
	}

	// Save the selected path
	if err := a.SetInstallPath(path); err != nil {
		return "", err
	}

	return path, nil
}

// restartServer stops and starts the server with current config
func (a *App) restartServer() {
	// Cancel current server
	if a.cancel != nil {
		a.cancel()
	}

	// Wait a bit for cleanup
	a.serverMu.Lock()
	a.server = nil
	a.serverMu.Unlock()

	// Start new server
	go a.startServer()
}

// =============================================================================
// System Tray
// =============================================================================

// startTray initializes and runs the system tray
func (a *App) startTray() {
	a.tray = tray.New(tray.Config{
		OnOpenWebUI: func() {
			runtime.BrowserOpenURL(a.ctx, fmt.Sprintf("http://localhost:%d", a.port))
		},
		OnCopyAddress: func() string {
			return a.getAddress()
		},
		OnToggleAccept: func(accept bool) {
			a.SetAcceptConnections(accept)
		},
		OnQuit: func() {
			runtime.Quit(a.ctx)
		},
		GetStatus: a.getTrayStatus,
	})
	a.tray.Run()
}

// getTrayStatus returns the current status for the tray
func (a *App) getTrayStatus() tray.Status {
	a.serverMu.RLock()
	running := a.server != nil
	a.serverMu.RUnlock()

	a.connectionMu.RLock()
	connectedHub := a.connectedHub
	acceptConnections := a.acceptConnections
	a.connectionMu.RUnlock()

	var hubInfo *tray.HubInfo
	if connectedHub != nil {
		hubInfo = &tray.HubInfo{
			Name: connectedHub.Name,
			IP:   connectedHub.IP,
		}
	}

	return tray.Status{
		Running:           running,
		AcceptConnections: acceptConnections,
		ConnectedHub:      hubInfo,
		Name:              a.getName(),
		Address:           a.getAddress(),
	}
}

// updateTrayStatus notifies the tray of a status change
func (a *App) updateTrayStatus() {
	if a.tray != nil {
		a.tray.UpdateStatus(a.getTrayStatus())
	}
}

// getAddress returns the first local IP with port
func (a *App) getAddress() string {
	ips := getLocalIPs()
	if len(ips) > 0 {
		return fmt.Sprintf("%s:%d", ips[0], a.port)
	}
	return fmt.Sprintf("localhost:%d", a.port)
}

// =============================================================================
// Helper functions
// =============================================================================

// getLocalIPs returns the local IP addresses, filtering out link-local (APIPA) addresses.
func getLocalIPs() []string {
	var ips []string

	addrs, err := net.InterfaceAddrs()
	if err != nil {
		return ips
	}

	for _, addr := range addrs {
		if ipnet, ok := addr.(*net.IPNet); ok && !ipnet.IP.IsLoopback() {
			ip4 := ipnet.IP.To4()
			if ip4 != nil {
				// Skip link-local addresses (169.254.x.x / APIPA)
				if ip4[0] == 169 && ip4[1] == 254 {
					continue
				}
				ips = append(ips, ip4.String())
			}
		}
	}

	return ips
}
