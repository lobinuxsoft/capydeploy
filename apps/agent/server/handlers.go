package server

import (
	"encoding/json"
	"log"
	"net/http"
	"strconv"

	"github.com/lobinuxsoft/capydeploy/apps/agent/artwork"
	"github.com/lobinuxsoft/capydeploy/apps/agent/shortcuts"
	agentSteam "github.com/lobinuxsoft/capydeploy/apps/agent/steam"
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/steam"
)

// registerHandlers sets up all HTTP endpoints.
func (s *Server) registerHandlers(mux *http.ServeMux) {
	// Health and info - always accessible (needed for discovery)
	mux.HandleFunc("GET /health", s.handleHealth)
	mux.HandleFunc("GET /info", s.handleInfo)
	mux.HandleFunc("GET /config", s.handleGetConfig)

	// Protected endpoints - require connections to be accepted
	// Steam
	mux.HandleFunc("GET /steam/users", s.requireConnections(s.handleSteamUsers))

	// Shortcuts
	mux.HandleFunc("GET /shortcuts/{userID}", s.requireConnections(s.handleListShortcuts))
	mux.HandleFunc("POST /shortcuts/{userID}", s.requireConnections(s.handleCreateShortcut))
	mux.HandleFunc("DELETE /shortcuts/{userID}/{appID}", s.requireConnections(s.handleDeleteShortcut))
	mux.HandleFunc("POST /shortcuts/{userID}/{appID}/artwork", s.requireConnections(s.handleApplyArtwork))

	// Steam control
	mux.HandleFunc("POST /steam/restart", s.requireConnections(s.handleSteamRestart))

	// Uploads
	mux.HandleFunc("POST /uploads", s.requireConnections(s.handleInitUpload))
	mux.HandleFunc("POST /uploads/{id}/chunks", s.requireConnections(s.handleUploadChunk))
	mux.HandleFunc("POST /uploads/{id}/complete", s.requireConnections(s.handleCompleteUpload))
	mux.HandleFunc("DELETE /uploads/{id}", s.requireConnections(s.handleCancelUpload))
	mux.HandleFunc("GET /uploads/{id}", s.requireConnections(s.handleGetUploadStatus))
}

// requireConnections wraps a handler to check if connections are accepted.
func (s *Server) requireConnections(next http.HandlerFunc) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		if s.cfg.AcceptConnections != nil && !s.cfg.AcceptConnections() {
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusServiceUnavailable)
			json.NewEncoder(w).Encode(map[string]string{
				"error": "connections are currently blocked",
			})
			log.Printf("Blocked request from %s: connections disabled", r.RemoteAddr)
			return
		}
		next(w, r)
	}
}

// handleHealth returns a simple health check response.
func (s *Server) handleHealth(w http.ResponseWriter, r *http.Request) {
	if s.cfg.Verbose {
		log.Printf("Health check from %s", r.RemoteAddr)
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]string{
		"status": "ok",
	})
}

// handleInfo returns the agent information.
func (s *Server) handleInfo(w http.ResponseWriter, r *http.Request) {
	if s.cfg.Verbose {
		log.Printf("Info request from %s", r.RemoteAddr)
	}

	info := s.GetInfo()

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(info)
}

// AgentConfigResponse is the response for GET /config.
type AgentConfigResponse struct {
	InstallPath string `json:"installPath"`
}

// handleGetConfig returns the agent configuration.
func (s *Server) handleGetConfig(w http.ResponseWriter, r *http.Request) {
	if s.cfg.Verbose {
		log.Printf("Config request from %s", r.RemoteAddr)
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(AgentConfigResponse{
		InstallPath: s.GetInstallPath(),
	})
}

// SteamUsersResponse is the response for GET /steam/users.
type SteamUsersResponse struct {
	Users []steam.User `json:"users"`
	Error string       `json:"error,omitempty"`
}

// handleSteamUsers returns the list of Steam users.
func (s *Server) handleSteamUsers(w http.ResponseWriter, r *http.Request) {
	if s.cfg.Verbose {
		log.Printf("Steam users request from %s", r.RemoteAddr)
	}

	users, err := steam.GetUsers()

	w.Header().Set("Content-Type", "application/json")

	if err != nil {
		log.Printf("Error getting Steam users: %v", err)
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(SteamUsersResponse{
			Users: []steam.User{},
			Error: err.Error(),
		})
		return
	}

	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(SteamUsersResponse{
		Users: users,
	})
}

// ShortcutsResponse is the response for shortcut operations.
type ShortcutsResponse struct {
	Shortcuts []protocol.ShortcutInfo `json:"shortcuts,omitempty"`
	AppID     uint32                  `json:"appId,omitempty"`
	Error     string                  `json:"error,omitempty"`
}

// handleListShortcuts returns shortcuts for a user.
func (s *Server) handleListShortcuts(w http.ResponseWriter, r *http.Request) {
	userID := r.PathValue("userID")
	if s.cfg.Verbose {
		log.Printf("List shortcuts request for user %s from %s", userID, r.RemoteAddr)
	}

	w.Header().Set("Content-Type", "application/json")

	mgr, err := shortcuts.NewManager()
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(ShortcutsResponse{Error: err.Error()})
		return
	}

	list, err := mgr.List(userID)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(ShortcutsResponse{Error: err.Error()})
		return
	}

	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(ShortcutsResponse{Shortcuts: list})
}

