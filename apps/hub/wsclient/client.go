// Package wsclient provides a WebSocket client for communicating with CapyDeploy Agents.
package wsclient

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"log"
	"sync"
	"time"

	"github.com/google/uuid"
	"github.com/gorilla/websocket"
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

// Errors returned by client operations.
var (
	ErrPairingRequired = errors.New("pairing required")
	ErrPairingFailed   = errors.New("pairing failed")
)

// Client is a WebSocket client for communicating with a CapyDeploy Agent.
type Client struct {
	url         string
	hubName     string
	hubVersion  string
	hubPlatform string
	hubID       string
	agentID     string

	mu        sync.RWMutex
	conn      *websocket.Conn
	sendCh    chan []byte
	closeCh   chan struct{}
	closed    bool
	requests  map[string]chan *protocol.Message
	authToken string

	// Token management
	getToken  func(agentID string) string
	saveToken func(agentID, token string) error

	// Callbacks
	onDisconnect      func()
	onUploadProgress  func(event protocol.UploadProgressEvent)
	onOperationEvent  func(event protocol.OperationEvent)
	onPairingRequired func(agentID string)
}

// NewClient creates a new WebSocket client for an Agent.
func NewClient(host string, port int, hubName, hubVersion string) *Client {
	return &Client{
		url:        fmt.Sprintf("ws://%s:%d/ws", host, port),
		hubName:    hubName,
		hubVersion: hubVersion,
		requests:   make(map[string]chan *protocol.Message),
	}
}

// SetAuth configures authentication for this client.
func (c *Client) SetAuth(hubID, agentID string, getToken func(string) string, saveToken func(string, string) error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.hubID = hubID
	c.agentID = agentID
	c.getToken = getToken
	c.saveToken = saveToken
}

// SetPlatform sets the hub platform to be sent during connection.
func (c *Client) SetPlatform(platform string) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.hubPlatform = platform
}

// SetPairingCallback sets the callback for when pairing is required.
func (c *Client) SetPairingCallback(cb func(agentID string)) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.onPairingRequired = cb
}

// SetCallbacks sets the event callbacks.
func (c *Client) SetCallbacks(onDisconnect func(), onUploadProgress func(protocol.UploadProgressEvent), onOperationEvent func(protocol.OperationEvent)) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.onDisconnect = onDisconnect
	c.onUploadProgress = onUploadProgress
	c.onOperationEvent = onOperationEvent
}

// Connect establishes the WebSocket connection to the Agent.
func (c *Client) Connect(ctx context.Context) error {
	c.mu.Lock()
	if c.conn != nil {
		c.mu.Unlock()
		return fmt.Errorf("already connected")
	}
	c.mu.Unlock()

	dialer := websocket.Dialer{
		HandshakeTimeout: 10 * time.Second,
	}

	conn, _, err := dialer.DialContext(ctx, c.url, nil)
	if err != nil {
		return fmt.Errorf("failed to connect: %w", err)
	}

	c.mu.Lock()
	c.conn = conn
	c.sendCh = make(chan []byte, 256)
	c.closeCh = make(chan struct{})
	c.closed = false
	c.requests = make(map[string]chan *protocol.Message)

	// Get existing token if available
	var token string
	if c.getToken != nil && c.agentID != "" {
		token = c.getToken(c.agentID)
	}
	c.authToken = token
	hubID := c.hubID
	hubPlatform := c.hubPlatform
	c.mu.Unlock()

	// Start goroutines
	go c.readPump()
	go c.writePump()

	// Send handshake with auth info
	resp, err := c.sendRequest(ctx, protocol.MsgTypeHubConnected, protocol.HubConnectedRequest{
		Name:     c.hubName,
		Version:  c.hubVersion,
		Platform: hubPlatform,
		HubID:    hubID,
		Token:    token,
	})
	if err != nil {
		c.Close()
		return fmt.Errorf("handshake failed: %w", err)
	}

	// Check response type
	switch resp.Type {
	case protocol.MsgTypeAgentStatus:
		// Successfully authenticated
		log.Printf("WS Client: Connected to %s (authenticated)", c.url)
		return nil

	case protocol.MsgTypePairingRequired:
		// Need to pair
		var pairingResp protocol.PairingRequiredResponse
		if err := resp.ParsePayload(&pairingResp); err != nil {
			c.Close()
			return fmt.Errorf("failed to parse pairing response: %w", err)
		}
		log.Printf("WS Client: Pairing required, code: %s (expires in %ds)", pairingResp.Code, pairingResp.ExpiresIn)

		// Notify callback if set
		c.mu.RLock()
		cb := c.onPairingRequired
		c.mu.RUnlock()
		if cb != nil {
			cb(c.agentID)
		}

		return ErrPairingRequired

	default:
		c.Close()
		return fmt.Errorf("unexpected response type: %s", resp.Type)
	}
}

