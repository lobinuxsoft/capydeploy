package auth

import (
	"sync"
	"testing"
	"time"
)

// mockStorage is an in-memory implementation of the Storage interface for testing.
type mockStorage struct {
	mu   sync.Mutex
	hubs []AuthorizedHub
}

func newMockStorage() *mockStorage {
	return &mockStorage{}
}

func (m *mockStorage) GetAuthorizedHubs() []AuthorizedHub {
	m.mu.Lock()
	defer m.mu.Unlock()
	result := make([]AuthorizedHub, len(m.hubs))
	copy(result, m.hubs)
	return result
}

func (m *mockStorage) AddAuthorizedHub(hub AuthorizedHub) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	for i, h := range m.hubs {
		if h.ID == hub.ID {
			m.hubs[i] = hub
			return nil
		}
	}
	m.hubs = append(m.hubs, hub)
	return nil
}

func (m *mockStorage) RemoveAuthorizedHub(hubID string) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	for i, h := range m.hubs {
		if h.ID == hubID {
			m.hubs = append(m.hubs[:i], m.hubs[i+1:]...)
			return nil
		}
	}
	return nil
}

func (m *mockStorage) UpdateHubLastSeen(hubID string, lastSeen time.Time) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	for i, h := range m.hubs {
		if h.ID == hubID {
			m.hubs[i].LastSeen = lastSeen
			return nil
		}
	}
	return nil
}

func (m *mockStorage) Save() error {
	return nil
}

// --- Tests ---

func TestGenerateCode(t *testing.T) {
	mgr := NewManager(newMockStorage())

	code, err := mgr.GenerateCode("hub-1", "Test Hub", "linux")
	if err != nil {
		t.Fatalf("GenerateCode() error = %v", err)
	}

	if len(code) != CodeLength {
		t.Errorf("GenerateCode() code length = %d, want %d", len(code), CodeLength)
	}

	// Code should be all digits
	for _, c := range code {
		if c < '0' || c > '9' {
			t.Errorf("GenerateCode() code contains non-digit: %c", c)
		}
	}
}

func TestGenerateCode_Callback(t *testing.T) {
	mgr := NewManager(newMockStorage())

	var callbackCode string
	var callbackExpiry time.Duration
	mgr.SetPairingCodeCallback(func(code string, expiresIn time.Duration) {
		callbackCode = code
		callbackExpiry = expiresIn
	})

	code, err := mgr.GenerateCode("hub-1", "Test Hub", "linux")
	if err != nil {
		t.Fatalf("GenerateCode() error = %v", err)
	}

	if callbackCode != code {
		t.Errorf("callback code = %q, want %q", callbackCode, code)
	}
	if callbackExpiry != CodeExpiry {
		t.Errorf("callback expiry = %v, want %v", callbackExpiry, CodeExpiry)
	}
}

func TestValidateCode_Success(t *testing.T) {
	store := newMockStorage()
	mgr := NewManager(store)

	code, err := mgr.GenerateCode("hub-1", "Test Hub", "linux")
	if err != nil {
		t.Fatalf("GenerateCode() error = %v", err)
	}

	token, err := mgr.ValidateCode("hub-1", "Test Hub", code)
	if err != nil {
		t.Fatalf("ValidateCode() error = %v", err)
	}

	if token == "" {
		t.Error("ValidateCode() returned empty token")
	}

	// Hub should be in authorized list
	hubs := store.GetAuthorizedHubs()
	if len(hubs) != 1 {
		t.Fatalf("expected 1 authorized hub, got %d", len(hubs))
	}
	if hubs[0].ID != "hub-1" {
		t.Errorf("authorized hub ID = %q, want %q", hubs[0].ID, "hub-1")
	}
	if hubs[0].Platform != "linux" {
		t.Errorf("authorized hub platform = %q, want %q", hubs[0].Platform, "linux")
	}
}

func TestValidateCode_WrongCode(t *testing.T) {
	mgr := NewManager(newMockStorage())

	_, err := mgr.GenerateCode("hub-1", "Test Hub", "linux")
	if err != nil {
		t.Fatalf("GenerateCode() error = %v", err)
	}

	_, err = mgr.ValidateCode("hub-1", "Test Hub", "000000")
	if err != ErrCodeInvalid {
		t.Errorf("ValidateCode() error = %v, want %v", err, ErrCodeInvalid)
	}
}

func TestValidateCode_WrongHubID(t *testing.T) {
	mgr := NewManager(newMockStorage())

	code, err := mgr.GenerateCode("hub-1", "Test Hub", "linux")
	if err != nil {
		t.Fatalf("GenerateCode() error = %v", err)
	}

	_, err = mgr.ValidateCode("hub-wrong", "Test Hub", code)
	if err != ErrCodeInvalid {
		t.Errorf("ValidateCode() error = %v, want %v", err, ErrCodeInvalid)
	}
}

func TestValidateCode_Expired(t *testing.T) {
	mgr := NewManager(newMockStorage())

	code, err := mgr.GenerateCode("hub-1", "Test Hub", "linux")
	if err != nil {
		t.Fatalf("GenerateCode() error = %v", err)
	}

	// Force expiration by manipulating the pending pairing
	mgr.mu.Lock()
	mgr.pendingPairing.ExpiresAt = time.Now().Add(-1 * time.Second)
	mgr.mu.Unlock()

	_, err = mgr.ValidateCode("hub-1", "Test Hub", code)
	if err != ErrCodeExpired {
		t.Errorf("ValidateCode() error = %v, want %v", err, ErrCodeExpired)
	}
}

