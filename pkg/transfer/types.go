// Package transfer provides chunked file transfer with resume support.
package transfer

import (
	"sync"
	"time"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

// DefaultChunkSize is the default size for file chunks (1MB).
const DefaultChunkSize = 1024 * 1024

// Chunk represents a single chunk of data in a transfer.
type Chunk struct {
	Offset   int64  `json:"offset"`
	Size     int    `json:"size"`
	Data     []byte `json:"data,omitempty"`
	FilePath string `json:"filePath"`
	Checksum string `json:"checksum,omitempty"`
}

// FileEntry represents a file in the upload.
type FileEntry struct {
	RelativePath string `json:"relativePath"`
	Size         int64  `json:"size"`
}

// UploadSession tracks an active upload operation.
type UploadSession struct {
	mu sync.RWMutex

	ID               string                  `json:"id"`
	Config           protocol.UploadConfig   `json:"config"`
	Status           protocol.UploadStatus   `json:"status"`
	TotalBytes       int64                   `json:"totalBytes"`
	TransferredBytes int64                   `json:"transferredBytes"`
	Files            []FileEntry             `json:"files"`
	CurrentFileIndex int                     `json:"currentFileIndex"`
	StartedAt        time.Time               `json:"startedAt"`
	UpdatedAt        time.Time               `json:"updatedAt"`
	CompletedAt      *time.Time              `json:"completedAt,omitempty"`
	Error            string                  `json:"error,omitempty"`
	ChunkOffsets     map[string]int64        `json:"chunkOffsets"` // file -> last confirmed offset
}

// NewUploadSession creates a new upload session.
func NewUploadSession(id string, config protocol.UploadConfig, totalBytes int64, files []FileEntry) *UploadSession {
	now := time.Now()
	return &UploadSession{
		ID:           id,
		Config:       config,
		Status:       protocol.UploadStatusPending,
		TotalBytes:   totalBytes,
		Files:        files,
		StartedAt:    now,
		UpdatedAt:    now,
		ChunkOffsets: make(map[string]int64),
	}
}

// Start marks the session as in progress.
func (s *UploadSession) Start() {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.Status = protocol.UploadStatusInProgress
	s.UpdatedAt = time.Now()
}

// AddProgress adds bytes to the transferred count.
func (s *UploadSession) AddProgress(bytes int64, filePath string, offset int64) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.TransferredBytes += bytes
	s.ChunkOffsets[filePath] = offset + bytes
	s.UpdatedAt = time.Now()
}

// Complete marks the session as completed.
func (s *UploadSession) Complete() {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.Status = protocol.UploadStatusCompleted
	now := time.Now()
	s.CompletedAt = &now
	s.UpdatedAt = now
}

// Fail marks the session as failed with an error.
func (s *UploadSession) Fail(err string) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.Status = protocol.UploadStatusFailed
	s.Error = err
	s.UpdatedAt = time.Now()
}

// Cancel marks the session as cancelled.
func (s *UploadSession) Cancel() {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.Status = protocol.UploadStatusCancelled
	s.UpdatedAt = time.Now()
}

// Progress returns the current progress.
func (s *UploadSession) Progress() protocol.UploadProgress {
	s.mu.RLock()
	defer s.mu.RUnlock()
	currentFile := ""
	if s.CurrentFileIndex < len(s.Files) {
		currentFile = s.Files[s.CurrentFileIndex].RelativePath
	}
	return protocol.UploadProgress{
		UploadID:         s.ID,
		Status:           s.Status,
		TotalBytes:       s.TotalBytes,
		TransferredBytes: s.TransferredBytes,
		CurrentFile:      currentFile,
		StartedAt:        s.StartedAt,
		UpdatedAt:        s.UpdatedAt,
		Error:            s.Error,
	}
}

// GetResumeOffset returns the offset to resume from for a file.
func (s *UploadSession) GetResumeOffset(filePath string) int64 {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.ChunkOffsets[filePath]
}

// IsActive returns true if the session is still active.
func (s *UploadSession) IsActive() bool {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.Status == protocol.UploadStatusPending || s.Status == protocol.UploadStatusInProgress
}