// ConfirmPairing sends the pairing code to confirm authentication.
func (c *Client) ConfirmPairing(ctx context.Context, code string) error {
	c.mu.RLock()
	if c.closed || c.conn == nil {
		c.mu.RUnlock()
		return fmt.Errorf("not connected")
	}
	c.mu.RUnlock()

	resp, err := c.sendRequest(ctx, protocol.MsgTypePairConfirm, protocol.PairConfirmRequest{
		Code: code,
	})
	if err != nil {
		return fmt.Errorf("pairing request failed: %w", err)
	}

	switch resp.Type {
	case protocol.MsgTypePairSuccess:
		var successResp protocol.PairSuccessResponse
		if err := resp.ParsePayload(&successResp); err != nil {
			return fmt.Errorf("failed to parse pairing success: %w", err)
		}

		// Save token
		c.mu.Lock()
		c.authToken = successResp.Token
		saveToken := c.saveToken
		agentID := c.agentID
		c.mu.Unlock()

		if saveToken != nil && agentID != "" {
			if err := saveToken(agentID, successResp.Token); err != nil {
				log.Printf("WS Client: Warning: failed to save token: %v", err)
			}
		}

		log.Printf("WS Client: Pairing successful")
		return nil

	case protocol.MsgTypePairFailed:
		var failResp protocol.PairFailedResponse
		if err := resp.ParsePayload(&failResp); err != nil {
			return ErrPairingFailed
		}
		return fmt.Errorf("%w: %s", ErrPairingFailed, failResp.Reason)

	default:
		return fmt.Errorf("unexpected response type: %s", resp.Type)
	}
}

// Close closes the WebSocket connection.
func (c *Client) Close() error {
	c.mu.Lock()
	defer c.mu.Unlock()

	if c.closed {
		return nil
	}
	c.closed = true

	if c.closeCh != nil {
		close(c.closeCh)
	}

	var err error
	if c.conn != nil {
		c.conn.WriteMessage(websocket.CloseMessage, websocket.FormatCloseMessage(websocket.CloseNormalClosure, ""))
		err = c.conn.Close()
		c.conn = nil
	}

	return err
}

// IsConnected returns true if the client is connected.
func (c *Client) IsConnected() bool {
	c.mu.RLock()
	defer c.mu.RUnlock()
	return c.conn != nil && !c.closed
}

// readPump handles incoming messages from the Agent.
func (c *Client) readPump() {
	defer c.handleDisconnect()

	c.conn.SetReadLimit(protocol.WSMaxMessageSize)
	c.conn.SetReadDeadline(time.Now().Add(protocol.WSPongWait))
	c.conn.SetPongHandler(func(string) error {
		c.conn.SetReadDeadline(time.Now().Add(protocol.WSPongWait))
		return nil
	})

	for {
		messageType, data, err := c.conn.ReadMessage()
		if err != nil {
			if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseAbnormalClosure) {
				log.Printf("WS Client: Read error: %v", err)
			}
			return
		}

		switch messageType {
		case websocket.TextMessage:
			c.handleTextMessage(data)
		case websocket.BinaryMessage:
			// Binary messages not expected from Agent to Hub
			log.Printf("WS Client: Unexpected binary message")
		}
	}
}

