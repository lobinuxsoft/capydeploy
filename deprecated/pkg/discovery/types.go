// Package discovery provides mDNS-based agent discovery for the Hub-Agent architecture.
package discovery

import (
	"net"
	"time"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

// ServiceName is the mDNS service type for CapyDeploy agents.
const ServiceName = "_capydeploy._tcp"

// DefaultPort was the default HTTP port for agent communication.
// Deprecated: Agents now use dynamic ports assigned by the OS.
// This constant is kept for reference and backwards compatibility with tests.
const DefaultPort = 8765

// DefaultTTL is the default TTL for mDNS records.
const DefaultTTL = 120

// DiscoveredAgent represents an agent found via mDNS.
type DiscoveredAgent struct {
	Info       protocol.AgentInfo `json:"info"`
	Host       string             `json:"host"`
	Port       int                `json:"port"`
	IPs        []net.IP           `json:"ips"`
	DiscoveredAt time.Time        `json:"discoveredAt"`
	LastSeen   time.Time          `json:"lastSeen"`
}

// Address returns the HTTP address for connecting to the agent.
func (a *DiscoveredAgent) Address() string {
	if len(a.IPs) > 0 {
		return net.JoinHostPort(a.IPs[0].String(), itoa(a.Port))
	}
	return net.JoinHostPort(a.Host, itoa(a.Port))
}

// WebSocketAddress returns the WebSocket address for the agent.
func (a *DiscoveredAgent) WebSocketAddress() string {
	return "ws://" + a.Address() + "/ws"
}

// IsStale returns true if the agent hasn't been seen recently.
func (a *DiscoveredAgent) IsStale(timeout time.Duration) bool {
	return time.Since(a.LastSeen) > timeout
}

// ServiceInfo contains information for advertising an agent.
type ServiceInfo struct {
	ID       string   `json:"id"`
	Name     string   `json:"name"`
	Platform string   `json:"platform"`
	Version  string   `json:"version"`
	Port     int      `json:"port"`
	IPs      []net.IP `json:"ips,omitempty"`
}

// ToAgentInfo converts ServiceInfo to protocol.AgentInfo.
func (s *ServiceInfo) ToAgentInfo() protocol.AgentInfo {
	return protocol.AgentInfo{
		ID:       s.ID,
		Name:     s.Name,
		Platform: s.Platform,
		Version:  s.Version,
	}
}

// DiscoveryEvent represents a discovery or loss event.
type DiscoveryEvent struct {
	Type  EventType        `json:"type"`
	Agent *DiscoveredAgent `json:"agent"`
}

// EventType indicates the type of discovery event.
type EventType int

const (
	EventDiscovered EventType = iota
	EventUpdated
	EventLost
)

func (e EventType) String() string {
	switch e {
	case EventDiscovered:
		return "discovered"
	case EventUpdated:
		return "updated"
	case EventLost:
		return "lost"
	default:
		return "unknown"
	}
}

// itoa converts int to string without importing strconv.
func itoa(i int) string {
	if i == 0 {
		return "0"
	}
	var b [20]byte
	n := len(b)
	neg := i < 0
	if neg {
		i = -i
	}
	for i > 0 {
		n--
		b[n] = byte('0' + i%10)
		i /= 10
	}
	if neg {
		n--
		b[n] = '-'
	}
	return string(b[n:])
}
