package protocol

import (
	"encoding/json"
	"testing"
)

func TestNewMessage(t *testing.T) {
	tests := []struct {
		name    string
		id      string
		msgType MessageType
		payload any
		wantErr bool
	}{
		{
			name:    "simple ping message",
			id:      "msg-1",
			msgType: MsgTypePing,
			payload: nil,
			wantErr: false,
		},
		{
			name:    "message with payload",
			id:      "msg-2",
			msgType: MsgTypeGetInfo,
			payload: map[string]string{"key": "value"},
			wantErr: false,
		},
		{
			name:    "init upload request",
			id:      "msg-3",
			msgType: MsgTypeInitUpload,
			payload: InitUploadRequest{
				Config:    UploadConfig{GameName: "Test"},
				TotalSize: 1024,
				FileCount: 5,
			},
			wantErr: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			msg, err := NewMessage(tt.id, tt.msgType, tt.payload)
			if (err != nil) != tt.wantErr {
				t.Errorf("NewMessage() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if msg == nil {
				t.Fatal("NewMessage() returned nil")
			}
			if msg.ID != tt.id {
				t.Errorf("Message.ID = %q, want %q", msg.ID, tt.id)
			}
			if msg.Type != tt.msgType {
				t.Errorf("Message.Type = %q, want %q", msg.Type, tt.msgType)
			}
		})
	}
}

func TestMessage_ParsePayload(t *testing.T) {
	// Create a message with a known payload
	original := InitUploadRequest{
		Config: UploadConfig{
			GameName:   "Test Game",
			InstallPath: "/games/test",
		},
		TotalSize: 2048,
		FileCount: 10,
	}

	msg, err := NewMessage("test-id", MsgTypeInitUpload, original)
	if err != nil {
		t.Fatalf("NewMessage() error = %v", err)
	}

	// Parse the payload back
	var parsed InitUploadRequest
	if err := msg.ParsePayload(&parsed); err != nil {
		t.Fatalf("ParsePayload() error = %v", err)
	}

	if parsed.Config.GameName != original.Config.GameName {
		t.Errorf("GameName = %q, want %q", parsed.Config.GameName, original.Config.GameName)
	}
	if parsed.TotalSize != original.TotalSize {
		t.Errorf("TotalSize = %d, want %d", parsed.TotalSize, original.TotalSize)
	}
}

func TestMessage_ParsePayload_NilPayload(t *testing.T) {
	msg := &Message{
		ID:      "test",
		Type:    MsgTypePing,
		Payload: nil,
	}

	var result map[string]string
	if err := msg.ParsePayload(&result); err != nil {
		t.Errorf("ParsePayload() with nil payload should not error, got %v", err)
	}
}

func TestMessageType_Constants(t *testing.T) {
	// Verify request message types
	requestTypes := []MessageType{
		MsgTypeHubConnected,
		MsgTypePing,
		MsgTypeGetInfo,
		MsgTypeGetConfig,
		MsgTypeGetSteamUsers,
		MsgTypeListShortcuts,
		MsgTypeCreateShortcut,
		MsgTypeDeleteShortcut,
		MsgTypeApplyArtwork,
		MsgTypeRestartSteam,
		MsgTypeInitUpload,
		MsgTypeUploadChunk,
		MsgTypeCompleteUpload,
		MsgTypeCancelUpload,
	}

	for _, mt := range requestTypes {
		if mt == "" {
			t.Error("Request MessageType should not be empty")
		}
	}

	// Verify response message types
	responseTypes := []MessageType{
		MsgTypeAgentStatus,
		MsgTypePong,
		MsgTypeInfoResponse,
		MsgTypeConfigResponse,
		MsgTypeSteamUsersResponse,
		MsgTypeShortcutsResponse,
		MsgTypeArtworkResponse,
		MsgTypeSteamResponse,
		MsgTypeUploadInitResponse,
		MsgTypeUploadChunkResponse,
		MsgTypeOperationResult,
		MsgTypeError,
		MsgTypeUploadProgress,
		MsgTypeOperationEvent,
	}

	for _, mt := range responseTypes {
		if mt == "" {
			t.Error("Response MessageType should not be empty")
		}
	}
}

func TestUploadChunkRequest_Serialization(t *testing.T) {
	req := UploadChunkRequest{
		UploadID: "upload-123",
		Offset:   1024,
		Data:     []byte("test data"),
		FilePath: "game/data.bin",
		IsLast:   false,
	}

	data, err := json.Marshal(req)
	if err != nil {
		t.Fatalf("Marshal() error = %v", err)
	}

	var parsed UploadChunkRequest
	if err := json.Unmarshal(data, &parsed); err != nil {
		t.Fatalf("Unmarshal() error = %v", err)
	}

	if parsed.UploadID != req.UploadID {
		t.Errorf("UploadID = %q, want %q", parsed.UploadID, req.UploadID)
	}
	if parsed.Offset != req.Offset {
		t.Errorf("Offset = %d, want %d", parsed.Offset, req.Offset)
	}
	if string(parsed.Data) != string(req.Data) {
		t.Errorf("Data = %q, want %q", parsed.Data, req.Data)
	}
}

func TestErrorResponse_Fields(t *testing.T) {
	resp := ErrorResponse{
		Code:    ErrCodeUploadFailed,
		Message: "upload failed",
		Details: "disk full",
	}

	if resp.Code != ErrCodeUploadFailed {
		t.Errorf("Code = %q, want %q", resp.Code, ErrCodeUploadFailed)
	}
	if resp.Message != "upload failed" {
		t.Errorf("Message = %q, want %q", resp.Message, "upload failed")
	}
	if resp.Details != "disk full" {
		t.Errorf("Details = %q, want %q", resp.Details, "disk full")
	}
}

func TestCreateShortcutRequest_Serialization(t *testing.T) {
	req := CreateShortcutRequest{
		UserID: 12345,
		Shortcut: ShortcutConfig{
			Name:     "Test Game",
			Exe:      "/path/to/game",
			StartDir: "/path/to",
			Tags:     []string{"action"},
		},
	}

	data, err := json.Marshal(req)
	if err != nil {
		t.Fatalf("Marshal() error = %v", err)
	}

	var parsed CreateShortcutRequest
	if err := json.Unmarshal(data, &parsed); err != nil {
		t.Fatalf("Unmarshal() error = %v", err)
	}

	if parsed.UserID != req.UserID {
		t.Errorf("UserID = %d, want %d", parsed.UserID, req.UserID)
	}
	if parsed.Shortcut.Name != req.Shortcut.Name {
		t.Errorf("Shortcut.Name = %q, want %q", parsed.Shortcut.Name, req.Shortcut.Name)
	}
}