// handleCreateShortcut creates a new shortcut.
func (s *Server) handleCreateShortcut(w http.ResponseWriter, r *http.Request) {
	userID := r.PathValue("userID")
	if s.cfg.Verbose {
		log.Printf("Create shortcut request for user %s from %s", userID, r.RemoteAddr)
	}

	w.Header().Set("Content-Type", "application/json")

	var cfg protocol.ShortcutConfig
	if err := json.NewDecoder(r.Body).Decode(&cfg); err != nil {
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(ShortcutsResponse{Error: "invalid request body"})
		return
	}

	mgr, err := shortcuts.NewManager()
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(ShortcutsResponse{Error: err.Error()})
		return
	}

	appID, err := mgr.Create(userID, cfg)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(ShortcutsResponse{Error: err.Error()})
		return
	}

	log.Printf("Created shortcut '%s' with AppID %d for user %s", cfg.Name, appID, userID)
	s.NotifyShortcutChange()

	// Restart Steam if requested
	restartSteam := r.URL.Query().Get("restart") == "true"
	var steamRestarted bool
	if restartSteam {
		controller := agentSteam.NewController()
		result := controller.Restart()
		steamRestarted = result.Success
		log.Printf("Steam restart after create: %v", result.Message)
	}

	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(map[string]interface{}{
		"appId":          appID,
		"steamRestarted": steamRestarted,
	})
}

// handleDeleteShortcut deletes a shortcut.
func (s *Server) handleDeleteShortcut(w http.ResponseWriter, r *http.Request) {
	userID := r.PathValue("userID")
	appIDStr := r.PathValue("appID")
	if s.cfg.Verbose {
		log.Printf("Delete shortcut request for user %s, appID %s from %s", userID, appIDStr, r.RemoteAddr)
	}

	w.Header().Set("Content-Type", "application/json")

	appIDInt, err := strconv.ParseUint(appIDStr, 10, 32)
	if err != nil {
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(ShortcutsResponse{Error: "invalid appID"})
		return
	}
	appID := uint32(appIDInt)

	mgr, err := shortcuts.NewManager()
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(ShortcutsResponse{Error: err.Error()})
		return
	}

	// Get shortcut info before deleting (for notification)
	list, _ := mgr.List(userID)
	var gameName string
	for _, sc := range list {
		if sc.AppID == appID {
			gameName = sc.Name
			break
		}
	}

	// Notify UI about delete start
	s.NotifyOperation("delete", "start", gameName, 0, "Eliminando...")

	if err := mgr.Delete(userID, appID, ""); err != nil {
		s.NotifyOperation("delete", "error", gameName, 0, err.Error())
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(ShortcutsResponse{Error: err.Error()})
		return
	}

	log.Printf("Deleted shortcut with AppID %d for user %s", appID, userID)
	s.NotifyShortcutChange()

	// Notify UI about delete complete
	s.NotifyOperation("delete", "complete", gameName, 100, "Eliminado")

	// Restart Steam if requested
	restartSteam := r.URL.Query().Get("restart") == "true"
	var steamRestarted bool
	if restartSteam {
		controller := agentSteam.NewController()
		result := controller.Restart()
		steamRestarted = result.Success
		log.Printf("Steam restart after delete: %v", result.Message)
	}

	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]interface{}{
		"status":         "deleted",
		"steamRestarted": steamRestarted,
	})
}

// handleApplyArtwork applies artwork to a shortcut.
func (s *Server) handleApplyArtwork(w http.ResponseWriter, r *http.Request) {
	userID := r.PathValue("userID")
	appIDStr := r.PathValue("appID")
	if s.cfg.Verbose {
		log.Printf("Apply artwork request for user %s, appID %s from %s", userID, appIDStr, r.RemoteAddr)
	}

	w.Header().Set("Content-Type", "application/json")

	appIDInt, err := strconv.ParseUint(appIDStr, 10, 32)
	if err != nil {
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(map[string]string{"error": "invalid appID"})
		return
	}
	appID := uint32(appIDInt)

	var cfg protocol.ArtworkConfig
	if err := json.NewDecoder(r.Body).Decode(&cfg); err != nil {
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(map[string]string{"error": "invalid request body"})
		return
	}

	result, err := artwork.Apply(userID, appID, &cfg)
	if err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(map[string]string{"error": err.Error()})
		return
	}

	log.Printf("Applied artwork for AppID %d: %v", appID, result.Applied)
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(result)
}

// handleSteamRestart restarts Steam.
func (s *Server) handleSteamRestart(w http.ResponseWriter, r *http.Request) {
	if s.cfg.Verbose {
		log.Printf("Steam restart request from %s", r.RemoteAddr)
	}

	w.Header().Set("Content-Type", "application/json")

	controller := agentSteam.NewController()
	result := controller.Restart()

	log.Printf("Steam restart: success=%v, message=%s", result.Success, result.Message)
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(result)
}
