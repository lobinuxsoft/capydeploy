package consolelog

import (
	"context"
	"encoding/json"
	"log"
	"sync"
	"time"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

const (
	maxBufferSize = 200
	maxBatchSize  = 50
	flushInterval = 500 * time.Millisecond
	maxRetries    = 3
)

// Collector streams console logs from Steam's CEF debugger to the Hub.
type Collector struct {
	mu     sync.Mutex
	sendFn func(protocol.ConsoleLogBatch)
	cancel context.CancelFunc

	// Ring buffer state
	buffer  []protocol.ConsoleLogEntry
	dropped int
}

// NewCollector creates a new console log collector.
// sendFn is called each flush with a batch of entries.
func NewCollector(sendFn func(protocol.ConsoleLogBatch)) *Collector {
	return &Collector{sendFn: sendFn}
}

// Start begins streaming console logs from CDP.
// If already running, this is a no-op.
func (c *Collector) Start() {
	c.mu.Lock()
	defer c.mu.Unlock()

	if c.cancel != nil {
		return // already running
	}

	ctx, cancel := context.WithCancel(context.Background())
	c.cancel = cancel

	go c.loop(ctx)
}

// Stop halts the console log streaming.
func (c *Collector) Stop() {
	c.mu.Lock()
	defer c.mu.Unlock()

	if c.cancel != nil {
		c.cancel()
		c.cancel = nil
	}
	c.buffer = nil
	c.dropped = 0
}

// IsRunning returns true if the collector is currently active.
func (c *Collector) IsRunning() bool {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.cancel != nil
}

// loop is the main goroutine that manages CDP connection and reading.
func (c *Collector) loop(ctx context.Context) {
	backoffs := []time.Duration{1 * time.Second, 2 * time.Second, 4 * time.Second}

	for attempt := 0; ; attempt++ {
		if ctx.Err() != nil {
			return
		}

		if attempt > 0 {
			if attempt > maxRetries {
				log.Printf("[consolelog] max retries exceeded, stopping")
				c.mu.Lock()
				c.cancel = nil
				c.mu.Unlock()
				return
			}
			delay := backoffs[attempt-1]
			log.Printf("[consolelog] reconnecting in %v (attempt %d/%d)", delay, attempt, maxRetries)
			select {
			case <-ctx.Done():
				return
			case <-time.After(delay):
			}
		}

		conn, err := dialCDP(ctx)
		if err != nil {
			log.Printf("[consolelog] failed to connect to CDP: %v", err)
			continue
		}

		if err := conn.enableRuntime(); err != nil {
			log.Printf("[consolelog] failed to enable Runtime: %v", err)
			conn.close()
			continue
		}

		if err := conn.enableLog(); err != nil {
			log.Printf("[consolelog] failed to enable Log: %v", err)
			conn.close()
			continue
		}

		log.Printf("[consolelog] connected to CDP, streaming console logs")
		attempt = 0 // Reset attempts on successful connection

		// Run the read+flush loop until error or cancellation.
		// readLoop closes conn via ctx watchdog on cancel, so only
		// close here on error (reconnect path).
		if err := c.readLoop(ctx, conn); err != nil {
			log.Printf("[consolelog] read loop error: %v", err)
			conn.close()
			continue
		}

		return // Context cancelled, conn already closed by readLoop
	}
}

// readLoop reads CDP events and flushes batches periodically.
func (c *Collector) readLoop(ctx context.Context, conn *cdpConn) error {
	// Channel for events from readEvent goroutine
	eventCh := make(chan *cdpEvent, 64)
	errCh := make(chan error, 1)

	// Close conn when ctx is cancelled to unblock the reader goroutine.
	// This ensures the goroutine exits even if readEvent blocks forever.
	go func() {
		<-ctx.Done()
		conn.close()
	}()

	go func() {
		for {
			event, err := conn.readEvent()
			if err != nil {
				errCh <- err
				return
			}
			select {
			case eventCh <- event:
			default:
				// Drop if channel full
			}
		}
	}()

	flushTicker := time.NewTicker(flushInterval)
	defer flushTicker.Stop()

	for {
		select {
		case <-ctx.Done():
			c.flush() // Final flush
			return nil

		case err := <-errCh:
			if ctx.Err() != nil {
				return nil // Context cancelled, not a real error
			}
			c.flush() // Flush before reconnect
			return err

		case event := <-eventCh:
			c.handleCDPEvent(event)

		case <-flushTicker.C:
			c.flush()
		}
	}
}

// skippedLevels are log levels filtered out at collection time to avoid
// buffer pollution from framework noise (e.g. Decky WSRouter debug spam).
var skippedLevels = map[string]bool{
	"debug":   true,
	"verbose": true,
}

// handleCDPEvent processes a CDP event and adds entries to the buffer.
func (c *Collector) handleCDPEvent(event *cdpEvent) {
	switch event.Method {
	case "Runtime.consoleAPICalled":
		var params consoleAPICalledParams
		if err := json.Unmarshal(event.Params, &params); err != nil {
			return
		}
		if skippedLevels[params.Type] {
			return
		}
		parsed := formatConsoleArgsRich(params.Args)
		if parsed.Text == "" {
			return
		}
		c.addEntry(protocol.ConsoleLogEntry{
			Timestamp: time.Now().UnixMilli(),
			Level:     params.Type, // "log", "warn", "error", "info"
			Source:    "console",
			Text:      parsed.Text,
			Segments:  parsed.Segments,
		})

	case "Log.entryAdded":
		var params logEntryAddedParams
		if err := json.Unmarshal(event.Params, &params); err != nil {
			return
		}
		if skippedLevels[params.Entry.Level] {
			return
		}
		c.addEntry(protocol.ConsoleLogEntry{
			Timestamp: time.Now().UnixMilli(),
			Level:     params.Entry.Level,
			Source:    params.Entry.Source,
			Text:      params.Entry.Text,
			URL:       params.Entry.URL,
			Line:      params.Entry.Line,
		})
	}
}

// addEntry adds a log entry to the ring buffer.
func (c *Collector) addEntry(entry protocol.ConsoleLogEntry) {
	c.mu.Lock()
	defer c.mu.Unlock()

	if len(c.buffer) >= maxBufferSize {
		// Drop oldest entry
		c.buffer = c.buffer[1:]
		c.dropped++
	}
	c.buffer = append(c.buffer, entry)
}

// flush sends buffered entries as a batch and clears the buffer.
func (c *Collector) flush() {
	c.mu.Lock()
	if len(c.buffer) == 0 || c.cancel == nil {
		c.mu.Unlock()
		return
	}

	// Take up to maxBatchSize entries
	n := len(c.buffer)
	if n > maxBatchSize {
		n = maxBatchSize
	}

	batch := protocol.ConsoleLogBatch{
		Entries: make([]protocol.ConsoleLogEntry, n),
		Dropped: c.dropped,
	}
	copy(batch.Entries, c.buffer[:n])

	// Remove sent entries from buffer
	c.buffer = c.buffer[n:]
	c.dropped = 0
	c.mu.Unlock()

	if c.sendFn != nil {
		c.sendFn(batch)
	}
}
