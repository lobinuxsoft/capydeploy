// Package server provides the WebSocket server for CapyDeploy Agent.
package server

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"log"
	"net"
	"net/http"
	"os"
	"path/filepath"
	"runtime"
	"sync"
	"time"

	"github.com/google/uuid"
	"github.com/lobinuxsoft/capydeploy/apps/agents/desktop/auth"
	"github.com/lobinuxsoft/capydeploy/apps/agents/desktop/pathutil"
	"github.com/lobinuxsoft/capydeploy/pkg/discovery"
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/transfer"
)

// OperationEvent represents an operation notification for the UI.
type OperationEvent struct {
	Type     string  `json:"type"`     // "install", "delete"
	Status   string  `json:"status"`   // "start", "progress", "complete", "error"
	GameName string  `json:"gameName"`
	Progress float64 `json:"progress"` // 0-100
	Message  string  `json:"message,omitempty"`
}

// Config holds the agent server configuration.
type Config struct {
	Port              int                                        // Port to listen on (0 = dynamic, OS assigns)
	Name              string
	Version           string
	Platform          string
	Verbose           bool
	UploadPath        string                                     // Base path for uploaded files (deprecated, use GetInstallPath)
	AcceptConnections func() bool                                // Callback to check if connections are accepted
	GetInstallPath    func() string                              // Callback to get the install path from config
	OnShortcutChange  func()                                     // Callback when shortcuts are created/deleted
	OnOperation       func(event OperationEvent)                 // Callback for operation progress
	OnHubConnect      func(hubID, hubName, hubIP string)         // Callback when a Hub connects
	OnHubDisconnect   func()                                     // Callback when the Hub disconnects
	AuthManager       *auth.Manager                              // Authentication manager for pairing
	OnPairingCode     func(code string, expiresIn time.Duration) // Callback when pairing code is generated
	OnPairingSuccess  func()                                     // Callback when pairing is successful
	OnPortAssigned    func(port int)                             // Callback when port is assigned (useful for dynamic ports)
}

// Server is the main agent server that handles WebSocket connections and mDNS discovery.
type Server struct {
	cfg        Config
	id         string
	actualPort int // The actual port assigned by the OS (may differ from cfg.Port if 0 was specified)
	httpSrv    *http.Server
	mdnsSrv    *discovery.Server
	wsSrv      *WSServer
	authMgr    *auth.Manager
	mu         sync.RWMutex
	startTime  time.Time

	// Upload management
	uploadMu sync.RWMutex
	uploads  map[string]*transfer.UploadSession
}

// New creates a new agent server.
func New(cfg Config) (*Server, error) {
	// Generate stable ID based on name + platform (survives restarts)
	hash := sha256.Sum256([]byte(cfg.Name + "-" + cfg.Platform))
	id := hex.EncodeToString(hash[:])[:8]

	// Set default upload path if not specified
	if cfg.UploadPath == "" {
		home, err := os.UserHomeDir()
		if err != nil {
			return nil, fmt.Errorf("failed to get home directory: %w", err)
		}
		cfg.UploadPath = filepath.Join(home, "Games")
	}

	// Ensure upload directory exists
	if err := os.MkdirAll(cfg.UploadPath, 0755); err != nil {
		return nil, fmt.Errorf("failed to create upload directory: %w", err)
	}

	srv := &Server{
		cfg:     cfg,
		id:      id,
		authMgr: cfg.AuthManager,
		uploads: make(map[string]*transfer.UploadSession),
	}

	// Set pairing code callback if provided
	if srv.authMgr != nil && cfg.OnPairingCode != nil {
		srv.authMgr.SetPairingCodeCallback(cfg.OnPairingCode)
	}

	return srv, nil
}