// writePump handles outgoing messages to the Agent.
func (c *Client) writePump() {
	ticker := time.NewTicker(protocol.WSPingPeriod)
	defer func() {
		ticker.Stop()
		c.mu.RLock()
		conn := c.conn
		c.mu.RUnlock()
		if conn != nil {
			conn.Close()
		}
	}()

	for {
		select {
		case message, ok := <-c.sendCh:
			c.mu.RLock()
			conn := c.conn
			c.mu.RUnlock()
			if conn == nil {
				return
			}
			conn.SetWriteDeadline(time.Now().Add(protocol.WSWriteWait))
			if !ok {
				conn.WriteMessage(websocket.CloseMessage, []byte{})
				return
			}

			if err := conn.WriteMessage(websocket.TextMessage, message); err != nil {
				log.Printf("WS Client: Write error: %v", err)
				return
			}

		case <-ticker.C:
			c.mu.RLock()
			conn := c.conn
			c.mu.RUnlock()
			if conn == nil {
				return
			}
			conn.SetWriteDeadline(time.Now().Add(protocol.WSWriteWait))
			if err := conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				return
			}

		case <-c.closeCh:
			return
		}
	}
}

// handleTextMessage processes JSON messages from the Agent.
func (c *Client) handleTextMessage(data []byte) {
	var msg protocol.Message
	if err := json.Unmarshal(data, &msg); err != nil {
		log.Printf("WS Client: Invalid message: %v", err)
		return
	}

	// Check if this is a response to a pending request
	c.mu.RLock()
	respCh, isResponse := c.requests[msg.ID]
	c.mu.RUnlock()

	if isResponse {
		select {
		case respCh <- &msg:
		default:
			log.Printf("WS Client: Response channel full for %s", msg.ID)
		}
		return
	}

	// Handle push events
	switch msg.Type {
	case protocol.MsgTypeUploadProgress:
		var event protocol.UploadProgressEvent
		if err := msg.ParsePayload(&event); err == nil {
			c.mu.RLock()
			callback := c.onUploadProgress
			c.mu.RUnlock()
			if callback != nil {
				callback(event)
			}
		}
	case protocol.MsgTypeOperationEvent:
		var event protocol.OperationEvent
		if err := msg.ParsePayload(&event); err == nil {
			c.mu.RLock()
			callback := c.onOperationEvent
			c.mu.RUnlock()
			if callback != nil {
				callback(event)
			}
		}
	default:
		log.Printf("WS Client: Unhandled message type: %s", msg.Type)
	}
}

// handleDisconnect handles connection loss.
func (c *Client) handleDisconnect() {
	c.mu.Lock()
	c.conn = nil

	// Cancel all pending requests
	for id, ch := range c.requests {
		close(ch)
		delete(c.requests, id)
	}

	callback := c.onDisconnect
	c.mu.Unlock()

	log.Printf("WS Client: Disconnected")
	if callback != nil {
		callback()
	}
}

// sendRequest sends a request and waits for a response.
func (c *Client) sendRequest(ctx context.Context, msgType protocol.MessageType, payload any) (*protocol.Message, error) {
	c.mu.RLock()
	if c.closed || c.conn == nil {
		c.mu.RUnlock()
		return nil, fmt.Errorf("not connected")
	}
	c.mu.RUnlock()

	// Create message
	id := uuid.New().String()
	msg, err := protocol.NewMessage(id, msgType, payload)
	if err != nil {
		return nil, fmt.Errorf("failed to create message: %w", err)
	}

	// Create response channel
	respCh := make(chan *protocol.Message, 1)
	c.mu.Lock()
	c.requests[id] = respCh
	c.mu.Unlock()

	defer func() {
		c.mu.Lock()
		delete(c.requests, id)
		c.mu.Unlock()
	}()

	// Send message
	data, err := json.Marshal(msg)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal message: %w", err)
	}

	select {
	case c.sendCh <- data:
	case <-ctx.Done():
		return nil, ctx.Err()
	default:
		return nil, fmt.Errorf("send buffer full")
	}

	// Wait for response
	timer := time.NewTimer(protocol.WSRequestTimeout)
	defer timer.Stop()

	select {
	case resp, ok := <-respCh:
		if !ok {
			return nil, fmt.Errorf("connection closed")
		}
		if resp.Error != nil {
			return nil, fmt.Errorf("agent error (%d): %s", resp.Error.Code, resp.Error.Message)
		}
		return resp, nil
	case <-ctx.Done():
		return nil, ctx.Err()
	case <-timer.C:
		return nil, fmt.Errorf("request timeout")
	}
}

