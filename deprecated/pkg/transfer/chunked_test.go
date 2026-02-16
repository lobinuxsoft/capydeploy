package transfer

import (
	"os"
	"path/filepath"
	"testing"
)

func TestNewChunkReader(t *testing.T) {
	// Create temp file
	tmpDir := t.TempDir()
	tmpFile := filepath.Join(tmpDir, "test.dat")
	data := []byte("test file content for chunk reading")
	if err := os.WriteFile(tmpFile, data, 0644); err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}

	reader, err := NewChunkReader(tmpFile, 10)
	if err != nil {
		t.Fatalf("NewChunkReader() error = %v", err)
	}
	defer reader.Close()

	if reader.FileSize() != int64(len(data)) {
		t.Errorf("FileSize() = %d, want %d", reader.FileSize(), len(data))
	}
	if reader.Offset() != 0 {
		t.Errorf("Initial Offset() = %d, want 0", reader.Offset())
	}
}

func TestNewChunkReader_DefaultChunkSize(t *testing.T) {
	tmpDir := t.TempDir()
	tmpFile := filepath.Join(tmpDir, "test.dat")
	if err := os.WriteFile(tmpFile, []byte("test"), 0644); err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}

	// Pass 0 chunk size, should use default
	reader, err := NewChunkReader(tmpFile, 0)
	if err != nil {
		t.Fatalf("NewChunkReader() error = %v", err)
	}
	defer reader.Close()

	if reader.chunkSize != DefaultChunkSize {
		t.Errorf("chunkSize = %d, want %d", reader.chunkSize, DefaultChunkSize)
	}
}

func TestNewChunkReader_NonExistentFile(t *testing.T) {
	_, err := NewChunkReader("/nonexistent/path/file.dat", 1024)
	if err == nil {
		t.Error("NewChunkReader() should error for non-existent file")
	}
}

func TestChunkReader_NextChunk(t *testing.T) {
	tmpDir := t.TempDir()
	tmpFile := filepath.Join(tmpDir, "test.dat")
	data := []byte("0123456789ABCDEFGHIJ") // 20 bytes
	if err := os.WriteFile(tmpFile, data, 0644); err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}

	reader, err := NewChunkReader(tmpFile, 5)
	if err != nil {
		t.Fatalf("NewChunkReader() error = %v", err)
	}
	defer reader.Close()

	// Read first chunk
	chunk, err := reader.NextChunk()
	if err != nil {
		t.Fatalf("NextChunk() error = %v", err)
	}
	if chunk == nil {
		t.Fatal("NextChunk() returned nil")
	}
	if string(chunk.Data) != "01234" {
		t.Errorf("Chunk 0 data = %q, want %q", chunk.Data, "01234")
	}
	if chunk.Offset != 0 {
		t.Errorf("Chunk 0 offset = %d, want 0", chunk.Offset)
	}

	// Read second chunk
	chunk, err = reader.NextChunk()
	if err != nil {
		t.Fatalf("NextChunk() error = %v", err)
	}
	if string(chunk.Data) != "56789" {
		t.Errorf("Chunk 1 data = %q, want %q", chunk.Data, "56789")
	}
	if chunk.Offset != 5 {
		t.Errorf("Chunk 1 offset = %d, want 5", chunk.Offset)
	}
}

func TestChunkReader_NextChunk_EOF(t *testing.T) {
	tmpDir := t.TempDir()
	tmpFile := filepath.Join(tmpDir, "test.dat")
	data := []byte("12345") // Exactly chunk size
	if err := os.WriteFile(tmpFile, data, 0644); err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}

	reader, err := NewChunkReader(tmpFile, 5)
	if err != nil {
		t.Fatalf("NewChunkReader() error = %v", err)
	}
	defer reader.Close()

	// Read first chunk
	chunk, err := reader.NextChunk()
	if err != nil {
		t.Fatalf("NextChunk() error = %v", err)
	}
	if chunk == nil {
		t.Fatal("NextChunk() should not be nil")
	}

	// Read past EOF
	chunk, err = reader.NextChunk()
	if err != nil {
		t.Fatalf("NextChunk() error = %v", err)
	}
	if chunk != nil {
		t.Error("NextChunk after EOF should return nil")
	}
}

