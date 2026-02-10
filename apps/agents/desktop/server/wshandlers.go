package server

import (
	"log"
	"os"
	"path/filepath"
	"runtime"
	"strconv"

	"github.com/lobinuxsoft/capydeploy/apps/agents/desktop/artwork"
	"github.com/lobinuxsoft/capydeploy/apps/agents/desktop/shortcuts"
	agentSteam "github.com/lobinuxsoft/capydeploy/apps/agents/desktop/steam"
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/steam"
	"github.com/lobinuxsoft/capydeploy/pkg/transfer"
)

// handleHubConnected processes the hub_connected handshake.
func (ws *WSServer) handleHubConnected(hub *HubConnection, msg *protocol.Message) {
	var req protocol.HubConnectedRequest
	if err := msg.ParsePayload(&req); err != nil {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeBadRequest, "invalid payload")
		return
	}

	hub.name = req.Name
	hub.version = req.Version
	hub.hubID = req.HubID
	log.Printf("WS: Hub connected: %s v%s (ID: %s)", req.Name, req.Version, req.HubID)

	// If no auth manager, allow all connections (backwards compatibility)
	if ws.authMgr == nil {
		ws.acceptHub(hub, msg)
		return
	}

	// If Hub provided a token, validate it
	if req.Token != "" && req.HubID != "" {
		if ws.authMgr.ValidateToken(req.HubID, req.Token) {
			log.Printf("WS: Hub %s authenticated with valid token", req.Name)
			ws.acceptHub(hub, msg)
			return
		}
		log.Printf("WS: Hub %s provided invalid token, requiring pairing", req.Name)
	}

	// Hub not authorized - require pairing
	if req.HubID == "" {
		// Hub without ID cannot pair (old client)
		ws.sendError(hub, msg.ID, protocol.WSErrCodeUnauthorized, "hub_id required for pairing")
		return
	}

	// Generate pairing code
	code, err := ws.authMgr.GenerateCode(req.HubID, req.Name, req.Platform)
	if err != nil {
		log.Printf("WS: Failed to generate pairing code: %v", err)
		ws.sendError(hub, msg.ID, protocol.WSErrCodeInternal, err.Error())
		return
	}

	log.Printf("WS: Pairing required for Hub %s, code: %s", req.Name, code)

	// Send pairing required response
	resp, _ := msg.Reply(protocol.MsgTypePairingRequired, protocol.PairingRequiredResponse{
		Code:      code,
		ExpiresIn: 60,
	})
	ws.send(hub, resp)
}

// acceptHub completes the handshake for an authorized Hub.
func (ws *WSServer) acceptHub(hub *HubConnection, msg *protocol.Message) {
	hub.authorized = true

	// Build agent status with telemetry info
	info := ws.server.GetInfo()
	statusResp := protocol.AgentStatusResponse{
		Name:              info.Name,
		Version:           info.Version,
		Platform:          info.Platform,
		AcceptConnections: info.AcceptConnections,
	}
	if ws.server.cfg.GetTelemetryEnabled != nil {
		statusResp.TelemetryEnabled = ws.server.cfg.GetTelemetryEnabled()
	}
	if ws.server.cfg.GetTelemetryInterval != nil {
		statusResp.TelemetryInterval = ws.server.cfg.GetTelemetryInterval()
	}

	resp, _ := msg.Reply(protocol.MsgTypeAgentStatus, statusResp)
	ws.send(hub, resp)

	// Notify callback
	if ws.onConnect != nil {
		ws.onConnect(hub.hubID, hub.name, hub.remoteAddr)
	}

	// Start telemetry if enabled
	ws.server.StartTelemetry()
}

// handlePairConfirm processes a pairing confirmation from the Hub.
func (ws *WSServer) handlePairConfirm(hub *HubConnection, msg *protocol.Message) {
	var req protocol.PairConfirmRequest
	if err := msg.ParsePayload(&req); err != nil {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeBadRequest, "invalid payload")
		return
	}

	if ws.authMgr == nil {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeInternal, "auth not configured")
		return
	}

	// Validate the pairing code
	token, err := ws.authMgr.ValidateCode(hub.hubID, hub.name, req.Code)
	if err != nil {
		log.Printf("WS: Pairing failed for Hub %s: %v", hub.name, err)
		resp, _ := msg.Reply(protocol.MsgTypePairFailed, protocol.PairFailedResponse{
			Reason: err.Error(),
		})
		ws.send(hub, resp)
		return
	}

	log.Printf("WS: Pairing successful for Hub %s", hub.name)

	// Send success with token
	resp, _ := msg.Reply(protocol.MsgTypePairSuccess, protocol.PairSuccessResponse{
		Token: token,
	})
	ws.send(hub, resp)

	// Mark hub as authorized and complete handshake
	hub.authorized = true

	// Notify pairing success callback (for UI update)
	if ws.server.cfg.OnPairingSuccess != nil {
		ws.server.cfg.OnPairingSuccess()
	}

	if ws.onConnect != nil {
		ws.onConnect(hub.hubID, hub.name, hub.remoteAddr)
	}
}