// sendRequestWithBinary sends a request with binary data.
func (c *Client) sendBinaryChunk(ctx context.Context, msgID, uploadID, filePath string, offset int64, checksum string, data []byte) error {
	c.mu.RLock()
	if c.closed || c.conn == nil {
		c.mu.RUnlock()
		return fmt.Errorf("not connected")
	}
	conn := c.conn
	c.mu.RUnlock()

	// Create header
	header := struct {
		ID       string `json:"id"`
		UploadID string `json:"uploadId"`
		FilePath string `json:"filePath"`
		Offset   int64  `json:"offset"`
		Checksum string `json:"checksum,omitempty"`
	}{
		ID:       msgID,
		UploadID: uploadID,
		FilePath: filePath,
		Offset:   offset,
		Checksum: checksum,
	}

	return c.sendBinary(conn, header, data)
}

// buildBinaryMessage encodes a header+data pair into the wire format:
// [4 bytes big-endian header length][JSON header][binary data].
func buildBinaryMessage(header any, data []byte) ([]byte, error) {
	headerBytes, err := json.Marshal(header)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal header: %w", err)
	}

	headerLen := len(headerBytes)
	message := make([]byte, 4+headerLen+len(data))
	message[0] = byte(headerLen >> 24)
	message[1] = byte(headerLen >> 16)
	message[2] = byte(headerLen >> 8)
	message[3] = byte(headerLen)
	copy(message[4:], headerBytes)
	copy(message[4+headerLen:], data)

	return message, nil
}

// sendBinary builds a binary message from header+data and writes it to the WS connection.
func (c *Client) sendBinary(conn *websocket.Conn, header any, data []byte) error {
	message, err := buildBinaryMessage(header, data)
	if err != nil {
		return err
	}

	conn.SetWriteDeadline(time.Now().Add(protocol.WSWriteWait))
	return conn.WriteMessage(websocket.BinaryMessage, message)
}

// API methods

// GetInfo returns agent information.
func (c *Client) GetInfo(ctx context.Context) (*protocol.AgentInfo, error) {
	resp, err := c.sendRequest(ctx, protocol.MsgTypeGetInfo, nil)
	if err != nil {
		return nil, err
	}

	var info protocol.InfoResponse
	if err := resp.ParsePayload(&info); err != nil {
		return nil, fmt.Errorf("failed to parse response: %w", err)
	}

	return &info.Agent, nil
}

// GetConfig returns agent configuration.
func (c *Client) GetConfig(ctx context.Context) (*protocol.ConfigResponse, error) {
	resp, err := c.sendRequest(ctx, protocol.MsgTypeGetConfig, nil)
	if err != nil {
		return nil, err
	}

	var config protocol.ConfigResponse
	if err := resp.ParsePayload(&config); err != nil {
		return nil, fmt.Errorf("failed to parse response: %w", err)
	}

	return &config, nil
}

// GetSteamUsers returns Steam users on the agent.
func (c *Client) GetSteamUsers(ctx context.Context) ([]protocol.SteamUser, error) {
	resp, err := c.sendRequest(ctx, protocol.MsgTypeGetSteamUsers, nil)
	if err != nil {
		return nil, err
	}

	var result protocol.SteamUsersResponse
	if err := resp.ParsePayload(&result); err != nil {
		return nil, fmt.Errorf("failed to parse response: %w", err)
	}

	return result.Users, nil
}

// ListShortcuts returns shortcuts for a Steam user.
func (c *Client) ListShortcuts(ctx context.Context, userID uint32) ([]protocol.ShortcutInfo, error) {
	resp, err := c.sendRequest(ctx, protocol.MsgTypeListShortcuts, protocol.ListShortcutsRequest{
		UserID: userID,
	})
	if err != nil {
		return nil, err
	}

	var result protocol.ShortcutsListResponse
	if err := resp.ParsePayload(&result); err != nil {
		return nil, fmt.Errorf("failed to parse response: %w", err)
	}

	return result.Shortcuts, nil
}