func TestChunkReader_SeekTo(t *testing.T) {
	tmpDir := t.TempDir()
	tmpFile := filepath.Join(tmpDir, "test.dat")
	data := []byte("0123456789")
	if err := os.WriteFile(tmpFile, data, 0644); err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}

	reader, err := NewChunkReader(tmpFile, 3)
	if err != nil {
		t.Fatalf("NewChunkReader() error = %v", err)
	}
	defer reader.Close()

	// Seek to position 5
	if err := reader.SeekTo(5); err != nil {
		t.Fatalf("SeekTo(5) error = %v", err)
	}

	if reader.Offset() != 5 {
		t.Errorf("Offset() = %d, want 5", reader.Offset())
	}

	// Read chunk from new position
	chunk, err := reader.NextChunk()
	if err != nil {
		t.Fatalf("NextChunk() error = %v", err)
	}
	if string(chunk.Data) != "567" {
		t.Errorf("Chunk data = %q, want %q", chunk.Data, "567")
	}
}

func TestChunkReader_Remaining(t *testing.T) {
	tmpDir := t.TempDir()
	tmpFile := filepath.Join(tmpDir, "test.dat")
	data := []byte("0123456789") // 10 bytes
	if err := os.WriteFile(tmpFile, data, 0644); err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}

	reader, err := NewChunkReader(tmpFile, 3)
	if err != nil {
		t.Fatalf("NewChunkReader() error = %v", err)
	}
	defer reader.Close()

	if reader.Remaining() != 10 {
		t.Errorf("Initial Remaining() = %d, want 10", reader.Remaining())
	}

	reader.NextChunk() // Read 3 bytes

	if reader.Remaining() != 7 {
		t.Errorf("Remaining() after read = %d, want 7", reader.Remaining())
	}
}

func TestChunkReader_Checksum(t *testing.T) {
	tmpDir := t.TempDir()
	tmpFile := filepath.Join(tmpDir, "test.dat")
	data := []byte("test data")
	if err := os.WriteFile(tmpFile, data, 0644); err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}

	reader, err := NewChunkReader(tmpFile, 1024)
	if err != nil {
		t.Fatalf("NewChunkReader() error = %v", err)
	}
	defer reader.Close()

	chunk, err := reader.NextChunk()
	if err != nil {
		t.Fatalf("NextChunk() error = %v", err)
	}

	if chunk.Checksum == "" {
		t.Error("Chunk checksum should not be empty")
	}
}

func TestNewChunkWriter(t *testing.T) {
	tmpDir := t.TempDir()

	writer := NewChunkWriter(tmpDir, 1024)

	if writer.BasePath() != tmpDir {
		t.Errorf("BasePath() = %q, want %q", writer.BasePath(), tmpDir)
	}
}

func TestNewChunkWriter_DefaultChunkSize(t *testing.T) {
	writer := NewChunkWriter("/tmp", 0)

	if writer.chunkSize != DefaultChunkSize {
		t.Errorf("chunkSize = %d, want %d", writer.chunkSize, DefaultChunkSize)
	}
}

func TestChunkWriter_WriteChunk(t *testing.T) {
	tmpDir := t.TempDir()
	writer := NewChunkWriter(tmpDir, 1024)

	chunk := &Chunk{
		Offset:   0,
		Size:     9,
		Data:     []byte("test data"),
		FilePath: "output.dat",
	}

	if err := writer.WriteChunk(chunk); err != nil {
		t.Fatalf("WriteChunk() error = %v", err)
	}

	// Verify file was written
	outPath := filepath.Join(tmpDir, "output.dat")
	data, err := os.ReadFile(outPath)
	if err != nil {
		t.Fatalf("Failed to read output file: %v", err)
	}

	if string(data) != "test data" {
		t.Errorf("File content = %q, want %q", data, "test data")
	}
}

func TestChunkWriter_WriteChunk_WithSubdirectory(t *testing.T) {
	tmpDir := t.TempDir()
	writer := NewChunkWriter(tmpDir, 1024)

	chunk := &Chunk{
		Offset:   0,
		Size:     4,
		Data:     []byte("data"),
		FilePath: "subdir/nested/file.dat",
	}

	if err := writer.WriteChunk(chunk); err != nil {
		t.Fatalf("WriteChunk() error = %v", err)
	}

	// Verify file was written in subdirectory
	outPath := filepath.Join(tmpDir, "subdir/nested/file.dat")
	if _, err := os.Stat(outPath); err != nil {
		t.Errorf("File not created at expected path: %v", err)
	}
}

