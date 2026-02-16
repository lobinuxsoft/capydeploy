package transfer

import (
	"testing"
	"time"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

func TestNewUploadSession(t *testing.T) {
	config := protocol.UploadConfig{
		GameName:   "Test Game",
		InstallPath: "/games/test",
	}
	files := []FileEntry{
		{RelativePath: "game.exe", Size: 1024},
		{RelativePath: "data/file.dat", Size: 2048},
	}

	session := NewUploadSession("upload-123", config, 3072, files)

	if session.ID != "upload-123" {
		t.Errorf("ID = %q, want %q", session.ID, "upload-123")
	}
	if session.Status != protocol.UploadStatusPending {
		t.Errorf("Status = %q, want %q", session.Status, protocol.UploadStatusPending)
	}
	if session.TotalBytes != 3072 {
		t.Errorf("TotalBytes = %d, want %d", session.TotalBytes, 3072)
	}
	if len(session.Files) != 2 {
		t.Errorf("Files length = %d, want %d", len(session.Files), 2)
	}
	if session.ChunkOffsets == nil {
		t.Error("ChunkOffsets should not be nil")
	}
}

func TestUploadSession_Start(t *testing.T) {
	session := NewUploadSession("test", protocol.UploadConfig{}, 1000, nil)

	if session.Status != protocol.UploadStatusPending {
		t.Errorf("Initial status = %q, want %q", session.Status, protocol.UploadStatusPending)
	}

	session.Start()

	if session.Status != protocol.UploadStatusInProgress {
		t.Errorf("After Start() status = %q, want %q", session.Status, protocol.UploadStatusInProgress)
	}
}

func TestUploadSession_AddProgress(t *testing.T) {
	session := NewUploadSession("test", protocol.UploadConfig{}, 1000, nil)
	session.Start()

	session.AddProgress(100, "file1.dat", 0)
	if session.TransferredBytes != 100 {
		t.Errorf("TransferredBytes = %d, want %d", session.TransferredBytes, 100)
	}
	if session.ChunkOffsets["file1.dat"] != 100 {
		t.Errorf("ChunkOffsets[file1.dat] = %d, want %d", session.ChunkOffsets["file1.dat"], 100)
	}

	session.AddProgress(200, "file1.dat", 100)
	if session.TransferredBytes != 300 {
		t.Errorf("TransferredBytes = %d, want %d", session.TransferredBytes, 300)
	}
	if session.ChunkOffsets["file1.dat"] != 300 {
		t.Errorf("ChunkOffsets[file1.dat] = %d, want %d", session.ChunkOffsets["file1.dat"], 300)
	}
}

func TestUploadSession_Complete(t *testing.T) {
	session := NewUploadSession("test", protocol.UploadConfig{}, 1000, nil)
	session.Start()
	session.AddProgress(1000, "file.dat", 0)

	session.Complete()

	if session.Status != protocol.UploadStatusCompleted {
		t.Errorf("Status = %q, want %q", session.Status, protocol.UploadStatusCompleted)
	}
	if session.CompletedAt == nil {
		t.Error("CompletedAt should not be nil")
	}
}

func TestUploadSession_Fail(t *testing.T) {
	session := NewUploadSession("test", protocol.UploadConfig{}, 1000, nil)
	session.Start()

	session.Fail("disk full")

	if session.Status != protocol.UploadStatusFailed {
		t.Errorf("Status = %q, want %q", session.Status, protocol.UploadStatusFailed)
	}
	if session.Error != "disk full" {
		t.Errorf("Error = %q, want %q", session.Error, "disk full")
	}
}

func TestUploadSession_Cancel(t *testing.T) {
	session := NewUploadSession("test", protocol.UploadConfig{}, 1000, nil)
	session.Start()

	session.Cancel()

	if session.Status != protocol.UploadStatusCancelled {
		t.Errorf("Status = %q, want %q", session.Status, protocol.UploadStatusCancelled)
	}
}

func TestUploadSession_Progress(t *testing.T) {
	files := []FileEntry{
		{RelativePath: "file1.dat", Size: 500},
		{RelativePath: "file2.dat", Size: 500},
	}
	session := NewUploadSession("test", protocol.UploadConfig{}, 1000, files)
	session.Start()
	session.AddProgress(250, "file1.dat", 0)

	progress := session.Progress()

	if progress.UploadID != "test" {
		t.Errorf("UploadID = %q, want %q", progress.UploadID, "test")
	}
	if progress.Status != protocol.UploadStatusInProgress {
		t.Errorf("Status = %q, want %q", progress.Status, protocol.UploadStatusInProgress)
	}
	if progress.TotalBytes != 1000 {
		t.Errorf("TotalBytes = %d, want %d", progress.TotalBytes, 1000)
	}
	if progress.TransferredBytes != 250 {
		t.Errorf("TransferredBytes = %d, want %d", progress.TransferredBytes, 250)
	}
	if progress.CurrentFile != "file1.dat" {
		t.Errorf("CurrentFile = %q, want %q", progress.CurrentFile, "file1.dat")
	}
}

func TestUploadSession_GetResumeOffset(t *testing.T) {
	session := NewUploadSession("test", protocol.UploadConfig{}, 1000, nil)
	session.AddProgress(500, "file.dat", 0)

	offset := session.GetResumeOffset("file.dat")
	if offset != 500 {
		t.Errorf("GetResumeOffset() = %d, want %d", offset, 500)
	}

	// Unknown file should return 0
	offset = session.GetResumeOffset("unknown.dat")
	if offset != 0 {
		t.Errorf("GetResumeOffset(unknown) = %d, want %d", offset, 0)
	}
}

func TestUploadSession_IsActive(t *testing.T) {
	tests := []struct {
		name   string
		status protocol.UploadStatus
		want   bool
	}{
		{"pending", protocol.UploadStatusPending, true},
		{"in_progress", protocol.UploadStatusInProgress, true},
		{"completed", protocol.UploadStatusCompleted, false},
		{"failed", protocol.UploadStatusFailed, false},
		{"cancelled", protocol.UploadStatusCancelled, false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			session := NewUploadSession("test", protocol.UploadConfig{}, 1000, nil)
			session.Status = tt.status

			if got := session.IsActive(); got != tt.want {
				t.Errorf("IsActive() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestChunk_Fields(t *testing.T) {
	chunk := Chunk{
		Offset:   1024,
		Size:     512,
		Data:     []byte("test data"),
		FilePath: "test.dat",
		Checksum: "abc123",
	}

	if chunk.Offset != 1024 {
		t.Errorf("Offset = %d, want %d", chunk.Offset, 1024)
	}
	if chunk.Size != 512 {
		t.Errorf("Size = %d, want %d", chunk.Size, 512)
	}
}

func TestFileEntry_Fields(t *testing.T) {
	entry := FileEntry{
		RelativePath: "data/game.dat",
		Size:         1024 * 1024,
	}

	if entry.RelativePath != "data/game.dat" {
		t.Errorf("RelativePath = %q, want %q", entry.RelativePath, "data/game.dat")
	}
	if entry.Size != 1024*1024 {
		t.Errorf("Size = %d, want %d", entry.Size, 1024*1024)
	}
}

func TestDefaultChunkSize(t *testing.T) {
	if DefaultChunkSize != 1024*1024 {
		t.Errorf("DefaultChunkSize = %d, want %d (1MB)", DefaultChunkSize, 1024*1024)
	}
}

func TestUploadSession_ConcurrentAccess(t *testing.T) {
	session := NewUploadSession("test", protocol.UploadConfig{}, 10000, nil)
	session.Start()

	done := make(chan bool)

	// Concurrent writers
	for i := 0; i < 10; i++ {
		go func(idx int) {
			for j := 0; j < 100; j++ {
				session.AddProgress(1, "file.dat", int64(idx*100+j))
			}
			done <- true
		}(i)
	}

	// Concurrent readers
	for i := 0; i < 10; i++ {
		go func() {
			for j := 0; j < 100; j++ {
				_ = session.Progress()
				_ = session.IsActive()
				_ = session.GetResumeOffset("file.dat")
			}
			done <- true
		}()
	}

	// Wait for all goroutines
	for i := 0; i < 20; i++ {
		select {
		case <-done:
		case <-time.After(5 * time.Second):
			t.Fatal("Timeout waiting for concurrent operations")
		}
	}
}
