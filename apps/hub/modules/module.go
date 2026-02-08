// Package modules provides platform-specific module interfaces for Hub-Agent communication.
package modules

import (
	"context"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/steam"
	"github.com/lobinuxsoft/capydeploy/pkg/transfer"
)

// PlatformModule defines the interface for platform-specific configuration.
// Each platform (Linux, Windows) implements this interface to provide
// platform-specific metadata like supported image formats.
type PlatformModule interface {
	// Platform returns the platform identifier (e.g., "linux", "windows").
	Platform() string

	// SupportedImageFormats returns the MIME types supported for Steam artwork.
	SupportedImageFormats() []string
}

// PlatformClient is the base interface for all Agent clients.
// Use type assertions to check for additional capabilities.
type PlatformClient interface {
	// Health checks if the agent is responsive.
	Health(ctx context.Context) error

	// GetInfo returns information about the agent.
	GetInfo(ctx context.Context) (*protocol.AgentInfo, error)

	// GetConfig returns the agent configuration.
	GetConfig(ctx context.Context) (*protocol.ConfigResponse, error)
}

// SteamUserProvider provides Steam user information.
type SteamUserProvider interface {
	// GetSteamUsers returns the list of Steam users on the agent.
	GetSteamUsers(ctx context.Context) ([]steam.User, error)
}

// ShortcutManager handles Steam shortcut operations.
type ShortcutManager interface {
	// ListShortcuts returns all shortcuts for a user.
	ListShortcuts(ctx context.Context, userID string) ([]protocol.ShortcutInfo, error)

	// CreateShortcut creates a new Steam shortcut.
	CreateShortcut(ctx context.Context, userID string, config protocol.ShortcutConfig) (uint32, error)

	// DeleteShortcut removes a Steam shortcut by app ID.
	DeleteShortcut(ctx context.Context, userID string, appID uint32) error
}

// ArtworkManager handles Steam artwork operations.
type ArtworkManager interface {
	// ApplyArtwork applies artwork to a shortcut.
	ApplyArtwork(ctx context.Context, userID string, appID uint32, cfg *protocol.ArtworkConfig) (*protocol.ArtworkResponse, error)
}

// SteamController manages Steam process operations.
type SteamController interface {
	// RestartSteam restarts the Steam client.
	RestartSteam(ctx context.Context) (*protocol.RestartSteamResponse, error)
}

// GameManager handles high-level game operations.
// The Agent handles everything internally (user detection, Steam restart, etc.)
type GameManager interface {
	// DeleteGame removes a game completely (shortcut, files, artwork) and restarts Steam.
	DeleteGame(ctx context.Context, appID uint32) (*protocol.DeleteGameResponse, error)
}

// FileUploader handles file upload operations.
type FileUploader interface {
	// InitUpload initializes a new upload session.
	InitUpload(ctx context.Context, config protocol.UploadConfig, totalSize int64, files []transfer.FileEntry) (*protocol.InitUploadResponseFull, error)

	// UploadChunk sends a single chunk to the agent.
	UploadChunk(ctx context.Context, uploadID string, chunk *transfer.Chunk) error

	// CompleteUpload finalizes an upload session.
	CompleteUpload(ctx context.Context, uploadID string, createShortcut bool, shortcut *protocol.ShortcutConfig) (*protocol.CompleteUploadResponseFull, error)

	// CancelUpload cancels an upload session.
	CancelUpload(ctx context.Context, uploadID string) error

	// GetUploadStatus returns the status of an upload session.
	GetUploadStatus(ctx context.Context, uploadID string) (*protocol.UploadProgress, error)
}

// FullPlatformClient combines all client capabilities.
// This is useful for type checking if a client supports all features.
type FullPlatformClient interface {
	PlatformClient
	SteamUserProvider
	ShortcutManager
	ArtworkManager
	SteamController
	FileUploader
	GameManager
}
