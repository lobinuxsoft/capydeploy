// Package auth provides token storage for Hub-Agent authentication.
package auth

import (
	"encoding/json"
	"os"
	"path/filepath"
	"sync"

	"github.com/google/uuid"
)

// TokenStore manages authentication tokens for connected Agents.
type TokenStore struct {
	mu       sync.RWMutex
	tokens   map[string]string // agentID → token
	hubID    string
	filePath string
}

// NewTokenStore creates a new token store.
func NewTokenStore() (*TokenStore, error) {
	configDir, err := os.UserConfigDir()
	if err != nil {
		return nil, err
	}

	dir := filepath.Join(configDir, "capydeploy-hub")
	if err := os.MkdirAll(dir, 0755); err != nil {
		return nil, err
	}

	store := &TokenStore{
		tokens:   make(map[string]string),
		filePath: filepath.Join(dir, "tokens.json"),
	}

	// Load existing tokens
	store.load()

	// Load or generate Hub ID
	store.loadHubID(dir)

	return store, nil
}

// loadHubID loads or generates a stable Hub ID.
func (s *TokenStore) loadHubID(dir string) {
	idPath := filepath.Join(dir, "hub_id")
	data, err := os.ReadFile(idPath)
	if err == nil && len(data) > 0 {
		s.hubID = string(data)
		return
	}

	// Generate new ID
	s.hubID = uuid.New().String()
	_ = os.WriteFile(idPath, []byte(s.hubID), 0600)
}

// load reads tokens from disk.
func (s *TokenStore) load() {
	data, err := os.ReadFile(s.filePath)
	if err != nil {
		return
	}

	var tokens map[string]string
	if err := json.Unmarshal(data, &tokens); err != nil {
		return
	}

	s.tokens = tokens
}

// save writes tokens to disk.
func (s *TokenStore) save() error {
	data, err := json.MarshalIndent(s.tokens, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(s.filePath, data, 0600)
}

// GetHubID returns the unique Hub identifier.
func (s *TokenStore) GetHubID() string {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.hubID
}

// GetToken returns the token for an Agent, or empty string if not found.
func (s *TokenStore) GetToken(agentID string) string {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.tokens[agentID]
}

// SaveToken stores a token for an Agent.
func (s *TokenStore) SaveToken(agentID, token string) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	s.tokens[agentID] = token
	return s.save()
}

// RemoveToken removes the token for an Agent.
func (s *TokenStore) RemoveToken(agentID string) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	delete(s.tokens, agentID)
	return s.save()
}

// HasToken returns true if a token exists for the Agent.
func (s *TokenStore) HasToken(agentID string) bool {
	s.mu.RLock()
	defer s.mu.RUnlock()
	_, ok := s.tokens[agentID]
	return ok
}
