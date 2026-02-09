package main

import (
	"context"
	"fmt"
	"time"

	"github.com/wailsapp/wails/v2/pkg/runtime"

	"github.com/lobinuxsoft/capydeploy/pkg/discovery"
)

// runDiscovery handles mDNS discovery and emits events
func (a *App) runDiscovery() {
	ctx, cancel := context.WithCancel(context.Background())
	a.discoveryCancel = cancel

	// Start continuous discovery
	go a.discoveryClient.StartContinuousDiscovery(ctx, 5*time.Second)

	// Process events
	for event := range a.discoveryClient.Events() {
		a.discoveredMu.Lock()
		switch event.Type {
		case discovery.EventDiscovered:
			a.discoveredCache[event.Agent.Info.ID] = event.Agent
			runtime.EventsEmit(a.ctx, "discovery:agent-found", a.agentToInfo(event.Agent))
		case discovery.EventUpdated:
			a.discoveredCache[event.Agent.Info.ID] = event.Agent
			runtime.EventsEmit(a.ctx, "discovery:agent-updated", a.agentToInfo(event.Agent))
		case discovery.EventLost:
			delete(a.discoveredCache, event.Agent.Info.ID)
			runtime.EventsEmit(a.ctx, "discovery:agent-lost", event.Agent.Info.ID)
		}
		a.discoveredMu.Unlock()
	}
}

// agentToInfo converts a DiscoveredAgent to frontend-friendly info
func (a *App) agentToInfo(agent *discovery.DiscoveredAgent) DiscoveredAgentInfo {
	ips := make([]string, 0, len(agent.IPs))
	for _, ip := range agent.IPs {
		ips = append(ips, ip.String())
	}

	return DiscoveredAgentInfo{
		ID:           agent.Info.ID,
		Name:         agent.Info.Name,
		Platform:     agent.Info.Platform,
		Version:      agent.Info.Version,
		Host:         agent.Host,
		Port:         agent.Port,
		IPs:          ips,
		DiscoveredAt: agent.DiscoveredAt.Format(time.RFC3339),
		LastSeen:     agent.LastSeen.Format(time.RFC3339),
		Online:       !agent.IsStale(30 * time.Second),
	}
}

// GetDiscoveredAgents returns all discovered agents
func (a *App) GetDiscoveredAgents() []DiscoveredAgentInfo {
	a.discoveredMu.RLock()
	defer a.discoveredMu.RUnlock()

	agents := make([]DiscoveredAgentInfo, 0, len(a.discoveredCache))
	for _, agent := range a.discoveredCache {
		agents = append(agents, a.agentToInfo(agent))
	}
	return agents
}

// RefreshDiscovery triggers a manual discovery scan
func (a *App) RefreshDiscovery() ([]DiscoveredAgentInfo, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	agents, err := a.discoveryClient.Discover(ctx, 3*time.Second)
	if err != nil {
		return nil, fmt.Errorf("discovery failed: %w", err)
	}

	// Update cache
	a.discoveredMu.Lock()
	for _, agent := range agents {
		a.discoveredCache[agent.Info.ID] = agent
	}
	a.discoveredMu.Unlock()

	return a.GetDiscoveredAgents(), nil
}