// CreateShortcut creates a new shortcut.
func (c *Client) CreateShortcut(ctx context.Context, userID uint32, shortcut protocol.ShortcutConfig) (uint32, error) {
	resp, err := c.sendRequest(ctx, protocol.MsgTypeCreateShortcut, protocol.CreateShortcutRequest{
		UserID:   userID,
		Shortcut: shortcut,
	})
	if err != nil {
		return 0, err
	}

	var result protocol.CreateShortcutResponse
	if err := resp.ParsePayload(&result); err != nil {
		return 0, fmt.Errorf("failed to parse response: %w", err)
	}

	return result.AppID, nil
}

// DeleteShortcut deletes a shortcut.
func (c *Client) DeleteShortcut(ctx context.Context, userID string, appID uint32, restartSteam bool) error {
	_, err := c.sendRequest(ctx, protocol.MsgTypeDeleteShortcut, protocol.DeleteShortcutWithRestartRequest{
		UserID:       userID,
		AppID:        appID,
		RestartSteam: restartSteam,
	})
	return err
}

// ApplyArtwork applies artwork to a shortcut.
func (c *Client) ApplyArtwork(ctx context.Context, userID string, appID uint32, artwork *protocol.ArtworkConfig) (*protocol.ArtworkResponse, error) {
	resp, err := c.sendRequest(ctx, protocol.MsgTypeApplyArtwork, protocol.ApplyArtworkRequest{
		UserID:  userID,
		AppID:   appID,
		Artwork: artwork,
	})
	if err != nil {
		return nil, err
	}

	var result protocol.ArtworkResponse
	if err := resp.ParsePayload(&result); err != nil {
		return nil, fmt.Errorf("failed to parse response: %w", err)
	}

	return &result, nil
}

// SendArtworkImage sends a binary artwork image to the agent.
func (c *Client) SendArtworkImage(ctx context.Context, appID uint32, artworkType, contentType string, data []byte) error {
	msgID := uuid.New().String()

	// Create response channel
	respCh := make(chan *protocol.Message, 1)
	c.mu.Lock()
	c.requests[msgID] = respCh
	c.mu.Unlock()

	defer func() {
		c.mu.Lock()
		delete(c.requests, msgID)
		c.mu.Unlock()
	}()

	// Send binary message
	if err := c.sendBinaryArtwork(ctx, msgID, appID, artworkType, contentType, data); err != nil {
		return err
	}

	// Wait for response
	artTimer := time.NewTimer(protocol.WSRequestTimeout)
	defer artTimer.Stop()

	select {
	case resp, ok := <-respCh:
		if !ok {
			return fmt.Errorf("connection closed")
		}
		if resp.Error != nil {
			return fmt.Errorf("agent error (%d): %s", resp.Error.Code, resp.Error.Message)
		}
		var result protocol.ArtworkImageResponse
		if err := resp.ParsePayload(&result); err != nil {
			return fmt.Errorf("failed to parse response: %w", err)
		}
		if !result.Success {
			return fmt.Errorf("artwork apply failed: %s", result.Error)
		}
		return nil
	case <-ctx.Done():
		return ctx.Err()
	case <-artTimer.C:
		return fmt.Errorf("artwork image upload timeout")
	}
}

// sendBinaryArtwork sends a binary artwork image message.
func (c *Client) sendBinaryArtwork(ctx context.Context, msgID string, appID uint32, artworkType, contentType string, data []byte) error {
	c.mu.RLock()
	if c.closed || c.conn == nil {
		c.mu.RUnlock()
		return fmt.Errorf("not connected")
	}
	conn := c.conn
	c.mu.RUnlock()

	header := struct {
		ID          string `json:"id"`
		Type        string `json:"type"`
		AppID       uint32 `json:"appId"`
		ArtworkType string `json:"artworkType"`
		ContentType string `json:"contentType"`
	}{
		ID:          msgID,
		Type:        "artwork_image",
		AppID:       appID,
		ArtworkType: artworkType,
		ContentType: contentType,
	}

	return c.sendBinary(conn, header, data)
}