func TestValidateCode_RateLimit(t *testing.T) {
	mgr := NewManager(newMockStorage())

	code, err := mgr.GenerateCode("hub-1", "Test Hub", "linux")
	if err != nil {
		t.Fatalf("GenerateCode() error = %v", err)
	}

	// Fail MaxFailedAttempts times
	for i := 0; i < MaxFailedAttempts; i++ {
		_, _ = mgr.ValidateCode("hub-1", "Test Hub", "wrong!")
	}

	// Next attempt should be rate limited even with correct code
	_, err = mgr.ValidateCode("hub-1", "Test Hub", code)
	if err != ErrRateLimited {
		t.Errorf("ValidateCode() after rate limit error = %v, want %v", err, ErrRateLimited)
	}
}

func TestValidateCode_NoPending(t *testing.T) {
	mgr := NewManager(newMockStorage())

	_, err := mgr.ValidateCode("hub-1", "Test Hub", "123456")
	if err != ErrNoPendingPairing {
		t.Errorf("ValidateCode() error = %v, want %v", err, ErrNoPendingPairing)
	}
}

func TestValidateToken(t *testing.T) {
	store := newMockStorage()
	mgr := NewManager(store)

	code, err := mgr.GenerateCode("hub-1", "Test Hub", "linux")
	if err != nil {
		t.Fatalf("GenerateCode() error = %v", err)
	}

	token, err := mgr.ValidateCode("hub-1", "Test Hub", code)
	if err != nil {
		t.Fatalf("ValidateCode() error = %v", err)
	}

	// Valid token
	if !mgr.ValidateToken("hub-1", token) {
		t.Error("ValidateToken() = false for valid token")
	}

	// Wrong token
	if mgr.ValidateToken("hub-1", "bad-token") {
		t.Error("ValidateToken() = true for invalid token")
	}

	// Wrong hub ID
	if mgr.ValidateToken("hub-wrong", token) {
		t.Error("ValidateToken() = true for wrong hub ID")
	}
}

func TestIsHubAuthorized(t *testing.T) {
	store := newMockStorage()
	mgr := NewManager(store)

	// Not authorized initially
	if mgr.IsHubAuthorized("hub-1") {
		t.Error("IsHubAuthorized() = true before pairing")
	}

	// Pair the hub
	code, _ := mgr.GenerateCode("hub-1", "Test Hub", "linux")
	_, _ = mgr.ValidateCode("hub-1", "Test Hub", code)

	if !mgr.IsHubAuthorized("hub-1") {
		t.Error("IsHubAuthorized() = false after pairing")
	}
}

func TestRevokeHub(t *testing.T) {
	store := newMockStorage()
	mgr := NewManager(store)

	// Pair first
	code, _ := mgr.GenerateCode("hub-1", "Test Hub", "linux")
	_, _ = mgr.ValidateCode("hub-1", "Test Hub", code)

	if !mgr.IsHubAuthorized("hub-1") {
		t.Fatal("hub should be authorized after pairing")
	}

	// Revoke
	if err := mgr.RevokeHub("hub-1"); err != nil {
		t.Fatalf("RevokeHub() error = %v", err)
	}

	if mgr.IsHubAuthorized("hub-1") {
		t.Error("IsHubAuthorized() = true after revoke")
	}
}

func TestGetPendingPairing(t *testing.T) {
	mgr := NewManager(newMockStorage())

	// No pending initially
	if p := mgr.GetPendingPairing(); p != nil {
		t.Error("GetPendingPairing() should be nil initially")
	}

	// Generate code
	code, _ := mgr.GenerateCode("hub-1", "Test Hub", "linux")

	p := mgr.GetPendingPairing()
	if p == nil {
		t.Fatal("GetPendingPairing() = nil after GenerateCode")
	}
	if p.Code != code {
		t.Errorf("pending code = %q, want %q", p.Code, code)
	}
	if p.HubID != "hub-1" {
		t.Errorf("pending HubID = %q, want %q", p.HubID, "hub-1")
	}
}

func TestGetPendingPairing_Expired(t *testing.T) {
	mgr := NewManager(newMockStorage())

	_, _ = mgr.GenerateCode("hub-1", "Test Hub", "linux")

	// Force expiration
	mgr.mu.Lock()
	mgr.pendingPairing.ExpiresAt = time.Now().Add(-1 * time.Second)
	mgr.mu.Unlock()

	if p := mgr.GetPendingPairing(); p != nil {
		t.Error("GetPendingPairing() should be nil after expiration")
	}
}

func TestCancelPendingPairing(t *testing.T) {
	mgr := NewManager(newMockStorage())

	_, _ = mgr.GenerateCode("hub-1", "Test Hub", "linux")

	mgr.CancelPendingPairing()

	if p := mgr.GetPendingPairing(); p != nil {
		t.Error("GetPendingPairing() should be nil after cancel")
	}
}

func TestGetAuthorizedHubs(t *testing.T) {
	store := newMockStorage()
	mgr := NewManager(store)

	// Empty initially
	hubs := mgr.GetAuthorizedHubs()
	if len(hubs) != 0 {
		t.Errorf("GetAuthorizedHubs() len = %d, want 0", len(hubs))
	}

	// Pair two hubs
	code1, _ := mgr.GenerateCode("hub-1", "Hub One", "linux")
	_, _ = mgr.ValidateCode("hub-1", "Hub One", code1)

	code2, _ := mgr.GenerateCode("hub-2", "Hub Two", "windows")
	_, _ = mgr.ValidateCode("hub-2", "Hub Two", code2)

	hubs = mgr.GetAuthorizedHubs()
	if len(hubs) != 2 {
		t.Errorf("GetAuthorizedHubs() len = %d, want 2", len(hubs))
	}
}