// handlePing responds to ping with pong.
func (ws *WSServer) handlePing(hub *HubConnection, msg *protocol.Message) {
	resp, _ := msg.Reply(protocol.MsgTypePong, nil)
	ws.send(hub, resp)
}

// handleGetInfo returns agent information.
func (ws *WSServer) handleGetInfo(hub *HubConnection, msg *protocol.Message) {
	info := ws.server.GetInfo()
	resp, _ := msg.Reply(protocol.MsgTypeInfoResponse, protocol.InfoResponse{Agent: info})
	ws.send(hub, resp)
}

// handleGetConfig returns agent configuration.
func (ws *WSServer) handleGetConfig(hub *HubConnection, msg *protocol.Message) {
	resp, _ := msg.Reply(protocol.MsgTypeConfigResponse, protocol.ConfigResponse{
		InstallPath: ws.server.GetInstallPath(),
	})
	ws.send(hub, resp)
}

// handleGetSteamUsers returns the list of Steam users.
func (ws *WSServer) handleGetSteamUsers(hub *HubConnection, msg *protocol.Message) {
	users, err := steam.GetUsers()
	if err != nil {
		log.Printf("WS: Error getting Steam users: %v", err)
		ws.sendError(hub, msg.ID, protocol.WSErrCodeInternal, err.Error())
		return
	}

	// Convert to protocol type
	protoUsers := make([]protocol.SteamUser, len(users))
	for i, u := range users {
		protoUsers[i] = protocol.SteamUser{
			ID: u.ID,
			// Name not available from steam.User, client should resolve if needed
		}
	}

	resp, _ := msg.Reply(protocol.MsgTypeSteamUsersResponse, protocol.SteamUsersResponse{
		Users: protoUsers,
	})
	ws.send(hub, resp)
}

// handleListShortcuts returns shortcuts for a user.
func (ws *WSServer) handleListShortcuts(hub *HubConnection, msg *protocol.Message) {
	var req protocol.ListShortcutsRequest
	if err := msg.ParsePayload(&req); err != nil {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeBadRequest, "invalid payload")
		return
	}

	mgr, err := shortcuts.NewManager()
	if err != nil {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeInternal, err.Error())
		return
	}

	userID := strconv.FormatUint(uint64(req.UserID), 10)
	list, err := mgr.List(userID)
	if err != nil {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeInternal, err.Error())
		return
	}

	resp, _ := msg.Reply(protocol.MsgTypeShortcutsResponse, protocol.ShortcutsListResponse{
		Shortcuts: list,
	})
	ws.send(hub, resp)
}

// handleCreateShortcut creates a new shortcut.
func (ws *WSServer) handleCreateShortcut(hub *HubConnection, msg *protocol.Message) {
	var req protocol.CreateShortcutRequest
	if err := msg.ParsePayload(&req); err != nil {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeBadRequest, "invalid payload")
		return
	}

	mgr, err := shortcuts.NewManager()
	if err != nil {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeInternal, err.Error())
		return
	}

	userID := strconv.FormatUint(uint64(req.UserID), 10)
	appID, err := mgr.Create(userID, req.Shortcut)
	if err != nil {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeInternal, err.Error())
		return
	}

	log.Printf("WS: Created shortcut '%s' with AppID %d for user %s", req.Shortcut.Name, appID, userID)
	ws.server.NotifyShortcutChange()

	resp, _ := msg.Reply(protocol.MsgTypeOperationResult, protocol.CreateShortcutResponse{
		AppID: appID,
	})
	ws.send(hub, resp)
}