// RestartSteam restarts Steam on the agent.
func (c *Client) RestartSteam(ctx context.Context) (*protocol.RestartSteamResponse, error) {
	resp, err := c.sendRequest(ctx, protocol.MsgTypeRestartSteam, nil)
	if err != nil {
		return nil, err
	}

	var result protocol.RestartSteamResponse
	if err := resp.ParsePayload(&result); err != nil {
		return nil, fmt.Errorf("failed to parse response: %w", err)
	}

	return &result, nil
}

// InitUpload initializes an upload session.
func (c *Client) InitUpload(ctx context.Context, config protocol.UploadConfig, totalSize int64, files []protocol.FileEntry) (*protocol.InitUploadResponseFull, error) {
	resp, err := c.sendRequest(ctx, protocol.MsgTypeInitUpload, protocol.InitUploadRequestFull{
		Config:    config,
		TotalSize: totalSize,
		Files:     files,
	})
	if err != nil {
		return nil, err
	}

	var result protocol.InitUploadResponseFull
	if err := resp.ParsePayload(&result); err != nil {
		return nil, fmt.Errorf("failed to parse response: %w", err)
	}

	return &result, nil
}

// UploadChunk uploads a chunk of data.
func (c *Client) UploadChunk(ctx context.Context, uploadID, filePath string, offset int64, data []byte, checksum string) error {
	msgID := uuid.New().String()

	// Create response channel
	respCh := make(chan *protocol.Message, 1)
	c.mu.Lock()
	c.requests[msgID] = respCh
	c.mu.Unlock()

	defer func() {
		c.mu.Lock()
		delete(c.requests, msgID)
		c.mu.Unlock()
	}()

	// Send binary chunk
	if err := c.sendBinaryChunk(ctx, msgID, uploadID, filePath, offset, checksum, data); err != nil {
		return err
	}

	// Wait for response
	chunkTimer := time.NewTimer(protocol.WSRequestTimeout)
	defer chunkTimer.Stop()

	select {
	case resp, ok := <-respCh:
		if !ok {
			return fmt.Errorf("connection closed")
		}
		if resp.Error != nil {
			return fmt.Errorf("agent error (%d): %s", resp.Error.Code, resp.Error.Message)
		}
		return nil
	case <-ctx.Done():
		return ctx.Err()
	case <-chunkTimer.C:
		return fmt.Errorf("chunk upload timeout")
	}
}

// CompleteUpload completes an upload session.
func (c *Client) CompleteUpload(ctx context.Context, uploadID string, createShortcut bool, shortcut *protocol.ShortcutConfig) (*protocol.CompleteUploadResponseFull, error) {
	resp, err := c.sendRequest(ctx, protocol.MsgTypeCompleteUpload, protocol.CompleteUploadRequestFull{
		UploadID:       uploadID,
		CreateShortcut: createShortcut,
		Shortcut:       shortcut,
	})
	if err != nil {
		return nil, err
	}

	var result protocol.CompleteUploadResponseFull
	if err := resp.ParsePayload(&result); err != nil {
		return nil, fmt.Errorf("failed to parse response: %w", err)
	}

	return &result, nil
}

// CancelUpload cancels an upload session.
func (c *Client) CancelUpload(ctx context.Context, uploadID string) error {
	_, err := c.sendRequest(ctx, protocol.MsgTypeCancelUpload, protocol.CancelUploadRequest{
		UploadID: uploadID,
	})
	return err
}

// DeleteGame deletes a game completely. Agent handles everything internally.
func (c *Client) DeleteGame(ctx context.Context, appID uint32) (*protocol.DeleteGameResponse, error) {
	resp, err := c.sendRequest(ctx, protocol.MsgTypeDeleteGame, protocol.DeleteGameRequest{
		AppID: appID,
	})
	if err != nil {
		return nil, err
	}

	var result protocol.DeleteGameResponse
	if err := resp.ParsePayload(&result); err != nil {
		return nil, fmt.Errorf("failed to parse response: %w", err)
	}

	return &result, nil
}
