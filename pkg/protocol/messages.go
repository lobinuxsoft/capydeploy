package protocol

import (
	"encoding/json"
	"time"
)

// WebSocket timing constants.
const (
	// WSWriteWait is the time allowed to write a message.
	WSWriteWait = 30 * time.Second

	// WSPongWait is the time to wait for a pong response.
	WSPongWait = 15 * time.Second

	// WSPingPeriod is how often to send pings (must be < PongWait).
	WSPingPeriod = 5 * time.Second

	// WSMaxMessageSize is the maximum message size in bytes (50MB).
	WSMaxMessageSize = 50 * 1024 * 1024

	// WSChunkSize is the size for binary chunks (1MB).
	WSChunkSize = 1024 * 1024

	// WSRequestTimeout is the timeout for request/response operations.
	WSRequestTimeout = 30 * time.Second
)

// MessageType identifies the type of WebSocket message.
type MessageType string

const (
	// Connection management
	MsgTypeHubConnected MessageType = "hub_connected" // Hub → Agent: handshake
	MsgTypeAgentStatus  MessageType = "agent_status"  // Agent → Hub: handshake response

	// Authentication / Pairing
	MsgTypePairingRequired MessageType = "pairing_required" // Agent → Hub: requires pairing
	MsgTypePairConfirm     MessageType = "pair_confirm"     // Hub → Agent: confirm pairing code
	MsgTypePairSuccess     MessageType = "pair_success"     // Agent → Hub: pairing successful
	MsgTypePairFailed      MessageType = "pair_failed"      // Agent → Hub: pairing failed

	// Requests from Hub to Agent
	MsgTypePing           MessageType = "ping"
	MsgTypeGetInfo        MessageType = "get_info"
	MsgTypeGetConfig      MessageType = "get_config"
	MsgTypeGetSteamUsers  MessageType = "get_steam_users"
	MsgTypeListShortcuts  MessageType = "list_shortcuts"
	MsgTypeCreateShortcut MessageType = "create_shortcut"
	MsgTypeDeleteShortcut MessageType = "delete_shortcut"
	MsgTypeDeleteGame     MessageType = "delete_game" // Agent handles everything internally
	MsgTypeApplyArtwork      MessageType = "apply_artwork"
	MsgTypeSendArtworkImage  MessageType = "send_artwork_image"  // Hub → Agent: binary image data
	MsgTypeRestartSteam      MessageType = "restart_steam"
	MsgTypeInitUpload     MessageType = "init_upload"
	MsgTypeUploadChunk    MessageType = "upload_chunk"
	MsgTypeCompleteUpload MessageType = "complete_upload"
	MsgTypeCancelUpload   MessageType = "cancel_upload"

	// Responses from Agent to Hub
	MsgTypePong              MessageType = "pong"
	MsgTypeInfoResponse      MessageType = "info_response"
	MsgTypeConfigResponse    MessageType = "config_response"
	MsgTypeSteamUsersResponse MessageType = "steam_users_response"
	MsgTypeShortcutsResponse MessageType = "shortcuts_response"
	MsgTypeArtworkResponse        MessageType = "artwork_response"
	MsgTypeArtworkImageResponse   MessageType = "artwork_image_response" // Agent → Hub: ack for binary artwork
	MsgTypeSteamResponse          MessageType = "steam_response"
	MsgTypeUploadInitResponse MessageType = "upload_init_response"
	MsgTypeUploadChunkResponse MessageType = "upload_chunk_response"
	MsgTypeOperationResult   MessageType = "operation_result"
	MsgTypeError             MessageType = "error"

	// Events from Agent to Hub (push notifications)
	MsgTypeUploadProgress MessageType = "upload_progress"
	MsgTypeOperationEvent MessageType = "operation_event"
)

// WSError represents an error in a WebSocket message.
type WSError struct {
	Code    int    `json:"code"`
	Message string `json:"message"`
}

// Message is the envelope for all WebSocket communication.
type Message struct {
	ID      string          `json:"id"`
	Type    MessageType     `json:"type"`
	Payload json.RawMessage `json:"payload,omitempty"`
	Error   *WSError        `json:"error,omitempty"`
}

// NewMessage creates a new message with the given type and payload.
func NewMessage(id string, msgType MessageType, payload any) (*Message, error) {
	var raw json.RawMessage
	if payload != nil {
		data, err := json.Marshal(payload)
		if err != nil {
			return nil, err
		}
		raw = data
	}
	return &Message{ID: id, Type: msgType, Payload: raw}, nil
}

// ParsePayload unmarshals the payload into the given type.
func (m *Message) ParsePayload(v any) error {
	if m.Payload == nil {
		return nil
	}
	return json.Unmarshal(m.Payload, v)
}

// NewErrorMessage creates an error response message.
func NewErrorMessage(id string, code int, message string) *Message {
	return &Message{
		ID:   id,
		Type: MsgTypeError,
		Error: &WSError{
			Code:    code,
			Message: message,
		},
	}
}