// handleDeleteShortcut deletes a shortcut.
func (ws *WSServer) handleDeleteShortcut(hub *HubConnection, msg *protocol.Message) {
	var req protocol.DeleteShortcutWithRestartRequest
	if err := msg.ParsePayload(&req); err != nil {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeBadRequest, "invalid payload")
		return
	}

	mgr, err := shortcuts.NewManager()
	if err != nil {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeInternal, err.Error())
		return
	}

	// Get shortcut info before deleting (for notification)
	list, _ := mgr.List(req.UserID)
	var gameName string
	for _, sc := range list {
		if sc.AppID == req.AppID {
			gameName = sc.Name
			break
		}
	}

	// Notify UI about delete start
	ws.server.NotifyOperation("delete", "start", gameName, 0, "Eliminando...")
	ws.SendEvent(protocol.MsgTypeOperationEvent, protocol.OperationEvent{
		Type:     "delete",
		Status:   "start",
		GameName: gameName,
		Progress: 0,
		Message:  "Eliminando...",
	})

	if err := mgr.Delete(req.UserID, req.AppID); err != nil {
		ws.server.NotifyOperation("delete", "error", gameName, 0, err.Error())
		ws.sendError(hub, msg.ID, protocol.WSErrCodeInternal, err.Error())
		return
	}

	log.Printf("WS: Deleted shortcut with AppID %d for user %s", req.AppID, req.UserID)
	ws.server.NotifyShortcutChange()

	// Notify UI about delete complete
	ws.server.NotifyOperation("delete", "complete", gameName, 100, "Eliminado")
	ws.SendEvent(protocol.MsgTypeOperationEvent, protocol.OperationEvent{
		Type:     "delete",
		Status:   "complete",
		GameName: gameName,
		Progress: 100,
		Message:  "Eliminado",
	})

	resp, _ := msg.Reply(protocol.MsgTypeOperationResult, protocol.OperationResult{
		Success: true,
		Message: "deleted",
	})
	ws.send(hub, resp)
}

// handleDeleteGame deletes a game completely. Agent handles everything internally.
func (ws *WSServer) handleDeleteGame(hub *HubConnection, msg *protocol.Message) {
	var req protocol.DeleteGameRequest
	if err := msg.ParsePayload(&req); err != nil {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeBadRequest, "invalid payload")
		return
	}

	// Get Steam users internally - Hub doesn't need to know about this
	users, err := steam.GetUsers()
	if err != nil || len(users) == 0 {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeInternal, "no Steam users found")
		return
	}
	userID := users[0].ID

	mgr, err := shortcuts.NewManager()
	if err != nil {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeInternal, err.Error())
		return
	}

	// Get shortcut info before deleting (for notification)
	list, _ := mgr.List(userID)
	var gameName string
	for _, sc := range list {
		if sc.AppID == req.AppID {
			gameName = sc.Name
			break
		}
	}

	if gameName == "" {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeNotFound, "game not found")
		return
	}

	// Notify UI about delete start
	ws.server.NotifyOperation("delete", "start", gameName, 0, "Eliminando...")
	ws.SendEvent(protocol.MsgTypeOperationEvent, protocol.OperationEvent{
		Type:     "delete",
		Status:   "start",
		GameName: gameName,
		Progress: 0,
		Message:  "Eliminando...",
	})

	// Delete shortcut (this also deletes game folder and artwork)
	if err := mgr.Delete(userID, req.AppID); err != nil {
		ws.server.NotifyOperation("delete", "error", gameName, 0, err.Error())
		ws.sendError(hub, msg.ID, protocol.WSErrCodeInternal, err.Error())
		return
	}

	log.Printf("WS: Deleted game '%s' (AppID: %d) for user %s", gameName, req.AppID, userID)
	ws.server.NotifyShortcutChange()

	// Notify UI about delete complete
	ws.server.NotifyOperation("delete", "complete", gameName, 100, "Eliminado")
	ws.SendEvent(protocol.MsgTypeOperationEvent, protocol.OperationEvent{
		Type:     "delete",
		Status:   "complete",
		GameName: gameName,
		Progress: 100,
		Message:  "Eliminado",
	})

	resp, _ := msg.Reply(protocol.MsgTypeOperationResult, protocol.DeleteGameResponse{
		Status:         "deleted",
		GameName:       gameName,
		SteamRestarted: false,
	})
	ws.send(hub, resp)
}

