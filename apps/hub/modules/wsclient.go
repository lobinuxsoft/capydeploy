package modules

import (
	"context"
	"fmt"
	"strconv"

	"github.com/lobinuxsoft/capydeploy/apps/hub/wsclient"
	"github.com/lobinuxsoft/capydeploy/internal/agent"
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/steam"
	"github.com/lobinuxsoft/capydeploy/pkg/transfer"
)

// WSClient wraps wsclient.Client and implements all module interfaces.
type WSClient struct {
	client   *wsclient.Client
	platform string
}

// NewWSClient creates a new WebSocket-based client.
func NewWSClient(host string, port int, platform, hubName, hubVersion string) *WSClient {
	return &WSClient{
		client:   wsclient.NewClient(host, port, hubName, hubVersion),
		platform: platform,
	}
}

// Ensure WSClient implements all interfaces.
var _ FullPlatformClient = (*WSClient)(nil)

// Connect establishes the WebSocket connection.
func (c *WSClient) Connect(ctx context.Context) error {
	return c.client.Connect(ctx)
}

// Close closes the WebSocket connection.
func (c *WSClient) Close() error {
	return c.client.Close()
}

// IsConnected returns true if the client is connected.
func (c *WSClient) IsConnected() bool {
	return c.client.IsConnected()
}

// SetCallbacks sets the event callbacks.
func (c *WSClient) SetCallbacks(onDisconnect func(), onUploadProgress func(protocol.UploadProgressEvent), onOperationEvent func(protocol.OperationEvent)) {
	c.client.SetCallbacks(onDisconnect, onUploadProgress, onOperationEvent)
}

// PlatformClient implementation

func (c *WSClient) Health(ctx context.Context) error {
	// For WS, connection itself is the health check
	if !c.client.IsConnected() {
		return fmt.Errorf("not connected")
	}
	return nil
}

func (c *WSClient) GetInfo(ctx context.Context) (*protocol.AgentInfo, error) {
	return c.client.GetInfo(ctx)
}

func (c *WSClient) GetConfig(ctx context.Context) (*agent.AgentConfig, error) {
	resp, err := c.client.GetConfig(ctx)
	if err != nil {
		return nil, err
	}
	return &agent.AgentConfig{
		InstallPath: resp.InstallPath,
	}, nil
}

// SteamUserProvider implementation

func (c *WSClient) GetSteamUsers(ctx context.Context) ([]steam.User, error) {
	users, err := c.client.GetSteamUsers(ctx)
	if err != nil {
		return nil, err
	}

	// Convert protocol.SteamUser to steam.User
	result := make([]steam.User, len(users))
	for i, u := range users {
		result[i] = steam.User{
			ID: u.ID,
			// HasShortcuts not available from protocol, default false
		}
	}
	return result, nil
}

// ShortcutManager implementation

func (c *WSClient) ListShortcuts(ctx context.Context, userID string) ([]protocol.ShortcutInfo, error) {
	uid, err := strconv.ParseUint(userID, 10, 32)
	if err != nil {
		return nil, fmt.Errorf("invalid userID: %w", err)
	}
	return c.client.ListShortcuts(ctx, uint32(uid))
}

func (c *WSClient) CreateShortcut(ctx context.Context, userID string, config protocol.ShortcutConfig) (uint32, error) {
	uid, err := strconv.ParseUint(userID, 10, 32)
	if err != nil {
		return 0, fmt.Errorf("invalid userID: %w", err)
	}
	return c.client.CreateShortcut(ctx, uint32(uid), config)
}

func (c *WSClient) DeleteShortcut(ctx context.Context, userID string, appID uint32) error {
	return c.client.DeleteShortcut(ctx, userID, appID, false)
}

// ArtworkManager implementation

func (c *WSClient) ApplyArtwork(ctx context.Context, userID string, appID uint32, cfg *protocol.ArtworkConfig) (*agent.ApplyArtworkResult, error) {
	resp, err := c.client.ApplyArtwork(ctx, userID, appID, cfg)
	if err != nil {
		return nil, err
	}

	// Convert protocol.ArtworkResponse to agent.ApplyArtworkResult
	result := &agent.ApplyArtworkResult{
		Applied: resp.Applied,
	}
	for _, f := range resp.Failed {
		result.Failed = append(result.Failed, struct {
			Type  string `json:"type"`
			Error string `json:"error,omitempty"`
		}{
			Type:  f.Type,
			Error: f.Error,
		})
	}
	return result, nil
}

// SteamController implementation

func (c *WSClient) RestartSteam(ctx context.Context) (*agent.RestartSteamResult, error) {
	resp, err := c.client.RestartSteam(ctx)
	if err != nil {
		return nil, err
	}
	return &agent.RestartSteamResult{
		Success: resp.Success,
		Message: resp.Message,
	}, nil
}

// FileUploader implementation

func (c *WSClient) InitUpload(ctx context.Context, config protocol.UploadConfig, totalSize int64, files []transfer.FileEntry) (*agent.InitUploadResponse, error) {
	// Convert transfer.FileEntry to protocol.FileEntry
	protoFiles := make([]protocol.FileEntry, len(files))
	for i, f := range files {
		protoFiles[i] = protocol.FileEntry{
			RelativePath: f.RelativePath,
			Size:         f.Size,
		}
	}

	resp, err := c.client.InitUpload(ctx, config, totalSize, protoFiles)
	if err != nil {
		return nil, err
	}

	return &agent.InitUploadResponse{
		UploadID:   resp.UploadID,
		ChunkSize:  resp.ChunkSize,
		ResumeFrom: resp.ResumeFrom,
	}, nil
}

func (c *WSClient) UploadChunk(ctx context.Context, uploadID string, chunk *transfer.Chunk) error {
	return c.client.UploadChunk(ctx, uploadID, chunk.FilePath, chunk.Offset, chunk.Data, chunk.Checksum)
}

func (c *WSClient) CompleteUpload(ctx context.Context, uploadID string, createShortcut bool, shortcut *protocol.ShortcutConfig) (*agent.CompleteUploadResponse, error) {
	resp, err := c.client.CompleteUpload(ctx, uploadID, createShortcut, shortcut)
	if err != nil {
		return nil, err
	}

	return &agent.CompleteUploadResponse{
		Success: resp.Success,
		Path:    resp.Path,
		AppID:   resp.AppID,
	}, nil
}

func (c *WSClient) CancelUpload(ctx context.Context, uploadID string) error {
	return c.client.CancelUpload(ctx, uploadID)
}

func (c *WSClient) GetUploadStatus(ctx context.Context, uploadID string) (*protocol.UploadProgress, error) {
	// WS client doesn't have this method directly - uploads use progress events
	return nil, fmt.Errorf("use progress events for WS uploads")
}
