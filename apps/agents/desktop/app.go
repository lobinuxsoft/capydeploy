package main

import (
	"context"
	"fmt"
	"log"
	"net"
	"sync"
	"time"

	"github.com/wailsapp/wails/v2/pkg/runtime"

	"github.com/lobinuxsoft/capydeploy/apps/agents/desktop/auth"
	"github.com/lobinuxsoft/capydeploy/apps/agents/desktop/config"
	"github.com/lobinuxsoft/capydeploy/apps/agents/desktop/firewall"
	"github.com/lobinuxsoft/capydeploy/apps/agents/desktop/server"
	"github.com/lobinuxsoft/capydeploy/apps/agents/desktop/shortcuts"
	agentSteam "github.com/lobinuxsoft/capydeploy/apps/agents/desktop/steam"
	"github.com/lobinuxsoft/capydeploy/apps/agents/desktop/tray"
	"github.com/lobinuxsoft/capydeploy/pkg/discovery"
	"github.com/lobinuxsoft/capydeploy/pkg/steam"
	"github.com/lobinuxsoft/capydeploy/pkg/version"
)

// App struct holds the application state
type App struct {
	ctx    context.Context
	cancel context.CancelFunc

	server    *server.Server
	serverMu  sync.RWMutex
	serverCtx context.Context

	// Agent configuration
	configMgr  *config.Manager
	authMgr    *auth.Manager
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
	ID   string `json:"id"`
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
	TelemetryEnabled  bool          `json:"telemetryEnabled"`
	TelemetryInterval int           `json:"telemetryInterval"`
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

	// Create auth manager with config storage
	var authMgr *auth.Manager
	if cfgMgr != nil {
		storage := auth.NewConfigStorage(cfgMgr)
		authMgr = auth.NewManager(storage)
	}

	return &App{
		configMgr:         cfgMgr,
		authMgr:           authMgr,
		port:              0, // Dynamic port - OS will assign
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

	// Start the HTTP server in background
	// Firewall rules are configured after port is assigned (see OnPortAssigned callback)
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
		Version:    version.Version,
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
		OnHubConnect: func(hubID, hubName, hubIP string) {
			a.connectionMu.Lock()
			a.connectedHub = &ConnectedHub{ID: hubID, Name: hubName, IP: hubIP}
			a.connectionMu.Unlock()
			log.Printf("Hub connected: %s (%s)", hubName, hubIP)
			runtime.EventsEmit(a.ctx, "status:changed", a.GetStatus())
			a.updateTrayStatus()
			// Hide pairing code on successful connection
			if a.tray != nil {
				a.tray.HidePairingCode()
			}
		},
		OnHubDisconnect: func() {
			a.connectionMu.Lock()
			a.connectedHub = nil
			a.connectionMu.Unlock()
			log.Printf("Hub disconnected")
			runtime.EventsEmit(a.ctx, "status:changed", a.GetStatus())
			a.updateTrayStatus()
		},
		AuthManager: a.authMgr,
		OnPairingCode: func(code string, expiresIn time.Duration) {
			log.Printf("Pairing code generated: %s (expires in %v)", code, expiresIn)
			runtime.EventsEmit(a.ctx, "pairing:code", code)
			if a.tray != nil {
				a.tray.ShowPairingCode(code, expiresIn)
			}
		},
		OnPairingSuccess: func() {
			runtime.EventsEmit(a.ctx, "pairing:success", nil)
			runtime.EventsEmit(a.ctx, "hubs:changed", nil)
			if a.tray != nil {
				a.tray.HidePairingCode()
			}
		},
		OnPortAssigned: func(port int) {
			a.port = port
			log.Printf("Port assigned: %d", port)

			// Configure firewall now that we know the port (Windows only)
			if err := firewall.EnsureRules(port); err != nil {
				log.Printf("Warning: could not configure firewall: %v", err)
				log.Printf("You may need to run the Agent as Administrator once")
			}

			// Emit status update with the actual port
			runtime.EventsEmit(a.ctx, "status:changed", a.GetStatus())
			a.updateTrayStatus()
		},
		GetTelemetryEnabled: func() bool {
			if a.configMgr != nil {
				return a.configMgr.GetTelemetryEnabled()
			}
			return false
		},
		GetTelemetryInterval: func() int {
			if a.configMgr != nil {
				return a.configMgr.GetTelemetryInterval()
			}
			return 2
		},
		GetSteamStatus: func() (bool, bool) {
			ctrl := agentSteam.NewController()
			return ctrl.IsRunning(), ctrl.IsGamingMode()
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

	log.Printf("CapyDeploy Agent %s starting on port %d", version.Full(), a.port)
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

	var telemetryEnabled bool
	var telemetryInterval int
	if a.configMgr != nil {
		telemetryEnabled = a.configMgr.GetTelemetryEnabled()
		telemetryInterval = a.configMgr.GetTelemetryInterval()
	}

	return AgentStatus{
		Running:           running,
		Name:              a.getName(),
		Platform:          discovery.GetPlatform(),
		Version:           version.Version,
		Port:              a.port,
		IPs:               getLocalIPs(),
		AcceptConnections: acceptConnections,
		ConnectedHub:      connectedHub,
		TelemetryEnabled:  telemetryEnabled,
		TelemetryInterval: telemetryInterval,
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

// DeleteShortcut deletes a shortcut by appID for a given user
func (a *App) DeleteShortcut(userID string, appID uint32) error {
	mgr, err := shortcuts.NewManager()
	if err != nil {
		return fmt.Errorf("failed to create shortcut manager: %w", err)
	}

	// Get shortcut info before deleting (for notification)
	list, _ := mgr.List(userID)
	var gameName string
	for _, s := range list {
		if s.AppID == appID {
			gameName = s.Name
			break
		}
	}

	// Notify UI about delete start
	runtime.EventsEmit(a.ctx, "operation", map[string]interface{}{
		"type":     "delete",
		"status":   "start",
		"gameName": gameName,
		"progress": 0,
		"message":  "Eliminando...",
	})

	if err := mgr.Delete(userID, appID); err != nil {
		// Notify UI about error
		runtime.EventsEmit(a.ctx, "operation", map[string]interface{}{
			"type":     "delete",
			"status":   "error",
			"gameName": gameName,
			"progress": 0,
			"message":  err.Error(),
		})
		return fmt.Errorf("failed to delete shortcut: %w", err)
	}

	// Notify shortcuts changed
	runtime.EventsEmit(a.ctx, "shortcuts:changed", nil)

	// Notify UI about delete complete
	runtime.EventsEmit(a.ctx, "operation", map[string]interface{}{
		"type":     "delete",
		"status":   "complete",
		"gameName": gameName,
		"progress": 100,
		"message":  "Eliminado",
	})

	log.Printf("Deleted shortcut '%s' (AppID: %d) for user %s", gameName, appID, userID)
	return nil
}

// SetAcceptConnections enables or disables new connections
func (a *App) SetAcceptConnections(accept bool) {
	a.connectionMu.Lock()
	a.acceptConnections = accept
	a.connectionMu.Unlock()

	runtime.EventsEmit(a.ctx, "status:changed", a.GetStatus())
	a.updateTrayStatus()
}

// GetTelemetryEnabled returns whether telemetry is enabled
func (a *App) GetTelemetryEnabled() bool {
	if a.configMgr != nil {
		return a.configMgr.GetTelemetryEnabled()
	}
	return false
}

// SetTelemetryEnabled enables or disables telemetry streaming
func (a *App) SetTelemetryEnabled(enabled bool) error {
	if a.configMgr == nil {
		return fmt.Errorf("configuration not available")
	}

	if err := a.configMgr.SetTelemetryEnabled(enabled); err != nil {
		return fmt.Errorf("failed to save telemetry setting: %w", err)
	}

	log.Printf("Telemetry enabled: %v", enabled)

	// Start or stop telemetry on the server
	a.serverMu.RLock()
	srv := a.server
	a.serverMu.RUnlock()

	if srv != nil {
		if enabled {
			srv.StartTelemetry()
		} else {
			srv.StopTelemetry()
			srv.NotifyTelemetryStatus()
		}
	}

	runtime.EventsEmit(a.ctx, "status:changed", a.GetStatus())
	return nil
}

// GetTelemetryInterval returns the telemetry interval in seconds
func (a *App) GetTelemetryInterval() int {
	if a.configMgr != nil {
		return a.configMgr.GetTelemetryInterval()
	}
	return 2
}

// SetTelemetryInterval sets the telemetry interval in seconds (1-10)
func (a *App) SetTelemetryInterval(seconds int) error {
	if a.configMgr == nil {
		return fmt.Errorf("configuration not available")
	}

	if err := a.configMgr.SetTelemetryInterval(seconds); err != nil {
		return fmt.Errorf("failed to save telemetry interval: %w", err)
	}

	log.Printf("Telemetry interval changed to: %ds", seconds)

	// Update running collector if active
	a.serverMu.RLock()
	srv := a.server
	a.serverMu.RUnlock()

	if srv != nil && a.configMgr.GetTelemetryEnabled() {
		srv.StopTelemetry()
		srv.StartTelemetry()
	}

	runtime.EventsEmit(a.ctx, "status:changed", a.GetStatus())
	return nil
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

// =============================================================================
// Authorized Hubs Management
// =============================================================================

// AuthorizedHubInfo represents an authorized Hub for the UI.
type AuthorizedHubInfo struct {
	ID       string `json:"id"`
	Name     string `json:"name"`
	PairedAt string `json:"pairedAt"`
	LastSeen string `json:"lastSeen"`
}

// GetAuthorizedHubs returns the list of authorized Hubs.
func (a *App) GetAuthorizedHubs() []AuthorizedHubInfo {
	if a.authMgr == nil {
		return []AuthorizedHubInfo{}
	}

	hubs := a.authMgr.GetAuthorizedHubs()
	result := make([]AuthorizedHubInfo, len(hubs))
	for i, h := range hubs {
		result[i] = AuthorizedHubInfo{
			ID:       h.ID,
			Name:     h.Name,
			PairedAt: h.PairedAt.Format(time.RFC3339),
			LastSeen: h.LastSeen.Format(time.RFC3339),
		}
	}
	return result
}

// RevokeHub removes a Hub from the authorized list.
func (a *App) RevokeHub(hubID string) error {
	if a.authMgr == nil {
		return fmt.Errorf("authentication not configured")
	}

	if err := a.authMgr.RevokeHub(hubID); err != nil {
		return fmt.Errorf("failed to revoke hub: %w", err)
	}

	log.Printf("Revoked Hub: %s", hubID)

	// Disconnect the Hub if it's currently connected
	a.connectionMu.RLock()
	isConnected := a.connectedHub != nil && a.connectedHub.ID == hubID
	a.connectionMu.RUnlock()

	if isConnected {
		a.serverMu.RLock()
		srv := a.server
		a.serverMu.RUnlock()
		if srv != nil {
			srv.DisconnectHub()
		}
	}

	runtime.EventsEmit(a.ctx, "auth:hub-revoked", hubID)
	return nil
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

// GetVersion returns the current version information.
func (a *App) GetVersion() version.Info {
	return version.GetInfo()
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