// handleApplyArtwork applies artwork to a shortcut.
func (ws *WSServer) handleApplyArtwork(hub *HubConnection, msg *protocol.Message) {
	var req protocol.ApplyArtworkRequest
	if err := msg.ParsePayload(&req); err != nil {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeBadRequest, "invalid payload")
		return
	}

	result, err := artwork.Apply(req.UserID, req.AppID, req.Artwork)
	if err != nil {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeInternal, err.Error())
		return
	}

	log.Printf("WS: Applied artwork for AppID %d: %v", req.AppID, result.Applied)

	// Convert result to protocol type
	failed := make([]protocol.ArtworkFailed, len(result.Failed))
	for i, f := range result.Failed {
		failed[i] = protocol.ArtworkFailed{
			Type:  f.Type,
			Error: f.Error,
		}
	}

	resp, _ := msg.Reply(protocol.MsgTypeArtworkResponse, protocol.ArtworkResponse{
		Applied: result.Applied,
		Failed:  failed,
	})
	ws.send(hub, resp)
}

// handleBinaryArtwork processes a binary artwork image message.
// When appID is 0 (pre-CompleteUpload), artwork data is stored as pending
// and applied during handleCompleteUpload with the real AppID.
// When appID > 0, artwork is applied immediately.
func (ws *WSServer) handleBinaryArtwork(hub *HubConnection, msgID string, appID uint32, artworkType, contentType string, data []byte) {
	log.Printf("WS: Received artwork image: appID=%d, type=%s, contentType=%s, size=%d",
		appID, artworkType, contentType, len(data))

	if appID == 0 {
		// Store for later — applied during handleCompleteUpload with real AppID
		ws.mu.Lock()
		ws.pendingArtwork = append(ws.pendingArtwork, pendingArtwork{
			ArtworkType: artworkType,
			ContentType: contentType,
			Data:        data,
		})
		ws.mu.Unlock()
		log.Printf("WS: Stored pending artwork: type=%s (%d bytes)", artworkType, len(data))
		resp, _ := protocol.NewMessage(msgID, protocol.MsgTypeArtworkImageResponse, protocol.ArtworkImageResponse{
			Success:     true,
			ArtworkType: artworkType,
		})
		ws.send(hub, resp)
		return
	}

	if err := artwork.ApplyFromData(appID, artworkType, data, contentType); err != nil {
		log.Printf("WS: Failed to apply artwork image: %v", err)
		resp, _ := protocol.NewMessage(msgID, protocol.MsgTypeArtworkImageResponse, protocol.ArtworkImageResponse{
			Success:     false,
			ArtworkType: artworkType,
			Error:       err.Error(),
		})
		ws.send(hub, resp)
		return
	}

	log.Printf("WS: Applied artwork image: appID=%d, type=%s", appID, artworkType)
	resp, _ := protocol.NewMessage(msgID, protocol.MsgTypeArtworkImageResponse, protocol.ArtworkImageResponse{
		Success:     true,
		ArtworkType: artworkType,
	})
	ws.send(hub, resp)
}

// handleRestartSteam restarts Steam.
func (ws *WSServer) handleRestartSteam(hub *HubConnection, msg *protocol.Message) {
	controller := agentSteam.NewController()
	result := controller.Restart()

	log.Printf("WS: Steam restart: success=%v, message=%s", result.Success, result.Message)

	resp, _ := msg.Reply(protocol.MsgTypeSteamResponse, protocol.RestartSteamResponse{
		Success: result.Success,
		Message: result.Message,
	})
	ws.send(hub, resp)
}

// handleInitUpload initializes a new upload session.
func (ws *WSServer) handleInitUpload(hub *HubConnection, msg *protocol.Message) {
	var req protocol.InitUploadRequestFull
	if err := msg.ParsePayload(&req); err != nil {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeBadRequest, "invalid payload")
		return
	}

	if req.Config.GameName == "" {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeBadRequest, "gameName is required")
		return
	}

	// Convert protocol files to transfer files
	files := make([]transfer.FileEntry, len(req.Files))
	for i, f := range req.Files {
		files[i] = transfer.FileEntry{
			RelativePath: f.RelativePath,
			Size:         f.Size,
		}
	}

	session := ws.server.CreateUpload(req.Config, req.TotalSize, files)
	session.Start()

	log.Printf("WS: Upload session created: %s for game '%s' (%d bytes, %d files)",
		session.ID, req.Config.GameName, req.TotalSize, len(req.Files))

	// Notify UI about install start
	ws.server.NotifyOperation("install", "start", req.Config.GameName, 0, "Iniciando instalación...")
	ws.SendEvent(protocol.MsgTypeOperationEvent, protocol.OperationEvent{
		Type:     "install",
		Status:   "start",
		GameName: req.Config.GameName,
		Progress: 0,
		Message:  "Iniciando instalación...",
	})

	resp, _ := msg.Reply(protocol.MsgTypeUploadInitResponse, protocol.InitUploadResponseFull{
		UploadID:  session.ID,
		ChunkSize: transfer.DefaultChunkSize,
	})
	ws.send(hub, resp)
}