// Run starts the WebSocket server and mDNS discovery.
// If cfg.Port is 0, the OS will assign an available port dynamically.
func (s *Server) Run(ctx context.Context) error {
	s.startTime = time.Now()
	log.Printf("Upload path: %s", s.cfg.UploadPath)

	// Setup WebSocket server
	s.wsSrv = NewWSServer(s, s.authMgr, s.cfg.OnHubConnect, s.cfg.OnHubDisconnect)

	// Setup HTTP server with WebSocket endpoint
	mux := http.NewServeMux()
	mux.HandleFunc("GET /ws", s.wsSrv.HandleWS)
	// Keep legacy HTTP endpoints for backward compatibility during migration
	s.registerHandlers(mux)

	// Create TCP listener - use :0 for dynamic port assignment
	listener, err := net.Listen("tcp", fmt.Sprintf(":%d", s.cfg.Port))
	if err != nil {
		return fmt.Errorf("failed to create listener: %w", err)
	}

	// Get the actual port assigned by the OS
	s.actualPort = listener.Addr().(*net.TCPAddr).Port
	log.Printf("Assigned port: %d", s.actualPort)

	// Notify caller about the assigned port
	if s.cfg.OnPortAssigned != nil {
		s.cfg.OnPortAssigned(s.actualPort)
	}

	s.httpSrv = &http.Server{
		Handler:      mux,
		ReadTimeout:  5 * time.Minute, // Allow time for chunk uploads
		WriteTimeout: 5 * time.Minute,
		IdleTimeout:  2 * time.Minute,
	}

	// Setup mDNS server with the actual assigned port
	s.mdnsSrv = discovery.NewServer(discovery.ServiceInfo{
		ID:       s.id,
		Name:     s.cfg.Name,
		Platform: s.cfg.Platform,
		Version:  s.cfg.Version,
		Port:     s.actualPort,
	})

	// Start mDNS in background
	errCh := make(chan error, 2)
	go func() {
		if err := s.mdnsSrv.Start(); err != nil {
			errCh <- fmt.Errorf("mDNS server error: %w", err)
		}
		log.Printf("mDNS service registered: %s._capydeploy._tcp.local (port %d)", s.id, s.actualPort)
	}()

	// Start HTTP/WS server in background
	go func() {
		log.Printf("WebSocket server listening on :%d/ws", s.actualPort)
		if err := s.httpSrv.Serve(listener); err != nil && err != http.ErrServerClosed {
			errCh <- fmt.Errorf("server error: %w", err)
		}
	}()

	// Wait for context cancellation or error
	select {
	case <-ctx.Done():
		return s.shutdown()
	case err := <-errCh:
		s.shutdown()
		return err
	}
}

// shutdown gracefully stops all services.
func (s *Server) shutdown() error {
	var errs []error

	// Stop HTTP server
	if s.httpSrv != nil {
		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()
		if err := s.httpSrv.Shutdown(ctx); err != nil {
			errs = append(errs, fmt.Errorf("HTTP shutdown: %w", err))
		}
	}

	// Stop mDNS server
	if s.mdnsSrv != nil {
		if err := s.mdnsSrv.Stop(); err != nil {
			errs = append(errs, fmt.Errorf("mDNS shutdown: %w", err))
		}
	}

	if len(errs) > 0 {
		return fmt.Errorf("shutdown errors: %v", errs)
	}
	return nil
}

// GetInfo returns the agent information.
func (s *Server) GetInfo() protocol.AgentInfo {
	s.mu.RLock()
	defer s.mu.RUnlock()

	acceptConnections := true
	if s.cfg.AcceptConnections != nil {
		acceptConnections = s.cfg.AcceptConnections()
	}

	// Determine supported image formats based on platform.
	// Windows Steam only supports PNG/JPEG for shortcut artwork.
	// Linux/macOS Steam supports WebP and GIF as well.
	var supportedFormats []string
	if runtime.GOOS == "windows" {
		supportedFormats = []string{"image/png", "image/jpeg"}
	} else {
		supportedFormats = []string{"image/png", "image/jpeg", "image/webp", "image/gif"}
	}

	// PC Agent with Steam supports all capabilities.
	capabilities := []protocol.Capability{
		protocol.CapFileUpload,
		protocol.CapFileList,
		protocol.CapSteamShortcuts,
		protocol.CapSteamArtwork,
		protocol.CapSteamUsers,
		protocol.CapSteamRestart,
	}

	return protocol.AgentInfo{
		ID:                    s.id,
		Name:                  s.cfg.Name,
		Platform:              s.cfg.Platform,
		Version:               s.cfg.Version,
		AcceptConnections:     acceptConnections,
		SupportedImageFormats: supportedFormats,
		Capabilities:          capabilities,
	}
}