func TestChunkWriter_WriteChunk_AtOffset(t *testing.T) {
	tmpDir := t.TempDir()
	writer := NewChunkWriter(tmpDir, 1024)

	// Write first chunk
	chunk1 := &Chunk{
		Offset:   0,
		Size:     5,
		Data:     []byte("AAAAA"),
		FilePath: "file.dat",
	}
	if err := writer.WriteChunk(chunk1); err != nil {
		t.Fatalf("WriteChunk(0) error = %v", err)
	}

	// Write second chunk at offset
	chunk2 := &Chunk{
		Offset:   5,
		Size:     5,
		Data:     []byte("BBBBB"),
		FilePath: "file.dat",
	}
	if err := writer.WriteChunk(chunk2); err != nil {
		t.Fatalf("WriteChunk(1) error = %v", err)
	}

	// Verify content
	outPath := filepath.Join(tmpDir, "file.dat")
	data, err := os.ReadFile(outPath)
	if err != nil {
		t.Fatalf("Failed to read file: %v", err)
	}

	if string(data) != "AAAAABBBBB" {
		t.Errorf("File content = %q, want %q", data, "AAAAABBBBB")
	}
}

func TestChunkWriter_WriteChunk_ChecksumMismatch(t *testing.T) {
	tmpDir := t.TempDir()
	writer := NewChunkWriter(tmpDir, 1024)

	chunk := &Chunk{
		Offset:   0,
		Size:     4,
		Data:     []byte("test"),
		FilePath: "file.dat",
		Checksum: "invalid_checksum", // Wrong checksum
	}

	err := writer.WriteChunk(chunk)
	if err == nil {
		t.Error("WriteChunk() should error on checksum mismatch")
	}
	if err != ErrChecksumMismatch {
		t.Errorf("Error = %v, want ErrChecksumMismatch", err)
	}
}

func TestChunkWriter_GetWrittenOffset(t *testing.T) {
	tmpDir := t.TempDir()
	writer := NewChunkWriter(tmpDir, 1024)

	// Initial offset should be 0
	if offset := writer.GetWrittenOffset("file.dat"); offset != 0 {
		t.Errorf("Initial offset = %d, want 0", offset)
	}

	// Write a chunk
	chunk := &Chunk{
		Offset:   0,
		Size:     100,
		Data:     make([]byte, 100),
		FilePath: "file.dat",
	}
	writer.WriteChunk(chunk)

	if offset := writer.GetWrittenOffset("file.dat"); offset != 100 {
		t.Errorf("Offset after write = %d, want 100", offset)
	}
}

func TestCalculateFileChecksum(t *testing.T) {
	tmpDir := t.TempDir()
	tmpFile := filepath.Join(tmpDir, "test.dat")
	data := []byte("test data for checksum")
	if err := os.WriteFile(tmpFile, data, 0644); err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}

	checksum, err := CalculateFileChecksum(tmpFile)
	if err != nil {
		t.Fatalf("CalculateFileChecksum() error = %v", err)
	}

	if checksum == "" {
		t.Error("Checksum should not be empty")
	}

	// Calculate again to verify consistency
	checksum2, _ := CalculateFileChecksum(tmpFile)
	if checksum != checksum2 {
		t.Error("Checksum should be consistent")
	}
}

func TestCalculateFileChecksum_NonExistent(t *testing.T) {
	_, err := CalculateFileChecksum("/nonexistent/file.dat")
	if err == nil {
		t.Error("CalculateFileChecksum() should error for non-existent file")
	}
}

func TestChecksumError(t *testing.T) {
	err := &ChecksumError{Message: "test error"}

	if err.Error() != "test error" {
		t.Errorf("Error() = %q, want %q", err.Error(), "test error")
	}
}

func TestErrChecksumMismatch(t *testing.T) {
	if ErrChecksumMismatch == nil {
		t.Error("ErrChecksumMismatch should not be nil")
	}
	if ErrChecksumMismatch.Error() == "" {
		t.Error("ErrChecksumMismatch message should not be empty")
	}
}

func TestChecksumBytes(t *testing.T) {
	data1 := []byte("test data")
	data2 := []byte("test data")
	data3 := []byte("different data")

	checksum1 := checksumBytes(data1)
	checksum2 := checksumBytes(data2)
	checksum3 := checksumBytes(data3)

	if checksum1 != checksum2 {
		t.Error("Same data should produce same checksum")
	}
	if checksum1 == checksum3 {
		t.Error("Different data should produce different checksum")
	}
	if checksum1 == "" {
		t.Error("Checksum should not be empty")
	}
}
