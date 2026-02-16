package discovery

import (
	"net"
	"testing"
	"time"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

func TestConstants(t *testing.T) {
	if ServiceName != "_capydeploy._tcp" {
		t.Errorf("ServiceName = %q, want %q", ServiceName, "_capydeploy._tcp")
	}
	if DefaultPort != 8765 {
		t.Errorf("DefaultPort = %d, want %d", DefaultPort, 8765)
	}
	if DefaultTTL != 120 {
		t.Errorf("DefaultTTL = %d, want %d", DefaultTTL, 120)
	}
}

func TestDiscoveredAgent_Address(t *testing.T) {
	tests := []struct {
		name  string
		agent DiscoveredAgent
		want  string
	}{
		{
			name: "with IP",
			agent: DiscoveredAgent{
				IPs:  []net.IP{net.ParseIP("192.168.1.100")},
				Host: "agent.local",
				Port: 8765,
			},
			want: "192.168.1.100:8765",
		},
		{
			name: "no IP uses host",
			agent: DiscoveredAgent{
				IPs:  nil,
				Host: "agent.local",
				Port: 8765,
			},
			want: "agent.local:8765",
		},
		{
			name: "multiple IPs uses first",
			agent: DiscoveredAgent{
				IPs: []net.IP{
					net.ParseIP("192.168.1.100"),
					net.ParseIP("10.0.0.50"),
				},
				Port: 8765,
			},
			want: "192.168.1.100:8765",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := tt.agent.Address(); got != tt.want {
				t.Errorf("Address() = %q, want %q", got, tt.want)
			}
		})
	}
}

func TestDiscoveredAgent_WebSocketAddress(t *testing.T) {
	agent := DiscoveredAgent{
		IPs:  []net.IP{net.ParseIP("192.168.1.100")},
		Port: 8765,
	}

	want := "ws://192.168.1.100:8765/ws"
	if got := agent.WebSocketAddress(); got != want {
		t.Errorf("WebSocketAddress() = %q, want %q", got, want)
	}
}

func TestDiscoveredAgent_IsStale(t *testing.T) {
	now := time.Now()

	tests := []struct {
		name     string
		lastSeen time.Time
		timeout  time.Duration
		want     bool
	}{
		{
			name:     "not stale",
			lastSeen: now,
			timeout:  1 * time.Minute,
			want:     false,
		},
		{
			name:     "stale",
			lastSeen: now.Add(-2 * time.Minute),
			timeout:  1 * time.Minute,
			want:     true,
		},
		{
			name:     "just before timeout",
			lastSeen: now.Add(-59 * time.Second),
			timeout:  1 * time.Minute,
			want:     false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			agent := DiscoveredAgent{LastSeen: tt.lastSeen}
			if got := agent.IsStale(tt.timeout); got != tt.want {
				t.Errorf("IsStale() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestServiceInfo_ToAgentInfo(t *testing.T) {
	info := ServiceInfo{
		ID:       "agent-123",
		Name:     "Test Agent",
		Platform: "steamdeck",
		Version:  "1.0.0",
		Port:     8765,
	}

	agentInfo := info.ToAgentInfo()

	if agentInfo.ID != info.ID {
		t.Errorf("ID = %q, want %q", agentInfo.ID, info.ID)
	}
	if agentInfo.Name != info.Name {
		t.Errorf("Name = %q, want %q", agentInfo.Name, info.Name)
	}
	if agentInfo.Platform != info.Platform {
		t.Errorf("Platform = %q, want %q", agentInfo.Platform, info.Platform)
	}
	if agentInfo.Version != info.Version {
		t.Errorf("Version = %q, want %q", agentInfo.Version, info.Version)
	}
}

func TestEventType_String(t *testing.T) {
	tests := []struct {
		eventType EventType
		want      string
	}{
		{EventDiscovered, "discovered"},
		{EventUpdated, "updated"},
		{EventLost, "lost"},
		{EventType(99), "unknown"},
	}

	for _, tt := range tests {
		t.Run(tt.want, func(t *testing.T) {
			if got := tt.eventType.String(); got != tt.want {
				t.Errorf("String() = %q, want %q", got, tt.want)
			}
		})
	}
}

func TestDiscoveryEvent_Fields(t *testing.T) {
	agent := &DiscoveredAgent{
		Info: protocol.AgentInfo{ID: "test"},
	}
	event := DiscoveryEvent{
		Type:  EventDiscovered,
		Agent: agent,
	}

	if event.Type != EventDiscovered {
		t.Errorf("Type = %v, want %v", event.Type, EventDiscovered)
	}
	if event.Agent != agent {
		t.Error("Agent should match")
	}
}

func TestItoa(t *testing.T) {
	tests := []struct {
		input int
		want  string
	}{
		{0, "0"},
		{1, "1"},
		{123, "123"},
		{8765, "8765"},
		{-1, "-1"},
		{-123, "-123"},
	}

	for _, tt := range tests {
		if got := itoa(tt.input); got != tt.want {
			t.Errorf("itoa(%d) = %q, want %q", tt.input, got, tt.want)
		}
	}
}

func TestServiceInfo_Fields(t *testing.T) {
	ips := []net.IP{net.ParseIP("192.168.1.100")}
	info := ServiceInfo{
		ID:       "agent-1",
		Name:     "My Agent",
		Platform: "linux",
		Version:  "0.1.0",
		Port:     8765,
		IPs:      ips,
	}

	if info.ID != "agent-1" {
		t.Errorf("ID = %q, want %q", info.ID, "agent-1")
	}
	if info.Name != "My Agent" {
		t.Errorf("Name = %q, want %q", info.Name, "My Agent")
	}
	if info.Port != 8765 {
		t.Errorf("Port = %d, want %d", info.Port, 8765)
	}
	if len(info.IPs) != 1 {
		t.Errorf("IPs length = %d, want 1", len(info.IPs))
	}
}

func TestEventType_Constants(t *testing.T) {
	// Verify distinct values
	types := map[EventType]bool{
		EventDiscovered: true,
		EventUpdated:    true,
		EventLost:       true,
	}

	if len(types) != 3 {
		t.Error("EventType constants should have distinct values")
	}
}