// Upload management methods

// CreateUpload creates a new upload session.
func (s *Server) CreateUpload(config protocol.UploadConfig, totalBytes int64, files []transfer.FileEntry) *transfer.UploadSession {
	s.uploadMu.Lock()
	defer s.uploadMu.Unlock()

	id := uuid.New().String()
	session := transfer.NewUploadSession(id, config, totalBytes, files)
	s.uploads[id] = session

	return session
}

// GetUpload returns an upload session by ID.
func (s *Server) GetUpload(id string) (*transfer.UploadSession, bool) {
	s.uploadMu.RLock()
	defer s.uploadMu.RUnlock()

	session, ok := s.uploads[id]
	return session, ok
}

// DeleteUpload removes an upload session.
func (s *Server) DeleteUpload(id string) {
	s.uploadMu.Lock()
	defer s.uploadMu.Unlock()

	delete(s.uploads, id)
}

// GetUploadPath returns the full path for an upload.
// Uses the install path from config callback, or falls back to UploadPath.
func (s *Server) GetUploadPath(gameName, installPath string) string {
	var basePath string

	// Priority: 1. GetInstallPath callback, 2. UploadPath config
	if s.cfg.GetInstallPath != nil {
		basePath = s.cfg.GetInstallPath()
	} else {
		basePath = s.cfg.UploadPath
	}

	// Expand ~ to home directory
	basePath = pathutil.ExpandHome(basePath)

	return filepath.Join(basePath, gameName)
}

// GetInstallPath returns the current install path (expanded).
func (s *Server) GetInstallPath() string {
	var path string
	if s.cfg.GetInstallPath != nil {
		path = s.cfg.GetInstallPath()
	} else {
		path = s.cfg.UploadPath
	}
	return pathutil.ExpandHome(path)
}

// NotifyShortcutChange calls the OnShortcutChange callback if set.
func (s *Server) NotifyShortcutChange() {
	if s.cfg.OnShortcutChange != nil {
		s.cfg.OnShortcutChange()
	}
}

// NotifyOperation emits an operation event to the UI.
func (s *Server) NotifyOperation(opType, status, gameName string, progress float64, message string) {
	if s.cfg.OnOperation != nil {
		s.cfg.OnOperation(OperationEvent{
			Type:     opType,
			Status:   status,
			GameName: gameName,
			Progress: progress,
			Message:  message,
		})
	}
}

// IsHubConnected returns true if a Hub is currently connected via WebSocket.
func (s *Server) IsHubConnected() bool {
	if s.wsSrv == nil {
		return false
	}
	return s.wsSrv.IsConnected()
}

// GetConnectedHub returns the name of the connected Hub, or empty string if none.
func (s *Server) GetConnectedHub() string {
	if s.wsSrv == nil {
		return ""
	}
	return s.wsSrv.GetConnectedHub()
}

// DisconnectHub disconnects the currently connected Hub.
func (s *Server) DisconnectHub() {
	if s.wsSrv != nil {
		s.wsSrv.DisconnectHub()
	}
}

// SendEvent sends a push event to the connected Hub via WebSocket.
func (s *Server) SendEvent(msgType protocol.MessageType, payload any) {
	if s.wsSrv != nil {
		s.wsSrv.SendEvent(msgType, payload)
	}
}

// GetPort returns the actual port the server is listening on.
// This may differ from the configured port if 0 was specified (dynamic port).
func (s *Server) GetPort() int {
	return s.actualPort
}
