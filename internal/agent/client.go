// Package agent provides an HTTP client for communicating with CapyDeploy Agents.
package agent

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/steam"
	"github.com/lobinuxsoft/capydeploy/pkg/transfer"
)

// Client is an HTTP client for communicating with a CapyDeploy Agent.
type Client struct {
	baseURL    string
	httpClient *http.Client
}

// NewClient creates a new Agent client.
func NewClient(host string, port int) *Client {
	return &Client{
		baseURL: fmt.Sprintf("http://%s:%d", host, port),
		httpClient: &http.Client{
			Timeout: 30 * time.Second,
		},
	}
}

// NewClientWithURL creates a new Agent client with a custom base URL.
func NewClientWithURL(baseURL string) *Client {
	return &Client{
		baseURL: baseURL,
		httpClient: &http.Client{
			Timeout: 30 * time.Second,
		},
	}
}

// SetTimeout sets the HTTP client timeout.
func (c *Client) SetTimeout(timeout time.Duration) {
	c.httpClient.Timeout = timeout
}

// Health checks if the agent is healthy.
func (c *Client) Health(ctx context.Context) error {
	req, err := http.NewRequestWithContext(ctx, "GET", c.baseURL+"/health", nil)
	if err != nil {
		return err
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return fmt.Errorf("health check failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("health check returned status %d", resp.StatusCode)
	}

	return nil
}

// GetInfo returns information about the agent.
func (c *Client) GetInfo(ctx context.Context) (*protocol.AgentInfo, error) {
	req, err := http.NewRequestWithContext(ctx, "GET", c.baseURL+"/info", nil)
	if err != nil {
		return nil, err
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("get info failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("get info returned status %d", resp.StatusCode)
	}

	var info protocol.AgentInfo
	if err := json.NewDecoder(resp.Body).Decode(&info); err != nil {
		return nil, fmt.Errorf("failed to decode info: %w", err)
	}

	return &info, nil
}

// AgentConfig represents the agent configuration.
type AgentConfig struct {
	InstallPath string `json:"installPath"`
}

// GetConfig returns the agent configuration.
func (c *Client) GetConfig(ctx context.Context) (*AgentConfig, error) {
	req, err := http.NewRequestWithContext(ctx, "GET", c.baseURL+"/config", nil)
	if err != nil {
		return nil, err
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("get config failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("get config returned status %d", resp.StatusCode)
	}

	var config AgentConfig
	if err := json.NewDecoder(resp.Body).Decode(&config); err != nil {
		return nil, fmt.Errorf("failed to decode config: %w", err)
	}

	return &config, nil
}

// GetSteamUsers returns the list of Steam users on the agent.
func (c *Client) GetSteamUsers(ctx context.Context) ([]steam.User, error) {
	req, err := http.NewRequestWithContext(ctx, "GET", c.baseURL+"/steam/users", nil)
	if err != nil {
		return nil, err
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("get steam users failed: %w", err)
	}
	defer resp.Body.Close()

	var result struct {
		Users []steam.User `json:"users"`
		Error string       `json:"error,omitempty"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, fmt.Errorf("failed to decode users: %w", err)
	}

	if result.Error != "" {
		return nil, fmt.Errorf("agent error: %s", result.Error)
	}

	return result.Users, nil
}

// Upload types

// InitUploadRequest is the request body for initializing an upload.
type InitUploadRequest struct {
	Config    protocol.UploadConfig `json:"config"`
	TotalSize int64                 `json:"totalSize"`
	Files     []transfer.FileEntry  `json:"files"`
}

// InitUploadResponse is the response from initializing an upload.
type InitUploadResponse struct {
	UploadID   string           `json:"uploadId"`
	ChunkSize  int              `json:"chunkSize"`
	ResumeFrom map[string]int64 `json:"resumeFrom,omitempty"`
	Error      string           `json:"error,omitempty"`
}

// InitUpload initializes a new upload session on the agent.
func (c *Client) InitUpload(ctx context.Context, config protocol.UploadConfig, totalSize int64, files []transfer.FileEntry) (*InitUploadResponse, error) {
	reqBody := InitUploadRequest{
		Config:    config,
		TotalSize: totalSize,
		Files:     files,
	}

	body, err := json.Marshal(reqBody)
	if err != nil {
		return nil, err
	}

	req, err := http.NewRequestWithContext(ctx, "POST", c.baseURL+"/uploads", bytes.NewReader(body))
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("init upload failed: %w", err)
	}
	defer resp.Body.Close()

	var result InitUploadResponse
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}

	if result.Error != "" {
		return nil, fmt.Errorf("agent error: %s", result.Error)
	}

	return &result, nil
}

// UploadChunk sends a single chunk to the agent.
func (c *Client) UploadChunk(ctx context.Context, uploadID string, chunk *transfer.Chunk) error {
	url := fmt.Sprintf("%s/uploads/%s/chunks", c.baseURL, uploadID)

	req, err := http.NewRequestWithContext(ctx, "POST", url, bytes.NewReader(chunk.Data))
	if err != nil {
		return err
	}

	req.Header.Set("Content-Type", "application/octet-stream")
	req.Header.Set("X-File-Path", chunk.FilePath)
	req.Header.Set("X-Chunk-Offset", fmt.Sprintf("%d", chunk.Offset))
	if chunk.Checksum != "" {
		req.Header.Set("X-Chunk-Checksum", chunk.Checksum)
	}

	// Use a longer timeout for chunk uploads
	client := &http.Client{Timeout: 5 * time.Minute}
	resp, err := client.Do(req)
	if err != nil {
		return fmt.Errorf("upload chunk failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("upload chunk returned status %d: %s", resp.StatusCode, string(body))
	}

	return nil
}

// CompleteUploadRequest is the request body for completing an upload.
type CompleteUploadRequest struct {
	CreateShortcut bool                     `json:"createShortcut"`
	Shortcut       *protocol.ShortcutConfig `json:"shortcut,omitempty"`
}

// CompleteUploadResponse is the response from completing an upload.
type CompleteUploadResponse struct {
	Success bool   `json:"success"`
	Path    string `json:"path,omitempty"`
	AppID   uint32 `json:"appId,omitempty"`
	Error   string `json:"error,omitempty"`
}

// CompleteUpload finalizes an upload session.
func (c *Client) CompleteUpload(ctx context.Context, uploadID string, createShortcut bool, shortcut *protocol.ShortcutConfig) (*CompleteUploadResponse, error) {
	url := fmt.Sprintf("%s/uploads/%s/complete", c.baseURL, uploadID)

	reqBody := CompleteUploadRequest{
		CreateShortcut: createShortcut,
		Shortcut:       shortcut,
	}

	body, err := json.Marshal(reqBody)
	if err != nil {
		return nil, err
	}

	req, err := http.NewRequestWithContext(ctx, "POST", url, bytes.NewReader(body))
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("complete upload failed: %w", err)
	}
	defer resp.Body.Close()

	var result CompleteUploadResponse
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}

	if result.Error != "" {
		return nil, fmt.Errorf("agent error: %s", result.Error)
	}

	return &result, nil
}

// CancelUpload cancels an upload session.
func (c *Client) CancelUpload(ctx context.Context, uploadID string) error {
	url := fmt.Sprintf("%s/uploads/%s", c.baseURL, uploadID)

	req, err := http.NewRequestWithContext(ctx, "DELETE", url, nil)
	if err != nil {
		return err
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return fmt.Errorf("cancel upload failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("cancel upload returned status %d", resp.StatusCode)
	}

	return nil
}

// GetUploadStatus returns the status of an upload session.
func (c *Client) GetUploadStatus(ctx context.Context, uploadID string) (*protocol.UploadProgress, error) {
	url := fmt.Sprintf("%s/uploads/%s", c.baseURL, uploadID)

	req, err := http.NewRequestWithContext(ctx, "GET", url, nil)
	if err != nil {
		return nil, err
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("get upload status failed: %w", err)
	}
	defer resp.Body.Close()

	var result struct {
		Progress *protocol.UploadProgress `json:"progress"`
		Error    string                   `json:"error,omitempty"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}

	if result.Error != "" {
		return nil, fmt.Errorf("agent error: %s", result.Error)
	}

	return result.Progress, nil
}

// Shortcuts

// ListShortcuts returns the shortcuts for a Steam user.
func (c *Client) ListShortcuts(ctx context.Context, userID string) ([]protocol.ShortcutInfo, error) {
	url := fmt.Sprintf("%s/shortcuts/%s", c.baseURL, userID)

	req, err := http.NewRequestWithContext(ctx, "GET", url, nil)
	if err != nil {
		return nil, err
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("list shortcuts failed: %w", err)
	}
	defer resp.Body.Close()

	var result struct {
		Shortcuts []protocol.ShortcutInfo `json:"shortcuts"`
		Error     string                  `json:"error,omitempty"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}

	if result.Error != "" {
		return nil, fmt.Errorf("agent error: %s", result.Error)
	}

	return result.Shortcuts, nil
}

// CreateShortcut creates a new shortcut for a Steam user.
func (c *Client) CreateShortcut(ctx context.Context, userID string, config protocol.ShortcutConfig) (uint32, error) {
	url := fmt.Sprintf("%s/shortcuts/%s", c.baseURL, userID)

	body, err := json.Marshal(config)
	if err != nil {
		return 0, err
	}

	req, err := http.NewRequestWithContext(ctx, "POST", url, bytes.NewReader(body))
	if err != nil {
		return 0, err
	}
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return 0, fmt.Errorf("create shortcut failed: %w", err)
	}
	defer resp.Body.Close()

	var result struct {
		AppID uint32 `json:"appId"`
		Error string `json:"error,omitempty"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return 0, fmt.Errorf("failed to decode response: %w", err)
	}

	if result.Error != "" {
		return 0, fmt.Errorf("agent error: %s", result.Error)
	}

	return result.AppID, nil
}

// DeleteShortcut deletes a shortcut.
func (c *Client) DeleteShortcut(ctx context.Context, userID string, appID uint32) error {
	url := fmt.Sprintf("%s/shortcuts/%s/%d", c.baseURL, userID, appID)

	req, err := http.NewRequestWithContext(ctx, "DELETE", url, nil)
	if err != nil {
		return err
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return fmt.Errorf("delete shortcut failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("delete shortcut returned status %d", resp.StatusCode)
	}

	return nil
}

// ApplyArtworkResult contains the result of applying artwork.
type ApplyArtworkResult struct {
	Applied []string `json:"applied"`
	Failed  []struct {
		Type  string `json:"type"`
		Error string `json:"error,omitempty"`
	} `json:"failed,omitempty"`
}

// ApplyArtwork applies artwork to a shortcut.
func (c *Client) ApplyArtwork(ctx context.Context, userID string, appID uint32, cfg *protocol.ArtworkConfig) (*ApplyArtworkResult, error) {
	url := fmt.Sprintf("%s/shortcuts/%s/%d/artwork", c.baseURL, userID, appID)

	body, err := json.Marshal(cfg)
	if err != nil {
		return nil, err
	}

	req, err := http.NewRequestWithContext(ctx, "POST", url, bytes.NewReader(body))
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("apply artwork failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		var errResp struct {
			Error string `json:"error"`
		}
		json.NewDecoder(resp.Body).Decode(&errResp)
		return nil, fmt.Errorf("apply artwork returned status %d: %s", resp.StatusCode, errResp.Error)
	}

	var result ApplyArtworkResult
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}

	return &result, nil
}

// RestartSteamResult contains the result of a Steam restart.
type RestartSteamResult struct {
	Success bool   `json:"success"`
	Message string `json:"message"`
}

// RestartSteam restarts Steam on the agent.
func (c *Client) RestartSteam(ctx context.Context) (*RestartSteamResult, error) {
	req, err := http.NewRequestWithContext(ctx, "POST", c.baseURL+"/steam/restart", nil)
	if err != nil {
		return nil, err
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("restart steam failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("restart steam returned status %d", resp.StatusCode)
	}

	var result RestartSteamResult
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}

	return &result, nil
}

// DeleteGameResult contains the result of deleting a game.
type DeleteGameResult struct {
	Status         string `json:"status"`
	GameName       string `json:"gameName"`
	SteamRestarted bool   `json:"steamRestarted"`
}

