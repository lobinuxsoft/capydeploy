package modules

// Type assertion helpers for checking client capabilities.
// These functions make it easy to verify and use optional interfaces.

// AsShortcutManager checks if the client supports shortcut operations.
// Returns the ShortcutManager interface and true if supported.
func AsShortcutManager(client PlatformClient) (ShortcutManager, bool) {
	sm, ok := client.(ShortcutManager)
	return sm, ok
}

// AsArtworkManager checks if the client supports artwork operations.
// Returns the ArtworkManager interface and true if supported.
func AsArtworkManager(client PlatformClient) (ArtworkManager, bool) {
	am, ok := client.(ArtworkManager)
	return am, ok
}

// AsSteamController checks if the client supports Steam control operations.
// Returns the SteamController interface and true if supported.
func AsSteamController(client PlatformClient) (SteamController, bool) {
	sc, ok := client.(SteamController)
	return sc, ok
}

// AsFileUploader checks if the client supports file upload operations.
// Returns the FileUploader interface and true if supported.
func AsFileUploader(client PlatformClient) (FileUploader, bool) {
	fu, ok := client.(FileUploader)
	return fu, ok
}

// AsSteamUserProvider checks if the client can provide Steam user information.
// Returns the SteamUserProvider interface and true if supported.
func AsSteamUserProvider(client PlatformClient) (SteamUserProvider, bool) {
	sup, ok := client.(SteamUserProvider)
	return sup, ok
}

// AsGameManager checks if the client supports high-level game operations.
// Returns the GameManager interface and true if supported.
func AsGameManager(client PlatformClient) (GameManager, bool) {
	gm, ok := client.(GameManager)
	return gm, ok
}

// AsFullClient checks if the client supports all capabilities.
// Returns the FullPlatformClient interface and true if supported.
func AsFullClient(client PlatformClient) (FullPlatformClient, bool) {
	fc, ok := client.(FullPlatformClient)
	return fc, ok
}

// ClientCapabilities represents the capabilities of a platform client.
type ClientCapabilities struct {
	Shortcuts  bool
	Artwork    bool
	Steam      bool
	Upload     bool
	SteamUsers bool
}

// GetCapabilities returns a summary of what operations a client supports.
func GetCapabilities(client PlatformClient) ClientCapabilities {
	return ClientCapabilities{
		Shortcuts:  hasInterface[ShortcutManager](client),
		Artwork:    hasInterface[ArtworkManager](client),
		Steam:      hasInterface[SteamController](client),
		Upload:     hasInterface[FileUploader](client),
		SteamUsers: hasInterface[SteamUserProvider](client),
	}
}

// hasInterface is a generic helper to check if a value implements an interface.
func hasInterface[T any](v any) bool {
	_, ok := v.(T)
	return ok
}
