package discovery

import (
	"context"
	"net"
	"strings"
	"sync"
	"time"

	"github.com/grandcat/zeroconf"
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

// Client discovers agents on the local network via mDNS/DNS-SD.
type Client struct {
	mu       sync.RWMutex
	agents   map[string]*DiscoveredAgent
	eventsCh chan DiscoveryEvent
	timeout  time.Duration
}

// NewClient creates a new mDNS discovery client.
func NewClient() *Client {
	return &Client{
		agents:   make(map[string]*DiscoveredAgent),
		eventsCh: make(chan DiscoveryEvent, 16),
		timeout:  time.Duration(DefaultTTL) * time.Second,
	}
}

// SetTimeout sets the stale agent timeout.
func (c *Client) SetTimeout(d time.Duration) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.timeout = d
}

// Events returns a channel of discovery events.
func (c *Client) Events() <-chan DiscoveryEvent {
	return c.eventsCh
}

// Discover performs a one-time mDNS query and returns discovered agents.
func (c *Client) Discover(ctx context.Context, timeout time.Duration) ([]*DiscoveredAgent, error) {
	resolver, err := zeroconf.NewResolver(nil)
	if err != nil {
		return nil, err
	}

	entries := make(chan *zeroconf.ServiceEntry)
	var agents []*DiscoveredAgent
	var wg sync.WaitGroup

	wg.Add(1)
	go func() {
		defer wg.Done()
		for entry := range entries {
			if agent := c.processEntry(entry); agent != nil {
				agents = append(agents, agent)
			}
		}
	}()

	// Create context with timeout
	browseCtx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()

	// Browse for services
	err = resolver.Browse(browseCtx, ServiceName, "local.", entries)
	if err != nil {
		return nil, err
	}

	// Wait for browsing to complete (zeroconf closes the channel)
	<-browseCtx.Done()
	wg.Wait()

	return agents, nil
}

// StartContinuousDiscovery begins continuous agent discovery.
func (c *Client) StartContinuousDiscovery(ctx context.Context, interval time.Duration) {
	ticker := time.NewTicker(interval)
	defer ticker.Stop()

	// Initial discovery
	c.Discover(ctx, 3*time.Second)

	for {
		select {
		case <-ticker.C:
			c.Discover(ctx, 3*time.Second)
			c.pruneStaleAgents()
		case <-ctx.Done():
			return
		}
	}
}

// processEntry converts a zeroconf entry to a DiscoveredAgent.
func (c *Client) processEntry(entry *zeroconf.ServiceEntry) *DiscoveredAgent {
	if entry == nil {
		return nil
	}

	// Parse TXT records
	info := protocol.AgentInfo{}
	for _, txt := range entry.Text {
		switch {
		case strings.HasPrefix(txt, "id="):
			info.ID = txt[3:]
		case strings.HasPrefix(txt, "name="):
			info.Name = txt[5:]
		case strings.HasPrefix(txt, "platform="):
			info.Platform = txt[9:]
		case strings.HasPrefix(txt, "version="):
			info.Version = txt[8:]
		}
	}

	// Use instance name as ID if not in TXT
	if info.ID == "" {
		info.ID = entry.Instance
	}
	if info.Name == "" {
		info.Name = entry.HostName
	}

	// Collect IPs (filter out link-local)
	var ips []net.IP
	for _, ip := range entry.AddrIPv4 {
		ip4 := ip.To4()
		if ip4 != nil && !(ip4[0] == 169 && ip4[1] == 254) {
			ips = append(ips, ip)
		}
	}

	now := time.Now()
	agent := &DiscoveredAgent{
		Info:         info,
		Host:         entry.HostName,
		Port:         entry.Port,
		IPs:          ips,
		DiscoveredAt: now,
		LastSeen:     now,
	}

	// Update or add agent
	c.mu.Lock()
	existing, exists := c.agents[info.ID]
	if exists {
		existing.LastSeen = now
		existing.IPs = ips
		existing.Port = entry.Port
		agent = existing
		c.mu.Unlock()
		c.emitEvent(DiscoveryEvent{Type: EventUpdated, Agent: agent})
	} else {
		c.agents[info.ID] = agent
		c.mu.Unlock()
		c.emitEvent(DiscoveryEvent{Type: EventDiscovered, Agent: agent})
	}

	return agent
}

// pruneStaleAgents removes agents that haven't been seen recently.
func (c *Client) pruneStaleAgents() {
	c.mu.Lock()
	defer c.mu.Unlock()

	for id, agent := range c.agents {
		if agent.IsStale(c.timeout) {
			delete(c.agents, id)
			c.emitEvent(DiscoveryEvent{Type: EventLost, Agent: agent})
		}
	}
}

// emitEvent sends an event non-blocking.
func (c *Client) emitEvent(event DiscoveryEvent) {
	select {
	case c.eventsCh <- event:
	default:
		// Channel full, skip event
	}
}

// GetAgents returns all currently known agents.
func (c *Client) GetAgents() []*DiscoveredAgent {
	c.mu.RLock()
	defer c.mu.RUnlock()

	agents := make([]*DiscoveredAgent, 0, len(c.agents))
	for _, agent := range c.agents {
		agents = append(agents, agent)
	}
	return agents
}

// GetAgent returns a specific agent by ID.
func (c *Client) GetAgent(id string) *DiscoveredAgent {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return c.agents[id]
}

// RemoveAgent removes an agent from tracking.
func (c *Client) RemoveAgent(id string) {
	c.mu.Lock()
	agent, exists := c.agents[id]
	if exists {
		delete(c.agents, id)
	}
	c.mu.Unlock()

	if exists {
		c.emitEvent(DiscoveryEvent{Type: EventLost, Agent: agent})
	}
}

// Clear removes all tracked agents.
func (c *Client) Clear() {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.agents = make(map[string]*DiscoveredAgent)
}

// Close closes the client and its event channel.
func (c *Client) Close() {
	close(c.eventsCh)
}
