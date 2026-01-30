// Package server provides the HTTP server for CapyDeploy Agent.
package server

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"sync"
	"time"

	"github.com/google/uuid"
	"github.com/lobinuxsoft/capydeploy/pkg/discovery"
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

// Config holds the agent server configuration.
type Config struct {
	Port     int
	Name     string
	Version  string
	Platform string
	Verbose  bool
}

// Server is the main agent server that handles HTTP requests and mDNS discovery.
type Server struct {
	cfg       Config
	id        string
	httpSrv   *http.Server
	mdnsSrv   *discovery.Server
	mu        sync.RWMutex
	startTime time.Time
}

// New creates a new agent server.
func New(cfg Config) (*Server, error) {
	id := uuid.New().String()[:8]

	return &Server{
		cfg: cfg,
		id:  id,
	}, nil
}

// Run starts the HTTP server and mDNS discovery.
func (s *Server) Run(ctx context.Context) error {
	s.startTime = time.Now()

	// Setup HTTP server
	mux := http.NewServeMux()
	s.registerHandlers(mux)

	s.httpSrv = &http.Server{
		Addr:         fmt.Sprintf(":%d", s.cfg.Port),
		Handler:      mux,
		ReadTimeout:  30 * time.Second,
		WriteTimeout: 30 * time.Second,
		IdleTimeout:  60 * time.Second,
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

	return protocol.AgentInfo{
		ID:           s.id,
		Name:         s.cfg.Name,
		Platform:     s.cfg.Platform,
		Version:      s.cfg.Version,
		SteamRunning: false, // TODO: Implement Steam status check
	}
}
