// Package auth provides authentication and pairing for Hub connections.
package auth

import (
	"crypto/rand"
	"encoding/base64"
	"errors"
	"fmt"
	"sync"
	"time"
)

const (
	// CodeLength is the number of digits in a pairing code.
	CodeLength = 6
	// CodeExpiry is how long a pairing code is valid.
	CodeExpiry = 60 * time.Second
	// TokenLength is the number of random bytes for tokens.
	TokenLength = 32
	// MaxFailedAttempts before rate limiting.
	MaxFailedAttempts = 3
	// RateLimitDuration after max failed attempts.
	RateLimitDuration = 5 * time.Minute
)

// Errors returned by auth operations.
var (
	ErrCodeExpired      = errors.New("pairing code expired")
	ErrCodeInvalid      = errors.New("invalid pairing code")
	ErrRateLimited      = errors.New("too many failed attempts, try again later")
	ErrHubNotFound      = errors.New("hub not found")
	ErrTokenInvalid     = errors.New("invalid token")
	ErrNoPendingPairing = errors.New("no pending pairing")
)

// AuthorizedHub represents a Hub that has been paired with this Agent.
type AuthorizedHub struct {
	ID       string    `json:"id"`
	Name     string    `json:"name"`
	Token    string    `json:"token"`
	PairedAt time.Time `json:"pairedAt"`
	LastSeen time.Time `json:"lastSeen"`
}

// PairingSession holds the state of an active pairing attempt.
type PairingSession struct {
	Code      string
	HubID     string
	HubName   string
	ExpiresAt time.Time
}

// Storage defines the interface for persisting authorized hubs.
type Storage interface {
	GetAuthorizedHubs() []AuthorizedHub
	AddAuthorizedHub(hub AuthorizedHub) error
	RemoveAuthorizedHub(hubID string) error
	UpdateHubLastSeen(hubID string, lastSeen time.Time) error
	Save() error
}

// Manager handles authentication and pairing operations.
type Manager struct {
	mu             sync.RWMutex
	storage        Storage
	pendingPairing *PairingSession
	failedAttempts int
	rateLimitUntil time.Time
	onPairingCode  func(code string, expiresIn time.Duration)
}

// NewManager creates a new auth Manager.
func NewManager(storage Storage) *Manager {
	return &Manager{
		storage: storage,
	}
}

// SetPairingCodeCallback sets the callback for when a pairing code is generated.
func (m *Manager) SetPairingCodeCallback(cb func(code string, expiresIn time.Duration)) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.onPairingCode = cb
}

// GenerateCode creates a new 6-digit pairing code for a Hub.
func (m *Manager) GenerateCode(hubID, hubName string) (string, error) {
	m.mu.Lock()
	defer m.mu.Unlock()

	// Check rate limiting
	if time.Now().Before(m.rateLimitUntil) {
		return "", ErrRateLimited
	}

	// Generate 6-digit numeric code
	code, err := generateNumericCode(CodeLength)
	if err != nil {
		return "", fmt.Errorf("failed to generate code: %w", err)
	}

	m.pendingPairing = &PairingSession{
		Code:      code,
		HubID:     hubID,
		HubName:   hubName,
		ExpiresAt: time.Now().Add(CodeExpiry),
	}

	// Notify callback
	if m.onPairingCode != nil {
		m.onPairingCode(code, CodeExpiry)
	}

	return code, nil
}

