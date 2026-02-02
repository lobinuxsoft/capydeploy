// Package server provides the HTTP server for CapyDeploy Agent.
package server

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"github.com/google/uuid"
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
	Port              int
	Name              string
	Version           string
	Platform          string
	Verbose           bool
	UploadPath        string                   // Base path for uploaded files (deprecated, use GetInstallPath)
	AcceptConnections func() bool              // Callback to check if connections are accepted
	GetInstallPath    func() string            // Callback to get the install path from config
	OnShortcutChange  func()                   // Callback when shortcuts are created/deleted
	OnOperation       func(event OperationEvent) // Callback for operation progress
}

// Server is the main agent server that handles HTTP requests and mDNS discovery.
type Server struct {
	cfg       Config
	id        string
	httpSrv   *http.Server
	mdnsSrv   *discovery.Server
	mu        sync.RWMutex
	startTime time.Time

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

	return &Server{
		cfg:     cfg,
		id:      id,
		uploads: make(map[string]*transfer.UploadSession),
	}, nil
}

// Run starts the HTTP server and mDNS discovery.
func (s *Server) Run(ctx context.Context) error {
	s.startTime = time.Now()
	log.Printf("Upload path: %s", s.cfg.UploadPath)

	// Setup HTTP server
	mux := http.NewServeMux()
	s.registerHandlers(mux)

	s.httpSrv = &http.Server{
		Addr:         fmt.Sprintf(":%d", s.cfg.Port),
		Handler:      mux,
		ReadTimeout:  5 * time.Minute,  // Allow time for chunk uploads
		WriteTimeout: 5 * time.Minute,
		IdleTimeout:  2 * time.Minute,
	}

	// Setup mDNS server
	s.mdnsSrv = discovery.NewServer(discovery.ServiceInfo{
		ID:       s.id,
		Name:     s.cfg.Name,
		Platform: s.cfg.Platform,
		Version:  s.cfg.Version,
		Port:     s.cfg.Port,
	})

	// Start mDNS in background
	errCh := make(chan error, 2)
	go func() {
		if err := s.mdnsSrv.Start(); err != nil {
			errCh <- fmt.Errorf("mDNS server error: %w", err)
		}
		log.Printf("mDNS service registered: %s._capydeploy._tcp.local", s.id)
	}()

	// Start HTTP server in background
	go func() {
		log.Printf("HTTP server listening on :%d", s.cfg.Port)
		if err := s.httpSrv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			errCh <- fmt.Errorf("HTTP server error: %w", err)
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

	return protocol.AgentInfo{
		ID:                s.id,
		Name:              s.cfg.Name,
		Platform:          s.cfg.Platform,
		Version:           s.cfg.Version,
		AcceptConnections: acceptConnections,
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
	basePath = expandPath(basePath)

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
	return expandPath(path)
}

// expandPath expands ~ to the user's home directory.
func expandPath(path string) string {
	if strings.HasPrefix(path, "~/") {
		home, err := os.UserHomeDir()
		if err == nil {
			return filepath.Join(home, path[2:])
		}
	}
	return path
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
