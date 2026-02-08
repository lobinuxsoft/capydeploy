package server

import (
	"encoding/json"
	"io"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"runtime"

	"github.com/lobinuxsoft/capydeploy/apps/agents/desktop/shortcuts"
	agentSteam "github.com/lobinuxsoft/capydeploy/apps/agents/desktop/steam"
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/steam"
	"github.com/lobinuxsoft/capydeploy/pkg/transfer"
)

// Upload request/response types

// InitUploadRequest is the request body for POST /uploads.
type InitUploadRequest struct {
	Config     protocol.UploadConfig `json:"config"`
	TotalSize  int64                 `json:"totalSize"`
	Files      []transfer.FileEntry  `json:"files"`
}

// InitUploadResponse is the response for POST /uploads.
type InitUploadResponse struct {
	UploadID   string           `json:"uploadId"`
	ChunkSize  int              `json:"chunkSize"`
	ResumeFrom map[string]int64 `json:"resumeFrom,omitempty"`
	Error      string           `json:"error,omitempty"`
}

// ChunkUploadResponse is the response for POST /uploads/{id}/chunks.
type ChunkUploadResponse struct {
	BytesWritten  int   `json:"bytesWritten"`
	TotalReceived int64 `json:"totalReceived"`
	Error         string `json:"error,omitempty"`
}

// CompleteUploadRequest is the request body for POST /uploads/{id}/complete.
type CompleteUploadRequest struct {
	CreateShortcut bool                    `json:"createShortcut"`
	Shortcut       *protocol.ShortcutConfig `json:"shortcut,omitempty"`
}

// CompleteUploadResponse is the response for POST /uploads/{id}/complete.
type CompleteUploadResponse struct {
	Success bool   `json:"success"`
	Path    string `json:"path,omitempty"`
	AppID   uint32 `json:"appId,omitempty"`
	Error   string `json:"error,omitempty"`
}

// UploadStatusResponse is the response for GET /uploads/{id}.
type UploadStatusResponse struct {
	Progress *protocol.UploadProgress `json:"progress,omitempty"`
	Error    string                   `json:"error,omitempty"`
}

// handleInitUpload handles POST /uploads - Initialize a new upload session.
func (s *Server) handleInitUpload(w http.ResponseWriter, r *http.Request) {
	if s.cfg.Verbose {
		log.Printf("Init upload request from %s", r.RemoteAddr)
	}

	w.Header().Set("Content-Type", "application/json")

	var req InitUploadRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(InitUploadResponse{Error: "invalid request body"})
		return
	}

	if req.Config.GameName == "" {
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(InitUploadResponse{Error: "gameName is required"})
		return
	}

	session := s.CreateUpload(req.Config, req.TotalSize, req.Files)
	session.Start()

	log.Printf("Upload session created: %s for game '%s' (%d bytes, %d files)",
		session.ID, req.Config.GameName, req.TotalSize, len(req.Files))

	// Notify UI about install start
	s.NotifyOperation("install", "start", req.Config.GameName, 0, "Iniciando instalación...")

	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(InitUploadResponse{
		UploadID:  session.ID,
		ChunkSize: transfer.DefaultChunkSize,
	})
}

// handleUploadChunk handles POST /uploads/{id}/chunks - Receive a chunk.
func (s *Server) handleUploadChunk(w http.ResponseWriter, r *http.Request) {
	uploadID := r.PathValue("id")

	session, ok := s.GetUpload(uploadID)
	if !ok {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusNotFound)
		json.NewEncoder(w).Encode(ChunkUploadResponse{Error: "upload not found"})
		return
	}

	if !session.IsActive() {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(ChunkUploadResponse{Error: "upload is not active"})
		return
	}

	// Get chunk metadata from headers
	filePath := r.Header.Get("X-File-Path")
	checksum := r.Header.Get("X-Chunk-Checksum")

	var offset int64
	if offsetStr := r.Header.Get("X-Chunk-Offset"); offsetStr != "" {
		json.Unmarshal([]byte(offsetStr), &offset)
	}

	if filePath == "" {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(ChunkUploadResponse{Error: "X-File-Path header is required"})
		return
	}

	// Read chunk data from body
	data, err := io.ReadAll(r.Body)
	if err != nil {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(ChunkUploadResponse{Error: "failed to read chunk data"})
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
	gamePath := s.GetUploadPath(session.Config.GameName, session.Config.InstallPath)
	writer := transfer.NewChunkWriter(gamePath, transfer.DefaultChunkSize)

	if err := writer.WriteChunk(chunk); err != nil {
		session.Fail(err.Error())
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusInternalServerError)
		json.NewEncoder(w).Encode(ChunkUploadResponse{Error: err.Error()})
		return
	}

	// Update session progress
	session.AddProgress(int64(len(data)), filePath, offset)
	progress := session.Progress()

	// Notify UI about progress
	s.NotifyOperation("install", "progress", session.Config.GameName, progress.Percentage(), "")

	if s.cfg.Verbose {
		log.Printf("Chunk received: %s/%s offset=%d size=%d (%.1f%%)",
			uploadID, filePath, offset, len(data), progress.Percentage())
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(ChunkUploadResponse{
		BytesWritten:  len(data),
		TotalReceived: session.Progress().TransferredBytes,
	})
}

