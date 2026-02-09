package main

import (
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/wailsapp/wails/v2/pkg/runtime"

	"github.com/lobinuxsoft/capydeploy/apps/hub/modules"
	"github.com/lobinuxsoft/capydeploy/apps/hub/wsclient"
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/version"
)

// ConnectAgent connects to an agent by ID using WebSocket
func (a *App) ConnectAgent(agentID string) error {
	// Find agent in cache
	a.discoveredMu.RLock()
	agent, ok := a.discoveredCache[agentID]
	a.discoveredMu.RUnlock()

	if !ok {
		return fmt.Errorf("agent not found: %s", agentID)
	}

	// Disconnect existing connection
	a.DisconnectAgent()

	// Create WebSocket client with auth
	var wsClient *modules.WSClient
	var err error

	// Get hub name from config (fallback to default)
	hubName := "CapyDeploy Hub"
	hubPlatform := ""
	if a.configMgr != nil {
		hubName = a.configMgr.GetName()
		hubPlatform = a.configMgr.GetPlatform()
	}

	if a.tokenStore != nil {
		wsClient, err = modules.WSClientFromAgentWithAuth(
			agent,
			hubName,
			version.Version,
			a.tokenStore.GetHubID(),
			a.tokenStore.GetToken,
			a.tokenStore.SaveToken,
		)
	} else {
		wsClient, err = modules.WSClientFromAgent(agent, hubName, version.Version)
	}

	// Set hub platform for agent to store
	if wsClient != nil && hubPlatform != "" {
		wsClient.SetPlatform(hubPlatform)
	}
	if err != nil {
		return fmt.Errorf("failed to create WS client: %w", err)
	}

	// Set callbacks for push events
	wsClient.SetCallbacks(
		func() {
			// On disconnect
			a.mu.Lock()
			a.connectedAgent = nil
			a.mu.Unlock()
			runtime.EventsEmit(a.ctx, "connection:changed", a.GetConnectionStatus())
		},
		func(event protocol.UploadProgressEvent) {
			// On upload progress
			runtime.EventsEmit(a.ctx, "upload:progress", UploadProgress{
				Progress: event.Percentage,
				Status:   fmt.Sprintf("Uploading: %s", event.CurrentFile),
				Done:     false,
			})
		},
		func(event protocol.OperationEvent) {
			// On operation event
			runtime.EventsEmit(a.ctx, "operation:event", event)
		},
	)

	// Set pairing callback
	wsClient.SetPairingCallback(func(agentID string) {
		runtime.EventsEmit(a.ctx, "pairing:required", agentID)
	})

	// Connect via WebSocket
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	if err := wsClient.Connect(ctx); err != nil {
		// Check if pairing is required
		if errors.Is(err, wsclient.ErrPairingRequired) {
			// Store the client for pairing completion
			a.mu.Lock()
			a.connectedAgent = &ConnectedAgent{
				Agent:    agent,
				Client:   wsClient,
				WSClient: wsClient,
				Info:     nil, // Not yet authenticated
			}
			a.mu.Unlock()

			// Emit pairing required event
			runtime.EventsEmit(a.ctx, "pairing:required", agentID)
			return nil // Not an error, waiting for pairing
		}
		return fmt.Errorf("failed to connect: %w", err)
	}

	// Get full agent info (includes capabilities)
	agentInfo, err := wsClient.GetInfo(ctx)
	if err != nil {
		wsClient.Close()
		return fmt.Errorf("failed to get agent info: %w", err)
	}

	a.mu.Lock()
	a.connectedAgent = &ConnectedAgent{
		Agent:    agent,
		Client:   wsClient,
		WSClient: wsClient,
		Info:     agentInfo,
	}
	a.mu.Unlock()

	// Emit connection status change
	runtime.EventsEmit(a.ctx, "connection:changed", a.GetConnectionStatus())

	return nil
}

// DisconnectAgent disconnects from the current agent
func (a *App) DisconnectAgent() {
	a.mu.Lock()
	if a.connectedAgent != nil && a.connectedAgent.WSClient != nil {
		a.connectedAgent.WSClient.Close()
	}
	a.connectedAgent = nil
	a.mu.Unlock()

	// Emit connection status change
	runtime.EventsEmit(a.ctx, "connection:changed", a.GetConnectionStatus())
}

// ConfirmPairing confirms a pairing with the connected agent using the provided code.
func (a *App) ConfirmPairing(code string) error {
	a.mu.RLock()
	if a.connectedAgent == nil || a.connectedAgent.WSClient == nil {
		a.mu.RUnlock()
		return fmt.Errorf("no agent connection pending pairing")
	}
	wsClient := a.connectedAgent.WSClient
	agent := a.connectedAgent.Agent
	a.mu.RUnlock()

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	// Confirm pairing
	if err := wsClient.ConfirmPairing(ctx, code); err != nil {
		return fmt.Errorf("pairing failed: %w", err)
	}

	// Now get agent info
	agentInfo, err := wsClient.GetInfo(ctx)
	if err != nil {
		return fmt.Errorf("failed to get agent info after pairing: %w", err)
	}

	a.mu.Lock()
	a.connectedAgent = &ConnectedAgent{
		Agent:    agent,
		Client:   wsClient,
		WSClient: wsClient,
		Info:     agentInfo,
	}
	a.mu.Unlock()

	// Emit connection status change
	runtime.EventsEmit(a.ctx, "connection:changed", a.GetConnectionStatus())

	return nil
}

// CancelPairing cancels a pending pairing and disconnects.
func (a *App) CancelPairing() {
	a.DisconnectAgent()
}

// GetConnectionStatus returns the current connection status
func (a *App) GetConnectionStatus() ConnectionStatus {
	a.mu.RLock()
	defer a.mu.RUnlock()

	if a.connectedAgent == nil {
		return ConnectionStatus{Connected: false}
	}

	agent := a.connectedAgent.Agent
	info := a.connectedAgent.Info

	ips := make([]string, 0, len(agent.IPs))
	for _, ip := range agent.IPs {
		ips = append(ips, ip.String())
	}

	// Convert capabilities to strings for JSON serialization
	var capabilities []string
	if info != nil {
		capabilities = make([]string, len(info.Capabilities))
		for i, cap := range info.Capabilities {
			capabilities[i] = string(cap)
		}
	}

	// Use formats from agent info if available, otherwise fall back to platform-based
	var supportedFormats []string
	if info != nil && len(info.SupportedImageFormats) > 0 {
		supportedFormats = info.SupportedImageFormats
	} else {
		supportedFormats = modules.GetSupportedImageFormats(agent.Info.Platform)
	}

	return ConnectionStatus{
		Connected:             true,
		AgentID:               agent.Info.ID,
		AgentName:             agent.Info.Name,
		Platform:              agent.Info.Platform,
		Host:                  agent.Host,
		Port:                  agent.Port,
		IPs:                   ips,
		SupportedImageFormats: supportedFormats,
		Capabilities:          capabilities,
	}
}

// GetAgentInstallPath returns the install path from the connected agent
func (a *App) GetAgentInstallPath() (string, error) {
	a.mu.RLock()
	if a.connectedAgent == nil {
		a.mu.RUnlock()
		return "", fmt.Errorf("no agent connected")
	}
	client := a.connectedAgent.Client
	a.mu.RUnlock()

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	config, err := client.GetConfig(ctx)
	if err != nil {
		return "", fmt.Errorf("failed to get agent config: %w", err)
	}

	return config.InstallPath, nil
}
