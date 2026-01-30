package server

import (
	"encoding/json"
	"log"
	"net/http"

	"github.com/lobinuxsoft/capydeploy/pkg/steam"
)

// registerHandlers sets up all HTTP endpoints.
func (s *Server) registerHandlers(mux *http.ServeMux) {
	mux.HandleFunc("GET /health", s.handleHealth)
	mux.HandleFunc("GET /info", s.handleInfo)
	mux.HandleFunc("GET /steam/users", s.handleSteamUsers)
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