// handleUploadChunk processes a JSON-based chunk (metadata only, data in binary message).
func (ws *WSServer) handleUploadChunk(hub *HubConnection, msg *protocol.Message) {
	// This handler is for JSON chunk requests (small chunks embedded in JSON)
	var req protocol.UploadChunkRequest
	if err := msg.ParsePayload(&req); err != nil {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeBadRequest, "invalid payload")
		return
	}

	ws.processChunk(hub, msg.ID, req.UploadID, req.FilePath, req.Offset, "", req.Data)
}

// handleBinaryChunk processes a binary chunk message.
func (ws *WSServer) handleBinaryChunk(hub *HubConnection, msgID, uploadID, filePath string, offset int64, checksum string, data []byte) {
	ws.processChunk(hub, msgID, uploadID, filePath, offset, checksum, data)
}

// processChunk writes a chunk to disk.
func (ws *WSServer) processChunk(hub *HubConnection, msgID, uploadID, filePath string, offset int64, checksum string, data []byte) {
	session, ok := ws.server.GetUpload(uploadID)
	if !ok {
		ws.sendError(hub, msgID, protocol.WSErrCodeNotFound, "upload not found")
		return
	}

	if !session.IsActive() {
		ws.sendError(hub, msgID, protocol.WSErrCodeBadRequest, "upload is not active")
		return
	}

	// Create chunk
	chunk := &transfer.Chunk{
		Offset:   offset,
		Size:     len(data),
		Data:     data,
		FilePath: filePath,
		Checksum: checksum,
	}

	// Write chunk to disk
	gamePath := ws.server.GetUploadPath(session.Config.GameName, session.Config.InstallPath)
	writer := transfer.NewChunkWriter(gamePath, transfer.DefaultChunkSize)

	if err := writer.WriteChunk(chunk); err != nil {
		session.Fail(err.Error())
		ws.sendError(hub, msgID, protocol.WSErrCodeInternal, err.Error())
		return
	}

	// Update session progress
	session.AddProgress(int64(len(data)), filePath, offset)
	progress := session.Progress()

	// Notify UI about progress
	ws.server.NotifyOperation("install", "progress", session.Config.GameName, progress.Percentage(), "")
	ws.SendEvent(protocol.MsgTypeUploadProgress, protocol.UploadProgressEvent{
		UploadID:         uploadID,
		TransferredBytes: progress.TransferredBytes,
		TotalBytes:       progress.TotalBytes,
		CurrentFile:      progress.CurrentFile,
		Percentage:       progress.Percentage(),
	})

	if ws.server.cfg.Verbose {
		log.Printf("WS: Chunk received: %s/%s offset=%d size=%d (%.1f%%)",
			uploadID, filePath, offset, len(data), progress.Percentage())
	}

	// Send acknowledgment
	resp, _ := protocol.NewMessage(msgID, protocol.MsgTypeUploadChunkResponse, protocol.UploadChunkResponse{
		UploadID:     uploadID,
		BytesWritten: int64(len(data)),
		TotalWritten: progress.TransferredBytes,
	})
	ws.send(hub, resp)
}

