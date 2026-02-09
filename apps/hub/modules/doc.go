// Package modules provides platform-specific modules for Hub-Agent communication.
//
// The module system allows the Hub to communicate with Agents running on different
// platforms (Linux, Windows) using a unified interface. WebSocket clients implement
// the same set of interfaces, enabling platform-agnostic code.
//
// # Architecture
//
// The module system consists of:
//   - PlatformModule: Interface for platform-specific metadata (image formats)
//   - PlatformClient: Base interface for all Agent clients (health, info, config)
//   - Capability interfaces: ShortcutManager, ArtworkManager, SteamController, FileUploader
//   - Registry: Manages modules and provides platform metadata lookup
//   - WSClient: WebSocket-based client implementing all interfaces
//
// # Usage
//
// Basic usage with discovered agents:
//
//	agents, _ := discoveryClient.Discover(ctx, 3*time.Second)
//	for _, agent := range agents {
//	    wsClient, err := modules.WSClientFromAgent(agent, hubName, version)
//	    if err != nil {
//	        continue
//	    }
//	    if err := wsClient.Connect(ctx); err != nil {
//	        continue
//	    }
//
//	    // Check capabilities using type assertions
//	    if sm, ok := modules.AsShortcutManager(wsClient); ok {
//	        shortcuts, _ := sm.ListShortcuts(ctx, userID)
//	    }
//	}
//
// # Capability Checking
//
// Use the As* functions to check for specific capabilities:
//
//	caps := modules.GetCapabilities(client)
//	if caps.Shortcuts {
//	    // Client supports shortcut operations
//	}
//	if caps.Upload {
//	    // Client supports file uploads
//	}
package modules
