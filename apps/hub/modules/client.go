package modules

import (
	"context"
	"fmt"

	"github.com/lobinuxsoft/capydeploy/internal/agent"
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/steam"
	"github.com/lobinuxsoft/capydeploy/pkg/transfer"
)

// baseClient wraps the internal agent.Client and implements all module interfaces.
// This is used by platform-specific modules (Linux, Windows) as the underlying client.
type baseClient struct {
	client   *agent.Client
	platform string
}

// newBaseClient creates a new base client wrapping the internal agent.Client.
func newBaseClient(host string, port int, platform string) *baseClient {
	return &baseClient{
		client:   agent.NewClient(host, port),
		platform: platform,
	}
}

// Ensure baseClient implements all interfaces.
var _ FullPlatformClient = (*baseClient)(nil)

// PlatformClient implementation

func (c *baseClient) Health(ctx context.Context) error {
	return c.client.Health(ctx)
}

func (c *baseClient) GetInfo(ctx context.Context) (*protocol.AgentInfo, error) {
	return c.client.GetInfo(ctx)
}

func (c *baseClient) GetConfig(ctx context.Context) (*agent.AgentConfig, error) {
	return c.client.GetConfig(ctx)
}

// SteamUserProvider implementation

func (c *baseClient) GetSteamUsers(ctx context.Context) ([]steam.User, error) {
	return c.client.GetSteamUsers(ctx)
}

// ShortcutManager implementation

func (c *baseClient) ListShortcuts(ctx context.Context, userID string) ([]protocol.ShortcutInfo, error) {
	return c.client.ListShortcuts(ctx, userID)
}

func (c *baseClient) CreateShortcut(ctx context.Context, userID string, config protocol.ShortcutConfig) (uint32, error) {
	return c.client.CreateShortcut(ctx, userID, config)
}

func (c *baseClient) DeleteShortcut(ctx context.Context, userID string, appID uint32) error {
	return c.client.DeleteShortcut(ctx, userID, appID)
}

// ArtworkManager implementation

func (c *baseClient) ApplyArtwork(ctx context.Context, userID string, appID uint32, cfg *protocol.ArtworkConfig) (*agent.ApplyArtworkResult, error) {
	return c.client.ApplyArtwork(ctx, userID, appID, cfg)
}

// SteamController implementation

func (c *baseClient) RestartSteam(ctx context.Context) (*agent.RestartSteamResult, error) {
	return c.client.RestartSteam(ctx)
}

// GameManager implementation

func (c *baseClient) DeleteGame(ctx context.Context, appID uint32) (*agent.DeleteGameResult, error) {
	// DeleteGame is only available via WebSocket - use WSClient instead
	return nil, fmt.Errorf("DeleteGame requires WebSocket connection")
}

// FileUploader implementation

func (c *baseClient) InitUpload(ctx context.Context, config protocol.UploadConfig, totalSize int64, files []transfer.FileEntry) (*agent.InitUploadResponse, error) {
	return c.client.InitUpload(ctx, config, totalSize, files)
}

func (c *baseClient) UploadChunk(ctx context.Context, uploadID string, chunk *transfer.Chunk) error {
	return c.client.UploadChunk(ctx, uploadID, chunk)
}

func (c *baseClient) CompleteUpload(ctx context.Context, uploadID string, createShortcut bool, shortcut *protocol.ShortcutConfig) (*agent.CompleteUploadResponse, error) {
	return c.client.CompleteUpload(ctx, uploadID, createShortcut, shortcut)
}

func (c *baseClient) CancelUpload(ctx context.Context, uploadID string) error {
	return c.client.CancelUpload(ctx, uploadID)
}

func (c *baseClient) GetUploadStatus(ctx context.Context, uploadID string) (*protocol.UploadProgress, error) {
	return c.client.GetUploadStatus(ctx, uploadID)
}