// Reply creates a response message for this request.
func (m *Message) Reply(msgType MessageType, payload any) (*Message, error) {
	return NewMessage(m.ID, msgType, payload)
}

// ReplyError creates an error response for this request.
func (m *Message) ReplyError(code int, message string) *Message {
	return NewErrorMessage(m.ID, code, message)
}

// Common WebSocket error codes.
const (
	WSErrCodeBadRequest     = 400
	WSErrCodeUnauthorized   = 401
	WSErrCodeNotFound       = 404
	WSErrCodeNotAccepted    = 406
	WSErrCodeConflict       = 409
	WSErrCodeInternal       = 500
	WSErrCodeNotImplemented = 501
)

// Request payloads

// InitUploadRequest starts a new upload session.
type InitUploadRequest struct {
	Config     UploadConfig `json:"config"`
	TotalSize  int64        `json:"totalSize"`
	FileCount  int          `json:"fileCount"`
	ResumeFrom int64        `json:"resumeFrom,omitempty"`
}

// UploadChunkRequest sends a chunk of data.
type UploadChunkRequest struct {
	UploadID string `json:"uploadId"`
	Offset   int64  `json:"offset"`
	Data     []byte `json:"data"`
	FilePath string `json:"filePath"`
	IsLast   bool   `json:"isLast"`
}

// CompleteUploadRequest finalizes an upload.
type CompleteUploadRequest struct {
	UploadID       string `json:"uploadId"`
	CreateShortcut bool   `json:"createShortcut"`
}

// CancelUploadRequest cancels an active upload.
type CancelUploadRequest struct {
	UploadID string `json:"uploadId"`
}

// CreateShortcutRequest creates a Steam shortcut.
type CreateShortcutRequest struct {
	UserID   uint32         `json:"userId"`
	Shortcut ShortcutConfig `json:"shortcut"`
}

// DeleteShortcutRequest removes a Steam shortcut.
type DeleteShortcutRequest struct {
	UserID  uint32 `json:"userId"`
	AppID   uint32 `json:"appId,omitempty"`
	Name    string `json:"name,omitempty"`
}

// ListShortcutsRequest lists shortcuts for a user.
type ListShortcutsRequest struct {
	UserID uint32 `json:"userId"`
}

// Response payloads

// InfoResponse contains agent information.
type InfoResponse struct {
	Agent AgentInfo `json:"agent"`
}

// InitUploadResponse acknowledges upload initialization.
type InitUploadResponse struct {
	UploadID   string `json:"uploadId"`
	ResumeFrom int64  `json:"resumeFrom"`
}

// UploadChunkResponse acknowledges a chunk.
type UploadChunkResponse struct {
	UploadID    string `json:"uploadId"`
	BytesWritten int64 `json:"bytesWritten"`
	TotalWritten int64 `json:"totalWritten"`
}

// CompleteUploadResponse confirms upload completion.
type CompleteUploadResponse struct {
	UploadID string `json:"uploadId"`
	Success  bool   `json:"success"`
}

// ShortcutResponse contains shortcut operation result.
type ShortcutResponse struct {
	Success   bool           `json:"success"`
	Shortcuts []ShortcutInfo `json:"shortcuts,omitempty"`
}

// SteamStatusResponse contains Steam status.
type SteamStatusResponse struct {
	Running bool   `json:"running"`
	Path    string `json:"path,omitempty"`
}

// ErrorResponse contains error details.
type ErrorResponse struct {
	Code    string `json:"code"`
	Message string `json:"message"`
	Details string `json:"details,omitempty"`
}

// Connection payloads

// HubConnectedRequest is sent when a Hub connects to an Agent.
type HubConnectedRequest struct {
	Name     string `json:"name"`
	Version  string `json:"version"`
	Platform string `json:"platform,omitempty"` // Hub platform (windows, linux, darwin)
	HubID    string `json:"hubId,omitempty"`    // Unique Hub identifier
	Token    string `json:"token,omitempty"`    // Auth token from previous pairing
}

// AgentStatusResponse is the Agent's response to a Hub connection.
type AgentStatusResponse struct {
	Name              string `json:"name"`
	Version           string `json:"version"`
	Platform          string `json:"platform"`
	AcceptConnections bool   `json:"acceptConnections"`
}

// PairingRequiredResponse is sent when a Hub needs to pair.
type PairingRequiredResponse struct {
	Code      string `json:"code"`      // 6-digit pairing code
	ExpiresIn int    `json:"expiresIn"` // Seconds until expiration
}

// PairConfirmRequest is sent by Hub to confirm pairing.
type PairConfirmRequest struct {
	Code string `json:"code"` // 6-digit code entered by user
}

// PairSuccessResponse is sent when pairing is successful.
type PairSuccessResponse struct {
	Token string `json:"token"` // Auth token for future connections
}

// PairFailedResponse is sent when pairing fails.
type PairFailedResponse struct {
	Reason string `json:"reason"` // Failure reason
}

// Config payloads

// ConfigResponse contains agent configuration.
type ConfigResponse struct {
	InstallPath string `json:"installPath"`
}

