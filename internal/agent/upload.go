package agent

import (
	"context"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
	"github.com/lobinuxsoft/capydeploy/pkg/transfer"
)

// UploadProgress is a callback for upload progress updates.
type UploadProgress func(transferred, total int64, currentFile string)

// UploadOptions contains options for uploading a game.
type UploadOptions struct {
	// LocalPath is the local directory to upload.
	LocalPath string

	// Config is the upload configuration.
	Config protocol.UploadConfig

	// ChunkSize is the size of each chunk (default: 1MB).
	ChunkSize int

	// OnProgress is called with progress updates.
	OnProgress UploadProgress

	// CreateShortcut indicates whether to create a Steam shortcut.
	CreateShortcut bool

	// Shortcut is the shortcut configuration (required if CreateShortcut is true).
	Shortcut *protocol.ShortcutConfig
}

// UploadResult contains the result of an upload.
type UploadResult struct {
	UploadID string
	Path     string
	AppID    uint32
}

// UploadGame uploads a game directory to the agent.
func (c *Client) UploadGame(ctx context.Context, opts UploadOptions) (*UploadResult, error) {
	if opts.ChunkSize <= 0 {
		opts.ChunkSize = transfer.DefaultChunkSize
	}

	// Collect file information
	files, totalSize, err := collectFiles(opts.LocalPath)
	if err != nil {
		return nil, fmt.Errorf("failed to collect files: %w", err)
	}

	if len(files) == 0 {
		return nil, fmt.Errorf("no files found in %s", opts.LocalPath)
	}

	// Initialize upload session
	initResp, err := c.InitUpload(ctx, opts.Config, totalSize, files)
	if err != nil {
		return nil, fmt.Errorf("failed to init upload: %w", err)
	}

	uploadID := initResp.UploadID
	var transferred int64

	// Upload each file
	for _, file := range files {
		localFilePath := filepath.Join(opts.LocalPath, file.RelativePath)

		reader, err := transfer.NewChunkReader(localFilePath, opts.ChunkSize)
		if err != nil {
			c.CancelUpload(ctx, uploadID)
			return nil, fmt.Errorf("failed to open file %s: %w", file.RelativePath, err)
		}

		// Check for resume offset
		if resumeOffset, ok := initResp.ResumeFrom[file.RelativePath]; ok && resumeOffset > 0 {
			if err := reader.SeekTo(resumeOffset); err != nil {
				reader.Close()
				c.CancelUpload(ctx, uploadID)
				return nil, fmt.Errorf("failed to seek in file %s: %w", file.RelativePath, err)
			}
			transferred += resumeOffset
		}

		// Upload chunks
		chunkIndex := 0
		for {
			select {
			case <-ctx.Done():
				reader.Close()
				c.CancelUpload(ctx, uploadID)
				return nil, ctx.Err()
			default:
			}

			chunk, err := reader.NextChunk(chunkIndex)
			if err != nil {
				reader.Close()
				c.CancelUpload(ctx, uploadID)
				return nil, fmt.Errorf("failed to read chunk from %s: %w", file.RelativePath, err)
			}

			if chunk == nil {
				// EOF
				break
			}

			// Set the relative path for the chunk
			chunk.FilePath = file.RelativePath

			if err := c.UploadChunk(ctx, uploadID, chunk); err != nil {
				reader.Close()
				c.CancelUpload(ctx, uploadID)
				return nil, fmt.Errorf("failed to upload chunk: %w", err)
			}

			transferred += int64(chunk.Size)
			chunkIndex++

			if opts.OnProgress != nil {
				opts.OnProgress(transferred, totalSize, file.RelativePath)
			}
		}

		reader.Close()
	}

	// Complete upload
	completeResp, err := c.CompleteUpload(ctx, uploadID, opts.CreateShortcut, opts.Shortcut)
	if err != nil {
		return nil, fmt.Errorf("failed to complete upload: %w", err)
	}

	return &UploadResult{
		UploadID: uploadID,
		Path:     completeResp.Path,
		AppID:    completeResp.AppID,
	}, nil
}

// collectFiles walks the directory and collects file information.
func collectFiles(basePath string) ([]transfer.FileEntry, int64, error) {
	var files []transfer.FileEntry
	var totalSize int64

	err := filepath.WalkDir(basePath, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}

		if d.IsDir() {
			return nil
		}

		info, err := d.Info()
		if err != nil {
			return err
		}

		relPath, err := filepath.Rel(basePath, path)
		if err != nil {
			return err
		}

		// Convert to forward slashes for cross-platform compatibility
		relPath = filepath.ToSlash(relPath)

		files = append(files, transfer.FileEntry{
			RelativePath: relPath,
			Size:         info.Size(),
			Mode:         uint32(info.Mode()),
		})

		totalSize += info.Size()
		return nil
	})

	if err != nil {
		return nil, 0, err
	}

	return files, totalSize, nil
}

// UploadSingleFile uploads a single file to the agent.
func (c *Client) UploadSingleFile(ctx context.Context, localPath string, config protocol.UploadConfig, onProgress UploadProgress) (*UploadResult, error) {
	info, err := os.Stat(localPath)
	if err != nil {
		return nil, fmt.Errorf("failed to stat file: %w", err)
	}

	if info.IsDir() {
		return nil, fmt.Errorf("expected file, got directory")
	}

	files := []transfer.FileEntry{
		{
			RelativePath: filepath.Base(localPath),
			Size:         info.Size(),
			Mode:         uint32(info.Mode()),
		},
	}

	// Initialize upload
	initResp, err := c.InitUpload(ctx, config, info.Size(), files)
	if err != nil {
		return nil, fmt.Errorf("failed to init upload: %w", err)
	}

	uploadID := initResp.UploadID

	// Upload file in chunks
	reader, err := transfer.NewChunkReader(localPath, transfer.DefaultChunkSize)
	if err != nil {
		c.CancelUpload(ctx, uploadID)
		return nil, fmt.Errorf("failed to open file: %w", err)
	}
	defer reader.Close()

	var transferred int64
	chunkIndex := 0

	for {
		select {
		case <-ctx.Done():
			c.CancelUpload(ctx, uploadID)
			return nil, ctx.Err()
		default:
		}

		chunk, err := reader.NextChunk(chunkIndex)
		if err != nil {
			c.CancelUpload(ctx, uploadID)
			return nil, fmt.Errorf("failed to read chunk: %w", err)
		}

		if chunk == nil {
			break
		}

		chunk.FilePath = filepath.Base(localPath)

		if err := c.UploadChunk(ctx, uploadID, chunk); err != nil {
			c.CancelUpload(ctx, uploadID)
			return nil, fmt.Errorf("failed to upload chunk: %w", err)
		}

		transferred += int64(chunk.Size)
		chunkIndex++

		if onProgress != nil {
			onProgress(transferred, info.Size(), filepath.Base(localPath))
		}
	}

	// Complete upload
	completeResp, err := c.CompleteUpload(ctx, uploadID, false, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to complete upload: %w", err)
	}

	return &UploadResult{
		UploadID: uploadID,
		Path:     completeResp.Path,
	}, nil
}