// ValidateCode checks if the provided code matches the pending pairing.
// Returns the generated token on success.
func (m *Manager) ValidateCode(hubID, hubName, code string) (string, error) {
	m.mu.Lock()
	defer m.mu.Unlock()

	// Check rate limiting
	if time.Now().Before(m.rateLimitUntil) {
		return "", ErrRateLimited
	}

	if m.pendingPairing == nil {
		return "", ErrNoPendingPairing
	}

	// Check expiration
	if time.Now().After(m.pendingPairing.ExpiresAt) {
		m.pendingPairing = nil
		return "", ErrCodeExpired
	}

	// Check Hub ID matches
	if m.pendingPairing.HubID != hubID {
		m.failedAttempts++
		if m.failedAttempts >= MaxFailedAttempts {
			m.rateLimitUntil = time.Now().Add(RateLimitDuration)
			m.failedAttempts = 0
		}
		return "", ErrCodeInvalid
	}

	// Validate code
	if m.pendingPairing.Code != code {
		m.failedAttempts++
		if m.failedAttempts >= MaxFailedAttempts {
			m.rateLimitUntil = time.Now().Add(RateLimitDuration)
			m.failedAttempts = 0
		}
		return "", ErrCodeInvalid
	}

	// Generate token
	token, err := generateToken(TokenLength)
	if err != nil {
		return "", fmt.Errorf("failed to generate token: %w", err)
	}

	// Add to authorized hubs
	hub := AuthorizedHub{
		ID:       hubID,
		Name:     hubName,
		Token:    token,
		PairedAt: time.Now(),
		LastSeen: time.Now(),
	}

	if err := m.storage.AddAuthorizedHub(hub); err != nil {
		return "", fmt.Errorf("failed to save authorized hub: %w", err)
	}

	// Clear pending pairing
	m.pendingPairing = nil
	m.failedAttempts = 0

	return token, nil
}

// ValidateToken checks if a Hub's token is valid.
func (m *Manager) ValidateToken(hubID, token string) bool {
	m.mu.Lock()
	defer m.mu.Unlock()

	hubs := m.storage.GetAuthorizedHubs()
	for _, hub := range hubs {
		if hub.ID == hubID && hub.Token == token {
			// Update last seen
			_ = m.storage.UpdateHubLastSeen(hubID, time.Now())
			return true
		}
	}
	return false
}

// IsHubAuthorized checks if a Hub is in the authorized list (by ID only).
func (m *Manager) IsHubAuthorized(hubID string) bool {
	m.mu.RLock()
	defer m.mu.RUnlock()

	hubs := m.storage.GetAuthorizedHubs()
	for _, hub := range hubs {
		if hub.ID == hubID {
			return true
		}
	}
	return false
}

// GetAuthorizedHubs returns the list of authorized Hubs.
func (m *Manager) GetAuthorizedHubs() []AuthorizedHub {
	m.mu.RLock()
	defer m.mu.RUnlock()
	return m.storage.GetAuthorizedHubs()
}

// RevokeHub removes a Hub from the authorized list.
func (m *Manager) RevokeHub(hubID string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	if err := m.storage.RemoveAuthorizedHub(hubID); err != nil {
		return fmt.Errorf("failed to revoke hub: %w", err)
	}
	return nil
}

// GetPendingPairing returns the current pending pairing session, if any.
func (m *Manager) GetPendingPairing() *PairingSession {
	m.mu.RLock()
	defer m.mu.RUnlock()

	if m.pendingPairing == nil {
		return nil
	}

	// Check if expired
	if time.Now().After(m.pendingPairing.ExpiresAt) {
		return nil
	}

	// Return a copy
	return &PairingSession{
		Code:      m.pendingPairing.Code,
		HubID:     m.pendingPairing.HubID,
		HubName:   m.pendingPairing.HubName,
		ExpiresAt: m.pendingPairing.ExpiresAt,
	}
}

// CancelPendingPairing cancels any pending pairing session.
func (m *Manager) CancelPendingPairing() {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.pendingPairing = nil
}

// generateNumericCode generates a random n-digit numeric code.
func generateNumericCode(length int) (string, error) {
	const digits = "0123456789"
	code := make([]byte, length)

	for i := 0; i < length; i++ {
		b := make([]byte, 1)
		if _, err := rand.Read(b); err != nil {
			return "", err
		}
		code[i] = digits[int(b[0])%len(digits)]
	}

	return string(code), nil
}

// generateToken generates a random base64-encoded token.
func generateToken(length int) (string, error) {
	bytes := make([]byte, length)
	if _, err := rand.Read(bytes); err != nil {
		return "", err
	}
	return base64.URLEncoding.EncodeToString(bytes), nil
}