// Steam payloads

// SteamUser represents a Steam user (matches steam.User).
type SteamUser struct {
	ID          string `json:"id"`
	Name        string `json:"name"`
	AvatarURL   string `json:"avatarUrl,omitempty"`
	LastLoginAt int64  `json:"lastLoginAt,omitempty"`
}

// SteamUsersResponse contains the list of Steam users.
type SteamUsersResponse struct {
	Users []SteamUser `json:"users"`
}

// ShortcutsListResponse contains the list of shortcuts.
type ShortcutsListResponse struct {
	Shortcuts []ShortcutInfo `json:"shortcuts"`
}

// CreateShortcutResponse contains the result of shortcut creation.
type CreateShortcutResponse struct {
	AppID          uint32 `json:"appId"`
	SteamRestarted bool   `json:"steamRestarted,omitempty"`
}

// DeleteShortcutRequest with restart option.
type DeleteShortcutWithRestartRequest struct {
	UserID       string `json:"userId"`
	AppID        uint32 `json:"appId"`
	RestartSteam bool   `json:"restartSteam,omitempty"`
}

// DeleteGameRequest requests deletion of a game. Agent handles everything internally.
type DeleteGameRequest struct {
	AppID uint32 `json:"appId"`
}

// DeleteGameResponse contains the result of game deletion.
type DeleteGameResponse struct {
	Status         string `json:"status"`
	GameName       string `json:"gameName"`
	SteamRestarted bool   `json:"steamRestarted"`
}

// Artwork payloads

// ApplyArtworkRequest requests artwork application.
type ApplyArtworkRequest struct {
	UserID  string         `json:"userId"`
	AppID   uint32         `json:"appId"`
	Artwork *ArtworkConfig `json:"artwork"`
}

// ArtworkResponse contains artwork operation result.
type ArtworkResponse struct {
	Applied []string        `json:"applied"`
	Failed  []ArtworkFailed `json:"failed,omitempty"`
}

// ArtworkFailed represents a failed artwork application.
type ArtworkFailed struct {
	Type  string `json:"type"`
	Error string `json:"error"`
}

// Operation payloads

// OperationResult is a generic result for operations.
type OperationResult struct {
	Success bool   `json:"success"`
	Message string `json:"message,omitempty"`
}

// OperationEvent is a push notification for operation progress.
type OperationEvent struct {
	Type     string  `json:"type"`     // "install", "delete"
	Status   string  `json:"status"`   // "start", "progress", "complete", "error"
	GameName string  `json:"gameName"`
	Progress float64 `json:"progress"` // 0-100
	Message  string  `json:"message,omitempty"`
}

// Upload payloads (extended)

// InitUploadRequestFull includes file entries for the upload.
type InitUploadRequestFull struct {
	Config    UploadConfig `json:"config"`
	TotalSize int64        `json:"totalSize"`
	Files     []FileEntry  `json:"files"`
}

// FileEntry represents a file in the upload manifest.
type FileEntry struct {
	RelativePath string `json:"relativePath"`
	Size         int64  `json:"size"`
}

// InitUploadResponseFull includes chunk size configuration.
type InitUploadResponseFull struct {
	UploadID   string           `json:"uploadId"`
	ChunkSize  int              `json:"chunkSize"`
	ResumeFrom map[string]int64 `json:"resumeFrom,omitempty"`
}

// UploadChunkRequestFull includes all chunk metadata.
type UploadChunkRequestFull struct {
	UploadID string `json:"uploadId"`
	FilePath string `json:"filePath"`
	Offset   int64  `json:"offset"`
	Size     int    `json:"size"`
	Checksum string `json:"checksum,omitempty"`
	// Data is sent as binary message, not in JSON
}

// CompleteUploadRequestFull includes shortcut configuration.
type CompleteUploadRequestFull struct {
	UploadID       string          `json:"uploadId"`
	CreateShortcut bool            `json:"createShortcut"`
	Shortcut       *ShortcutConfig `json:"shortcut,omitempty"`
}

// CompleteUploadResponseFull includes the result path and appID.
type CompleteUploadResponseFull struct {
	Success bool   `json:"success"`
	Path    string `json:"path,omitempty"`
	AppID   uint32 `json:"appId,omitempty"`
}

// UploadProgressEvent is sent during upload to report progress.
type UploadProgressEvent struct {
	UploadID         string  `json:"uploadId"`
	TransferredBytes int64   `json:"transferredBytes"`
	TotalBytes       int64   `json:"totalBytes"`
	CurrentFile      string  `json:"currentFile,omitempty"`
	Percentage       float64 `json:"percentage"`
}

// ArtworkImageResponse contains the result of a binary artwork image transfer.
type ArtworkImageResponse struct {
	Success     bool   `json:"success"`
	ArtworkType string `json:"artworkType"`
	Error       string `json:"error,omitempty"`
}

// Steam control payloads

// RestartSteamResponse contains the result of Steam restart.
type RestartSteamResponse struct {
	Success bool   `json:"success"`
	Message string `json:"message"`
}