// handleCompleteUpload handles POST /uploads/{id}/complete - Finalize upload.
func (s *Server) handleCompleteUpload(w http.ResponseWriter, r *http.Request) {
	uploadID := r.PathValue("id")

	if s.cfg.Verbose {
		log.Printf("Complete upload request: %s from %s", uploadID, r.RemoteAddr)
	}

	w.Header().Set("Content-Type", "application/json")

	session, ok := s.GetUpload(uploadID)
	if !ok {
		w.WriteHeader(http.StatusNotFound)
		json.NewEncoder(w).Encode(CompleteUploadResponse{Error: "upload not found"})
		return
	}

	var req CompleteUploadRequest
	json.NewDecoder(r.Body).Decode(&req) // Ignore error, fields are optional

	session.Complete()
	gamePath := s.GetUploadPath(session.Config.GameName, session.Config.InstallPath)

	log.Printf("Upload completed: %s -> %s", uploadID, gamePath)

	// Notify UI about completion
	s.NotifyOperation("install", "complete", session.Config.GameName, 100, "Instalación completada")

	// Make executable on Linux
	if runtime.GOOS == "linux" && session.Config.Executable != "" {
		exePath := filepath.Join(gamePath, session.Config.Executable)
		if err := os.Chmod(exePath, 0755); err != nil {
			log.Printf("Warning: failed to make executable: %v", err)
		} else {
			log.Printf("Made executable: %s", exePath)
		}
	}

	resp := CompleteUploadResponse{
		Success: true,
		Path:    gamePath,
	}

	// Create shortcut if requested
	if req.CreateShortcut && req.Shortcut != nil {
		// Ensure CEF debugger is available (needed for shortcut creation via CEF API)
		if err := agentSteam.EnsureCEFReady(); err != nil {
			log.Printf("Warning: CEF not available, shortcut creation may fail: %v", err)
		}

		mgr, err := shortcuts.NewManager()
		if err != nil {
			log.Printf("Warning: failed to create shortcut manager: %v", err)
		} else {
			users, err := steam.GetUsers()
			if err != nil || len(users) == 0 {
				log.Printf("Warning: no Steam users found for shortcut creation")
			} else {
				// Build correct paths using gamePath (where files were actually installed)
				// Extract just the executable name from whatever path was sent
				exeName := filepath.Base(req.Shortcut.Exe)
				if exeName == "" || exeName == "." {
					exeName = session.Config.Executable
				}

				// Create shortcut config with correct absolute paths
				shortcutCfg := *req.Shortcut
				shortcutCfg.Exe = filepath.Join(gamePath, exeName)
				shortcutCfg.StartDir = gamePath

				log.Printf("Creating shortcut: Exe=%s, StartDir=%s", shortcutCfg.Exe, shortcutCfg.StartDir)

				appID, artResult, err := mgr.CreateWithArtwork(users[0].ID, shortcutCfg)
				if err != nil {
					log.Printf("Warning: failed to create shortcut: %v", err)
				} else {
					resp.AppID = appID
					log.Printf("Created shortcut '%s' with AppID %d", shortcutCfg.Name, appID)
					if artResult != nil && len(artResult.Applied) > 0 {
						log.Printf("Applied artwork: %v", artResult.Applied)
					}
					s.NotifyShortcutChange()
				}
			}
		}
	}

	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(resp)
}

// handleCancelUpload handles DELETE /uploads/{id} - Cancel upload.
func (s *Server) handleCancelUpload(w http.ResponseWriter, r *http.Request) {
	uploadID := r.PathValue("id")

	if s.cfg.Verbose {
		log.Printf("Cancel upload request: %s from %s", uploadID, r.RemoteAddr)
	}

	w.Header().Set("Content-Type", "application/json")

	session, ok := s.GetUpload(uploadID)
	if !ok {
		w.WriteHeader(http.StatusNotFound)
		json.NewEncoder(w).Encode(map[string]string{"error": "upload not found"})
		return
	}

	session.Cancel()

	// TODO: Clean up partial files
	log.Printf("Upload cancelled: %s", uploadID)

	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]string{"status": "cancelled"})
}

// handleGetUploadStatus handles GET /uploads/{id} - Get upload status.
func (s *Server) handleGetUploadStatus(w http.ResponseWriter, r *http.Request) {
	uploadID := r.PathValue("id")

	w.Header().Set("Content-Type", "application/json")

	session, ok := s.GetUpload(uploadID)
	if !ok {
		w.WriteHeader(http.StatusNotFound)
		json.NewEncoder(w).Encode(UploadStatusResponse{Error: "upload not found"})
		return
	}

	progress := session.Progress()
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(UploadStatusResponse{Progress: &progress})
}
