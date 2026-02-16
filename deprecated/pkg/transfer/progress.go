package transfer

import (
	"sync"
	"time"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

// ProgressCallback is called when upload progress changes.
type ProgressCallback func(progress protocol.UploadProgress)

// ProgressTracker tracks upload progress and notifies callbacks.
type ProgressTracker struct {
	mu        sync.RWMutex
	callbacks []ProgressCallback
	sessions  map[string]*UploadSession
	interval  time.Duration
	stopCh    chan struct{}
	stopOnce  sync.Once
}

// NewProgressTracker creates a new progress tracker.
func NewProgressTracker(updateInterval time.Duration) *ProgressTracker {
	if updateInterval <= 0 {
		updateInterval = 500 * time.Millisecond
	}
	return &ProgressTracker{
		sessions: make(map[string]*UploadSession),
		interval: updateInterval,
		stopCh:   make(chan struct{}),
	}
}

// OnProgress registers a callback for progress updates.
func (t *ProgressTracker) OnProgress(cb ProgressCallback) {
	t.mu.Lock()
	defer t.mu.Unlock()
	t.callbacks = append(t.callbacks, cb)
}

// Track starts tracking a session.
func (t *ProgressTracker) Track(session *UploadSession) {
	t.mu.Lock()
	defer t.mu.Unlock()
	t.sessions[session.ID] = session
}

// Untrack stops tracking a session.
func (t *ProgressTracker) Untrack(sessionID string) {
	t.mu.Lock()
	defer t.mu.Unlock()
	delete(t.sessions, sessionID)
}

// GetSession returns a tracked session by ID.
func (t *ProgressTracker) GetSession(sessionID string) *UploadSession {
	t.mu.RLock()
	defer t.mu.RUnlock()
	return t.sessions[sessionID]
}

// notify sends progress updates to all callbacks.
func (t *ProgressTracker) notify(progress protocol.UploadProgress) {
	t.mu.RLock()
	callbacks := make([]ProgressCallback, len(t.callbacks))
	copy(callbacks, t.callbacks)
	t.mu.RUnlock()

	for _, cb := range callbacks {
		cb(progress)
	}
}

// NotifyProgress manually triggers a progress notification for a session.
func (t *ProgressTracker) NotifyProgress(sessionID string) {
	t.mu.RLock()
	session, ok := t.sessions[sessionID]
	t.mu.RUnlock()

	if ok {
		t.notify(session.Progress())
	}
}

// Start begins periodic progress notifications.
func (t *ProgressTracker) Start() {
	go func() {
		ticker := time.NewTicker(t.interval)
		defer ticker.Stop()

		for {
			select {
			case <-ticker.C:
				t.mu.RLock()
				for _, session := range t.sessions {
					if session.IsActive() {
						t.notify(session.Progress())
					}
				}
				t.mu.RUnlock()
			case <-t.stopCh:
				return
			}
		}
	}()
}

// Stop stops the progress tracker. Safe to call multiple times.
func (t *ProgressTracker) Stop() {
	t.stopOnce.Do(func() {
		close(t.stopCh)
	})
}

// SpeedCalculator calculates transfer speed.
type SpeedCalculator struct {
	mu          sync.Mutex
	samples     []speedSample
	maxSamples  int
	windowSize  time.Duration
}

type speedSample struct {
	bytes     int64
	timestamp time.Time
}

// NewSpeedCalculator creates a new speed calculator.
func NewSpeedCalculator(windowSize time.Duration, maxSamples int) *SpeedCalculator {
	if maxSamples <= 0 {
		maxSamples = 100
	}
	if windowSize <= 0 {
		windowSize = 5 * time.Second
	}
	return &SpeedCalculator{
		samples:    make([]speedSample, 0, maxSamples),
		maxSamples: maxSamples,
		windowSize: windowSize,
	}
}

// AddSample adds a new byte count sample.
func (c *SpeedCalculator) AddSample(bytes int64) {
	c.mu.Lock()
	defer c.mu.Unlock()

	now := time.Now()
	c.samples = append(c.samples, speedSample{bytes: bytes, timestamp: now})

	// Remove old samples
	cutoff := now.Add(-c.windowSize)
	for len(c.samples) > 0 && c.samples[0].timestamp.Before(cutoff) {
		c.samples = c.samples[1:]
	}

	// Limit sample count
	if len(c.samples) > c.maxSamples {
		c.samples = c.samples[len(c.samples)-c.maxSamples:]
	}
}

// BytesPerSecond returns the current transfer speed.
func (c *SpeedCalculator) BytesPerSecond() float64 {
	c.mu.Lock()
	defer c.mu.Unlock()

	if len(c.samples) < 2 {
		return 0
	}

	first := c.samples[0]
	last := c.samples[len(c.samples)-1]
	duration := last.timestamp.Sub(first.timestamp).Seconds()
	if duration <= 0 {
		return 0
	}

	var totalBytes int64
	for _, s := range c.samples {
		totalBytes += s.bytes
	}

	return float64(totalBytes) / duration
}

// ETA estimates time remaining based on current speed.
func (c *SpeedCalculator) ETA(remainingBytes int64) time.Duration {
	speed := c.BytesPerSecond()
	if speed <= 0 {
		return 0
	}
	return time.Duration(float64(remainingBytes)/speed) * time.Second
}

// Reset clears all samples.
func (c *SpeedCalculator) Reset() {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.samples = c.samples[:0]
}
