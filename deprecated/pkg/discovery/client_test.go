package discovery

import (
	"net"
	"testing"
	"time"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

func TestNewClient(t *testing.T) {
	client := NewClient()

	if client == nil {
		t.Fatal("NewClient() returned nil")
	}
	if client.agents == nil {
		t.Error("agents map should not be nil")
	}
	if client.eventsCh == nil {
		t.Error("eventsCh should not be nil")
	}
}

func TestClient_SetTimeout(t *testing.T) {
	client := NewClient()

	client.SetTimeout(5 * time.Minute)

	if client.timeout != 5*time.Minute {
		t.Errorf("timeout = %v, want 5m", client.timeout)
	}
}

func TestClient_Events(t *testing.T) {
	client := NewClient()

	ch := client.Events()
	if ch == nil {
		t.Error("Events() should not return nil")
	}
}

func TestClient_GetAgents_Empty(t *testing.T) {
	client := NewClient()

	agents := client.GetAgents()
	if agents == nil {
		t.Error("GetAgents() should not return nil")
	}
	if len(agents) != 0 {
		t.Errorf("GetAgents() length = %d, want 0", len(agents))
	}
}

func TestClient_GetAgent_NotFound(t *testing.T) {
	client := NewClient()

	agent := client.GetAgent("nonexistent")
	if agent != nil {
		t.Error("GetAgent() should return nil for non-existent agent")
	}
}

func TestClient_AddAndGetAgent(t *testing.T) {
	client := NewClient()

	// Manually add an agent (simulating discovery)
	agent := &DiscoveredAgent{
		Info: protocol.AgentInfo{
			ID:   "test-agent",
			Name: "Test",
		},
		Port:     8765,
		LastSeen: time.Now(),
	}

	client.mu.Lock()
	client.agents["test-agent"] = agent
	client.mu.Unlock()

	// Retrieve it
	got := client.GetAgent("test-agent")
	if got == nil {
		t.Fatal("GetAgent() returned nil")
	}
	if got.Info.ID != "test-agent" {
		t.Errorf("Agent ID = %q, want %q", got.Info.ID, "test-agent")
	}
}

func TestClient_GetAgents_Multiple(t *testing.T) {
	client := NewClient()

	// Add multiple agents
	for i := 0; i < 3; i++ {
		agent := &DiscoveredAgent{
			Info:     protocol.AgentInfo{ID: string(rune('A' + i))},
			LastSeen: time.Now(),
		}
		client.mu.Lock()
		client.agents[agent.Info.ID] = agent
		client.mu.Unlock()
	}

	agents := client.GetAgents()
	if len(agents) != 3 {
		t.Errorf("GetAgents() length = %d, want 3", len(agents))
	}
}

func TestClient_RemoveAgent(t *testing.T) {
	client := NewClient()

	// Add an agent
	agent := &DiscoveredAgent{
		Info:     protocol.AgentInfo{ID: "test"},
		LastSeen: time.Now(),
	}
	client.mu.Lock()
	client.agents["test"] = agent
	client.mu.Unlock()

	// Remove it
	client.RemoveAgent("test")

	if got := client.GetAgent("test"); got != nil {
		t.Error("Agent should be removed")
	}
}

func TestClient_RemoveAgent_EmitsEvent(t *testing.T) {
	client := NewClient()

	// Add an agent
	agent := &DiscoveredAgent{
		Info:     protocol.AgentInfo{ID: "test"},
		LastSeen: time.Now(),
	}
	client.mu.Lock()
	client.agents["test"] = agent
	client.mu.Unlock()

	// Start listening for events
	done := make(chan bool)
	go func() {
		select {
		case event := <-client.Events():
			if event.Type != EventLost {
				t.Errorf("Event type = %v, want %v", event.Type, EventLost)
			}
			done <- true
		case <-time.After(100 * time.Millisecond):
			t.Error("Expected event not received")
			done <- false
		}
	}()

	// Remove the agent
	client.RemoveAgent("test")

	<-done
}

func TestClient_RemoveAgent_NotFound(t *testing.T) {
	client := NewClient()

	// Should not panic
	client.RemoveAgent("nonexistent")
}

func TestClient_Clear(t *testing.T) {
	client := NewClient()

	// Add some agents
	for i := 0; i < 3; i++ {
		client.mu.Lock()
		client.agents[string(rune('A'+i))] = &DiscoveredAgent{}
		client.mu.Unlock()
	}

	client.Clear()

	if len(client.GetAgents()) != 0 {
		t.Error("Clear() should remove all agents")
	}
}

func TestClient_Close(t *testing.T) {
	client := NewClient()

	client.Close()

	// Channel should be closed
	_, ok := <-client.Events()
	if ok {
		t.Error("Events channel should be closed")
	}
}

func TestClient_PruneStaleAgents(t *testing.T) {
	client := NewClient()
	client.timeout = 100 * time.Millisecond

	// Add a stale agent
	staleAgent := &DiscoveredAgent{
		Info:     protocol.AgentInfo{ID: "stale"},
		LastSeen: time.Now().Add(-1 * time.Second),
	}
	// Add a fresh agent
	freshAgent := &DiscoveredAgent{
		Info:     protocol.AgentInfo{ID: "fresh"},
		LastSeen: time.Now(),
	}

	client.mu.Lock()
	client.agents["stale"] = staleAgent
	client.agents["fresh"] = freshAgent
	client.mu.Unlock()

	client.pruneStaleAgents()

	if client.GetAgent("stale") != nil {
		t.Error("Stale agent should be pruned")
	}
	if client.GetAgent("fresh") == nil {
		t.Error("Fresh agent should not be pruned")
	}
}

func TestClient_EmitEvent_NonBlocking(t *testing.T) {
	client := NewClient()

	// Fill the channel
	for i := 0; i < 20; i++ {
		client.emitEvent(DiscoveryEvent{Type: EventUpdated})
	}

	// Should not block
	done := make(chan bool)
	go func() {
		client.emitEvent(DiscoveryEvent{Type: EventLost})
		done <- true
	}()

	select {
	case <-done:
		// Good, didn't block
	case <-time.After(100 * time.Millisecond):
		t.Error("emitEvent should not block when channel is full")
	}
}

func TestClient_ProcessEntry(t *testing.T) {
	client := NewClient()

	// Create a mock entry by adding an agent directly
	// (since we can't easily create mdns.ServiceEntry)
	agent := &DiscoveredAgent{
		Info: protocol.AgentInfo{
			ID:       "test-id",
			Name:     "Test Agent",
			Platform: "linux",
			Version:  "1.0.0",
		},
		Host:         "test.local",
		Port:         8765,
		IPs:          []net.IP{net.ParseIP("192.168.1.100")},
		DiscoveredAt: time.Now(),
		LastSeen:     time.Now(),
	}

	client.mu.Lock()
	client.agents[agent.Info.ID] = agent
	client.mu.Unlock()

	// Verify the agent is stored correctly
	got := client.GetAgent("test-id")
	if got == nil {
		t.Fatal("Agent should be stored")
	}
	if got.Info.Name != "Test Agent" {
		t.Errorf("Name = %q, want %q", got.Info.Name, "Test Agent")
	}
}

func TestClient_ConcurrentAccess(t *testing.T) {
	client := NewClient()

	done := make(chan bool)

	// Concurrent writes
	for i := 0; i < 10; i++ {
		go func(idx int) {
			for j := 0; j < 50; j++ {
				id := string(rune('A' + idx))
				agent := &DiscoveredAgent{
					Info:     protocol.AgentInfo{ID: id},
					LastSeen: time.Now(),
				}
				client.mu.Lock()
				client.agents[id] = agent
				client.mu.Unlock()
			}
			done <- true
		}(i)
	}

	// Concurrent reads
	for i := 0; i < 10; i++ {
		go func() {
			for j := 0; j < 50; j++ {
				_ = client.GetAgents()
				_ = client.GetAgent("A")
			}
			done <- true
		}()
	}

	// Wait for all goroutines
	for i := 0; i < 20; i++ {
		select {
		case <-done:
		case <-time.After(5 * time.Second):
			t.Fatal("Timeout waiting for concurrent operations")
		}
	}
}
