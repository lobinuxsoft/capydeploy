// Package modules provides platform-specific modules for Hub-Agent communication.
//
// The module system allows the Hub to communicate with Agents running on different
// platforms (Linux, Windows) using a unified interface. Each platform module creates
// clients that implement the same set of interfaces, enabling platform-agnostic code.
//
// # Architecture
//
// The module system consists of:
//   - PlatformModule: Interface for creating platform-specific clients
//   - PlatformClient: Base interface for all Agent clients (health, info)
//   - Capability interfaces: ShortcutManager, ArtworkManager, SteamController, FileUploader
//   - Registry: Manages modules and provides automatic platform selection
//
// # Usage
//
// Basic usage with discovered agents:
//
//	// Using the discovery client
//	agents, _ := discoveryClient.Discover(ctx, 3*time.Second)
//	for _, agent := range agents {
//	    client, err := modules.ClientFromAgent(agent)
//	    if err != nil {
//	        continue
//	    }
//
//	    // Check capabilities using type assertions
//	    if sm, ok := modules.AsShortcutManager(client); ok {
//	        shortcuts, _ := sm.ListShortcuts(ctx, userID)
//	        // ...
//	    }
//	}
//
// Direct client creation:
//
//	// Create client for a known platform
//	client, err := modules.GetClientForPlatform("linux", "192.168.1.100", 8765)
//	if err != nil {
//	    // Handle error
//	}
//
//	// Check health
//	if err := client.Health(ctx); err != nil {
//	    // Agent not healthy
//	}
//
//	// Use type assertions for optional features
//	if uploader, ok := modules.AsFileUploader(client); ok {
//	    resp, _ := uploader.InitUpload(ctx, config, totalSize, files)
//	    // ...
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
//
// # Custom Modules
//
// To add support for a new platform:
//
//	type MyPlatformModule struct{}
//
//	func (m *MyPlatformModule) Platform() string {
//	    return "myplatform"
//	}
//
//	func (m *MyPlatformModule) NewClient(host string, port int) modules.PlatformClient {
//	    // Return your client implementation
//	}
//
//	// Register the module
//	modules.DefaultRegistry.Register(&MyPlatformModule{})
package modules