// handleCompleteUpload finalizes an upload session.
func (ws *WSServer) handleCompleteUpload(hub *HubConnection, msg *protocol.Message) {
	var req protocol.CompleteUploadRequestFull
	if err := msg.ParsePayload(&req); err != nil {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeBadRequest, "invalid payload")
		return
	}

	session, ok := ws.server.GetUpload(req.UploadID)
	if !ok {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeNotFound, "upload not found")
		return
	}

	session.Complete()
	gamePath := ws.server.GetUploadPath(session.Config.GameName, session.Config.InstallPath)

	log.Printf("WS: Upload completed: %s -> %s", req.UploadID, gamePath)

	// Notify UI about completion
	ws.server.NotifyOperation("install", "complete", session.Config.GameName, 100, "Instalación completada")
	ws.SendEvent(protocol.MsgTypeOperationEvent, protocol.OperationEvent{
		Type:     "install",
		Status:   "complete",
		GameName: session.Config.GameName,
		Progress: 100,
		Message:  "Instalación completada",
	})

	// Make executable on Linux
	if runtime.GOOS == "linux" && session.Config.Executable != "" {
		exePath := filepath.Join(gamePath, session.Config.Executable)
		if err := os.Chmod(exePath, 0755); err != nil {
			log.Printf("WS: Warning: failed to make executable: %v", err)
		} else {
			log.Printf("WS: Made executable: %s", exePath)
		}
	}

	// Clean up upload session from memory
	defer ws.server.DeleteUpload(req.UploadID)

	resp := protocol.CompleteUploadResponseFull{
		Success: true,
		Path:    gamePath,
	}

	// Create shortcut if requested
	if req.CreateShortcut && req.Shortcut != nil {
		// Ensure CEF debugger is available (needed for shortcut creation via CEF API)
		if err := agentSteam.EnsureCEFReady(); err != nil {
			log.Printf("WS: Warning: CEF not available, shortcut creation may fail: %v", err)
		}

		mgr, err := shortcuts.NewManager()
		if err != nil {
			log.Printf("WS: Warning: failed to create shortcut manager: %v", err)
		} else {
			users, err := steam.GetUsers()
			if err != nil || len(users) == 0 {
				log.Printf("WS: Warning: no Steam users found for shortcut creation")
			} else {
				// Build correct paths using gamePath
				exeName := filepath.Base(req.Shortcut.Exe)
				if exeName == "" || exeName == "." {
					exeName = session.Config.Executable
				}

				shortcutCfg := *req.Shortcut
				shortcutCfg.Exe = filepath.Join(gamePath, exeName)
				shortcutCfg.StartDir = gamePath

				log.Printf("WS: Creating shortcut: Exe=%s, StartDir=%s", shortcutCfg.Exe, shortcutCfg.StartDir)

				appID, artResult, err := mgr.CreateWithArtwork(users[0].ID, shortcutCfg)
				if err != nil {
					log.Printf("WS: Warning: failed to create shortcut: %v", err)
				} else {
					resp.AppID = appID
					log.Printf("WS: Created shortcut '%s' with AppID %d", shortcutCfg.Name, appID)
					if artResult != nil && len(artResult.Applied) > 0 {
						log.Printf("WS: Applied artwork: %v", artResult.Applied)
					}
					ws.server.NotifyShortcutChange()

					// Apply pending local artwork now that we have the real AppID
					ws.mu.Lock()
					pending := ws.pendingArtwork
					ws.pendingArtwork = nil
					ws.mu.Unlock()

					for _, pa := range pending {
						if err := artwork.ApplyFromData(appID, pa.ArtworkType, pa.Data, pa.ContentType); err != nil {
							log.Printf("WS: Failed to apply pending artwork %s: %v", pa.ArtworkType, err)
						} else {
							log.Printf("WS: Applied pending artwork: appID=%d, type=%s", appID, pa.ArtworkType)
						}
					}
				}
			}
		}
	}

	respMsg, _ := msg.Reply(protocol.MsgTypeOperationResult, resp)
	ws.send(hub, respMsg)
}

// handleCancelUpload cancels an upload session.
func (ws *WSServer) handleCancelUpload(hub *HubConnection, msg *protocol.Message) {
	var req protocol.CancelUploadRequest
	if err := msg.ParsePayload(&req); err != nil {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeBadRequest, "invalid payload")
		return
	}

	session, ok := ws.server.GetUpload(req.UploadID)
	if !ok {
		ws.sendError(hub, msg.ID, protocol.WSErrCodeNotFound, "upload not found")
		return
	}

	gamePath := ws.server.GetUploadPath(session.Config.GameName, session.Config.InstallPath)
	session.Cancel()
	ws.server.DeleteUpload(req.UploadID)

	// Clean up pending artwork data to free memory
	ws.mu.Lock()
	ws.pendingArtwork = nil
	ws.mu.Unlock()

	// Clean up partial files left on disk
	if err := os.RemoveAll(gamePath); err != nil {
		log.Printf("WS: Warning: failed to clean up partial upload at %s: %v", gamePath, err)
	}

	log.Printf("WS: Upload cancelled: %s (cleaned %s)", req.UploadID, gamePath)

	resp, _ := msg.Reply(protocol.MsgTypeOperationResult, protocol.OperationResult{
		Success: true,
		Message: "cancelled",
	})
	ws.send(hub, resp)
}
