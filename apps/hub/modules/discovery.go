package modules

import (
	"fmt"

	"github.com/lobinuxsoft/capydeploy/pkg/discovery"
)

// ClientFromAgent creates a platform-appropriate client for a discovered agent.
// It automatically selects the correct module based on the agent's platform.
func ClientFromAgent(agent *discovery.DiscoveredAgent) (PlatformClient, error) {
	if agent == nil {
		return nil, fmt.Errorf("agent is nil")
	}

	platform := agent.Info.Platform
	if platform == "" {
		return nil, fmt.Errorf("agent has no platform information")
	}

	// Get the primary IP address
	host := ""
	if len(agent.IPs) > 0 {
		host = agent.IPs[0].String()
	} else if agent.Host != "" {
		host = agent.Host
	} else {
		return nil, fmt.Errorf("agent has no reachable address")
	}

	return DefaultRegistry.GetClient(platform, host, agent.Port)
}

// ClientFromAgentWithRegistry creates a client using a custom registry.
func ClientFromAgentWithRegistry(registry *Registry, agent *discovery.DiscoveredAgent) (PlatformClient, error) {
	if agent == nil {
		return nil, fmt.Errorf("agent is nil")
	}

	platform := agent.Info.Platform
	if platform == "" {
		return nil, fmt.Errorf("agent has no platform information")
	}

	// Get the primary IP address
	host := ""
	if len(agent.IPs) > 0 {
		host = agent.IPs[0].String()
	} else if agent.Host != "" {
		host = agent.Host
	} else {
		return nil, fmt.Errorf("agent has no reachable address")
	}

	return registry.GetClient(platform, host, agent.Port)
}
