package main

import (
	"context"
	"log"
	"sync"

	"github.com/lobinuxsoft/capydeploy/apps/hub/auth"
	hubconfig "github.com/lobinuxsoft/capydeploy/apps/hub/config"
	"github.com/lobinuxsoft/capydeploy/apps/hub/modules"
	"github.com/lobinuxsoft/capydeploy/pkg/discovery"
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/version"
)

// App struct holds the application state
type App struct {
	ctx             context.Context
	connectedAgent  *ConnectedAgent
	discoveryClient *discovery.Client
	discoveryCancel context.CancelFunc
	discoveredMu    sync.RWMutex
	discoveredCache map[string]*discovery.DiscoveredAgent
	mu              sync.RWMutex
	tokenStore      *auth.TokenStore
	configMgr       *hubconfig.Manager
}

// ConnectedAgent represents a connected agent with its client
type ConnectedAgent struct {
	Agent    *discovery.DiscoveredAgent
	Client   modules.PlatformClient  // Interface for capability checks (type assertions)
	WSClient *modules.WSClient       // WebSocket client for WS-specific operations
	Info     *protocol.AgentInfo     // Full agent info from WS connection
}

// ConnectionStatus represents the current connection status
type ConnectionStatus struct {
	Connected             bool     `json:"connected"`
	AgentID               string   `json:"agentId"`
	AgentName             string   `json:"agentName"`
	Platform              string   `json:"platform"`
	Host                  string   `json:"host"`
	Port                  int      `json:"port"`
	IPs                   []string `json:"ips"`
	SupportedImageFormats []string `json:"supportedImageFormats"`
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
	Name  string `json:"name"`
	Path  string `json:"path"`
	Size  string `json:"size"`
	AppID uint32 `json:"appId,omitempty"`
}

// UploadProgress represents upload progress data
type UploadProgress struct {
	Progress float64 `json:"progress"`
	Status   string  `json:"status"`
	Error    string  `json:"error,omitempty"`
	Done     bool    `json:"done"`
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

// HubInfo represents the Hub's identity information.
type HubInfo struct {
	ID       string `json:"id"`
	Name     string `json:"name"`
	Platform string `json:"platform"`
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
