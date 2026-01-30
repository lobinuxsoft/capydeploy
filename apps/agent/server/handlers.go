package server

import (
	"encoding/json"
	"log"
	"net/http"
	"strconv"

	"github.com/lobinuxsoft/capydeploy/apps/agent/shortcuts"
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/steam"
)

// registerHandlers sets up all HTTP endpoints.
func (s *Server) registerHandlers(mux *http.ServeMux) {
	// Health and info
	mux.HandleFunc("GET /health", s.handleHealth)
	mux.HandleFunc("GET /info", s.handleInfo)

	// Steam
	mux.HandleFunc("GET /steam/users", s.handleSteamUsers)

	// Shortcuts
	mux.HandleFunc("GET /shortcuts/{userID}", s.handleListShortcuts)
	mux.HandleFunc("POST /shortcuts/{userID}", s.handleCreateShortcut)
	mux.HandleFunc("DELETE /shortcuts/{userID}/{appID}", s.handleDeleteShortcut)
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
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(ShortcutsResponse{AppID: appID})
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

	if err := mgr.Delete(userID, appID, ""); err != nil {
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(ShortcutsResponse{Error: err.Error()})
		return
	}

	log.Printf("Deleted shortcut with AppID %d for user %s", appID, userID)
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]string{"status": "deleted"})
}
