// Package logwriter provides crash-safe file logging for console/game log entries.
package logwriter

import (
	"fmt"
	"os"
	"path/filepath"
	"sync"
	"time"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

// Writer writes console log entries to a text file, one line per entry.
// Syncs after every write for crash safety.
type Writer struct {
	dir  string
	file *os.File
	mu   sync.Mutex
}

// New creates a Writer that logs to the given directory.
// A new log file is created with a timestamp-based name.
// Returns nil if dir is empty (disabled).
func New(dir string) (*Writer, error) {
	if dir == "" {
		return nil, nil
	}

	if err := os.MkdirAll(dir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create log directory: %w", err)
	}

	filename := fmt.Sprintf("log_%s.txt", time.Now().Format("2006-01-02_15-04-05"))
	path := filepath.Join(dir, filename)

	f, err := os.OpenFile(path, os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0644)
	if err != nil {
		return nil, fmt.Errorf("failed to open log file: %w", err)
	}

	return &Writer{dir: dir, file: f}, nil
}

// Write appends a single console log entry to the file.
func (w *Writer) Write(entry protocol.ConsoleLogEntry) error {
	ts := time.UnixMilli(entry.Timestamp).Format("15:04:05.000")
	line := fmt.Sprintf("[%s] [%s] [%s] %s\n", ts, entry.Level, entry.Source, entry.Text)

	w.mu.Lock()
	defer w.mu.Unlock()

	if w.file == nil {
		return fmt.Errorf("log file closed")
	}

	if _, err := w.file.WriteString(line); err != nil {
		return err
	}
	return w.file.Sync()
}

// WriteBatch writes multiple entries at once.
func (w *Writer) WriteBatch(entries []protocol.ConsoleLogEntry) error {
	w.mu.Lock()
	defer w.mu.Unlock()

	if w.file == nil {
		return fmt.Errorf("log file closed")
	}

	for _, entry := range entries {
		ts := time.UnixMilli(entry.Timestamp).Format("15:04:05.000")
		line := fmt.Sprintf("[%s] [%s] [%s] %s\n", ts, entry.Level, entry.Source, entry.Text)
		if _, err := w.file.WriteString(line); err != nil {
			return err
		}
	}
	return w.file.Sync()
}

// Close closes the log file.
func (w *Writer) Close() error {
	w.mu.Lock()
	defer w.mu.Unlock()

	if w.file != nil {
		err := w.file.Close()
		w.file = nil
		return err
	}
	return nil
}
