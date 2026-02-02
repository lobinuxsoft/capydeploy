package transfer

import (
	"crypto/sha256"
	"encoding/hex"
	"io"
	"os"
	"path/filepath"
)

// ChunkReader reads a file in chunks.
type ChunkReader struct {
	file      *os.File
	chunkSize int
	offset    int64
	filePath  string
	fileSize  int64
}

// NewChunkReader creates a new chunk reader for the given file.
func NewChunkReader(path string, chunkSize int) (*ChunkReader, error) {
	if chunkSize <= 0 {
		chunkSize = DefaultChunkSize
	}

	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}

	info, err := file.Stat()
	if err != nil {
		file.Close()
		return nil, err
	}

	return &ChunkReader{
		file:      file,
		chunkSize: chunkSize,
		filePath:  path,
		fileSize:  info.Size(),
	}, nil
}

// SeekTo moves the reader to the given offset for resume support.
func (r *ChunkReader) SeekTo(offset int64) error {
	_, err := r.file.Seek(offset, io.SeekStart)
	if err != nil {
		return err
	}
	r.offset = offset
	return nil
}

// NextChunk reads and returns the next chunk, or nil if EOF.
func (r *ChunkReader) NextChunk() (*Chunk, error) {
	buf := make([]byte, r.chunkSize)
	n, err := r.file.Read(buf)
	if err == io.EOF {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}

	data := buf[:n]
	chunk := &Chunk{
		Offset:   r.offset,
		Size:     n,
		Data:     data,
		FilePath: r.filePath,
		Checksum: checksumBytes(data),
	}

	r.offset += int64(n)
	return chunk, nil
}

// Close closes the underlying file.
func (r *ChunkReader) Close() error {
	return r.file.Close()
}

// Offset returns the current read offset.
func (r *ChunkReader) Offset() int64 {
	return r.offset
}

// FileSize returns the total file size.
func (r *ChunkReader) FileSize() int64 {
	return r.fileSize
}

// Remaining returns the number of bytes remaining.
func (r *ChunkReader) Remaining() int64 {
	return r.fileSize - r.offset
}

// ChunkWriter writes chunks to a file with resume support.
type ChunkWriter struct {
	basePath  string
	chunkSize int
	written   map[string]int64 // file -> bytes written
}

// NewChunkWriter creates a new chunk writer.
func NewChunkWriter(basePath string, chunkSize int) *ChunkWriter {
	if chunkSize <= 0 {
		chunkSize = DefaultChunkSize
	}
	return &ChunkWriter{
		basePath:  basePath,
		chunkSize: chunkSize,
		written:   make(map[string]int64),
	}
}

// WriteChunk writes a chunk to disk at the correct offset.
func (w *ChunkWriter) WriteChunk(chunk *Chunk) error {
	fullPath := filepath.Join(w.basePath, chunk.FilePath)

	// Ensure directory exists
	dir := filepath.Dir(fullPath)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return err
	}

	// Verify checksum if provided
	if chunk.Checksum != "" {
		if checksumBytes(chunk.Data) != chunk.Checksum {
			return ErrChecksumMismatch
		}
	}

	// Open file for writing (create if not exists)
	file, err := os.OpenFile(fullPath, os.O_CREATE|os.O_WRONLY, 0644)
	if err != nil {
		return err
	}
	defer file.Close()

	// Seek to offset
	if _, err := file.Seek(chunk.Offset, io.SeekStart); err != nil {
		return err
	}

	// Write data
	n, err := file.Write(chunk.Data)
	if err != nil {
		return err
	}

	w.written[chunk.FilePath] = chunk.Offset + int64(n)
	return nil
}

// GetWrittenOffset returns the last written offset for a file.
func (w *ChunkWriter) GetWrittenOffset(filePath string) int64 {
	return w.written[filePath]
}

// BasePath returns the base path for writes.
func (w *ChunkWriter) BasePath() string {
	return w.basePath
}

// checksumBytes calculates SHA256 checksum of data.
func checksumBytes(data []byte) string {
	hash := sha256.Sum256(data)
	return hex.EncodeToString(hash[:])
}

// CalculateFileChecksum calculates the SHA256 checksum of an entire file.
func CalculateFileChecksum(path string) (string, error) {
	file, err := os.Open(path)
	if err != nil {
		return "", err
	}
	defer file.Close()

	hash := sha256.New()
	if _, err := io.Copy(hash, file); err != nil {
		return "", err
	}

	return hex.EncodeToString(hash.Sum(nil)), nil
}

// ErrChecksumMismatch is returned when chunk checksum verification fails.
var ErrChecksumMismatch = &ChecksumError{Message: "checksum mismatch"}

// ChecksumError represents a checksum verification failure.
type ChecksumError struct {
	Message string
}

func (e *ChecksumError) Error() string {
	return e.Message
}
