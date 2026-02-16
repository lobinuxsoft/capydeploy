package transfer

import (
	"sync"
	"testing"
	"time"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

func TestNewProgressTracker(t *testing.T) {
	tracker := NewProgressTracker(100 * time.Millisecond)

	if tracker == nil {
		t.Fatal("NewProgressTracker() returned nil")
	}
	if tracker.sessions == nil {
		t.Error("sessions map should not be nil")
	}
}

func TestNewProgressTracker_DefaultInterval(t *testing.T) {
	tracker := NewProgressTracker(0) // Should use default

	if tracker.interval != 500*time.Millisecond {
		t.Errorf("interval = %v, want 500ms", tracker.interval)
	}
}

func TestProgressTracker_Track(t *testing.T) {
	tracker := NewProgressTracker(100 * time.Millisecond)
	session := NewUploadSession("test-123", protocol.UploadConfig{}, 1000, nil)

	tracker.Track(session)

	got := tracker.GetSession("test-123")
	if got == nil {
		t.Fatal("GetSession() returned nil after Track()")
	}
	if got.ID != "test-123" {
		t.Errorf("Session ID = %q, want %q", got.ID, "test-123")
	}
}

func TestProgressTracker_Untrack(t *testing.T) {
	tracker := NewProgressTracker(100 * time.Millisecond)
	session := NewUploadSession("test-123", protocol.UploadConfig{}, 1000, nil)

	tracker.Track(session)
	tracker.Untrack("test-123")

	if got := tracker.GetSession("test-123"); got != nil {
		t.Error("GetSession() should return nil after Untrack()")
	}
}

func TestProgressTracker_GetSession_NotFound(t *testing.T) {
	tracker := NewProgressTracker(100 * time.Millisecond)

	if got := tracker.GetSession("nonexistent"); got != nil {
		t.Error("GetSession() should return nil for non-existent session")
	}
}

func TestProgressTracker_OnProgress(t *testing.T) {
	tracker := NewProgressTracker(100 * time.Millisecond)

	var received []protocol.UploadProgress
	var mu sync.Mutex

	tracker.OnProgress(func(p protocol.UploadProgress) {
		mu.Lock()
		received = append(received, p)
		mu.Unlock()
	})

	session := NewUploadSession("test", protocol.UploadConfig{}, 1000, nil)
	session.Start()
	tracker.Track(session)

	// Manually notify
	tracker.NotifyProgress("test")

	// Give callback time to execute
	time.Sleep(50 * time.Millisecond)

	mu.Lock()
	if len(received) == 0 {
		t.Error("Callback should have been called")
	}
	mu.Unlock()
}

func TestProgressTracker_NotifyProgress_NonExistent(t *testing.T) {
	tracker := NewProgressTracker(100 * time.Millisecond)

	// Should not panic
	tracker.NotifyProgress("nonexistent")
}

func TestProgressTracker_StartStop(t *testing.T) {
	tracker := NewProgressTracker(50 * time.Millisecond)

	var callCount int
	var mu sync.Mutex

	tracker.OnProgress(func(p protocol.UploadProgress) {
		mu.Lock()
		callCount++
		mu.Unlock()
	})

	session := NewUploadSession("test", protocol.UploadConfig{}, 1000, nil)
	session.Start()
	tracker.Track(session)

	tracker.Start()

	// Wait for a few ticks
	time.Sleep(200 * time.Millisecond)

	tracker.Stop()

	mu.Lock()
	count := callCount
	mu.Unlock()

	if count == 0 {
		t.Error("Start() should trigger periodic callbacks")
	}
}

func TestNewSpeedCalculator(t *testing.T) {
	calc := NewSpeedCalculator(5*time.Second, 100)

	if calc == nil {
		t.Fatal("NewSpeedCalculator() returned nil")
	}
}

func TestNewSpeedCalculator_Defaults(t *testing.T) {
	calc := NewSpeedCalculator(0, 0)

	if calc.maxSamples != 100 {
		t.Errorf("maxSamples = %d, want 100", calc.maxSamples)
	}
	if calc.windowSize != 5*time.Second {
		t.Errorf("windowSize = %v, want 5s", calc.windowSize)
	}
}

func TestSpeedCalculator_AddSample(t *testing.T) {
	calc := NewSpeedCalculator(5*time.Second, 100)

	calc.AddSample(1024)
	calc.AddSample(2048)

	if len(calc.samples) != 2 {
		t.Errorf("samples length = %d, want 2", len(calc.samples))
	}
}

func TestSpeedCalculator_BytesPerSecond_NoSamples(t *testing.T) {
	calc := NewSpeedCalculator(5*time.Second, 100)

	if speed := calc.BytesPerSecond(); speed != 0 {
		t.Errorf("BytesPerSecond() with no samples = %f, want 0", speed)
	}
}

func TestSpeedCalculator_BytesPerSecond_OneSample(t *testing.T) {
	calc := NewSpeedCalculator(5*time.Second, 100)
	calc.AddSample(1024)

	// Need at least 2 samples
	if speed := calc.BytesPerSecond(); speed != 0 {
		t.Errorf("BytesPerSecond() with one sample = %f, want 0", speed)
	}
}

func TestSpeedCalculator_BytesPerSecond(t *testing.T) {
	calc := NewSpeedCalculator(5*time.Second, 100)

	// Add samples with known timing
	calc.AddSample(1000)
	time.Sleep(100 * time.Millisecond)
	calc.AddSample(1000)

	speed := calc.BytesPerSecond()
	// Should be approximately 20000 bytes/second (2000 bytes / 0.1 seconds)
	// Allow some tolerance for timing variations
	if speed < 10000 || speed > 30000 {
		t.Errorf("BytesPerSecond() = %f, expected around 20000", speed)
	}
}

func TestSpeedCalculator_ETA(t *testing.T) {
	calc := NewSpeedCalculator(5*time.Second, 100)

	// No samples, should return 0
	if eta := calc.ETA(1000); eta != 0 {
		t.Errorf("ETA() with no samples = %v, want 0", eta)
	}

	// Add samples with enough time between them
	calc.AddSample(1000)
	time.Sleep(200 * time.Millisecond)
	calc.AddSample(1000)

	// With ~10000 bytes/sec and 10000 remaining, should be ~1 second
	eta := calc.ETA(10000)
	if eta <= 0 {
		t.Errorf("ETA() = %v, should be positive with samples", eta)
	}
}

func TestSpeedCalculator_Reset(t *testing.T) {
	calc := NewSpeedCalculator(5*time.Second, 100)

	calc.AddSample(1000)
	calc.AddSample(2000)

	calc.Reset()

	if len(calc.samples) != 0 {
		t.Errorf("samples length after Reset() = %d, want 0", len(calc.samples))
	}
}

func TestSpeedCalculator_PrunesOldSamples(t *testing.T) {
	calc := NewSpeedCalculator(50*time.Millisecond, 100) // Short window

	calc.AddSample(1000)
	time.Sleep(100 * time.Millisecond) // Wait longer than window
	calc.AddSample(1000)               // This should prune the old sample

	// Old sample should be pruned
	if len(calc.samples) > 1 {
		t.Errorf("Old samples should be pruned, got %d samples", len(calc.samples))
	}
}

func TestSpeedCalculator_LimitsSampleCount(t *testing.T) {
	calc := NewSpeedCalculator(1*time.Hour, 5) // Large window, small max

	for i := 0; i < 10; i++ {
		calc.AddSample(1000)
	}

	if len(calc.samples) > 5 {
		t.Errorf("samples length = %d, should be limited to 5", len(calc.samples))
	}
}

func TestSpeedCalculator_ConcurrentAccess(t *testing.T) {
	calc := NewSpeedCalculator(5*time.Second, 100)

	done := make(chan bool)

	// Concurrent writers
	for i := 0; i < 10; i++ {
		go func() {
			for j := 0; j < 100; j++ {
				calc.AddSample(1000)
			}
			done <- true
		}()
	}

	// Concurrent readers
	for i := 0; i < 10; i++ {
		go func() {
			for j := 0; j < 100; j++ {
				_ = calc.BytesPerSecond()
				_ = calc.ETA(10000)
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

func TestProgressTracker_ConcurrentAccess(t *testing.T) {
	tracker := NewProgressTracker(100 * time.Millisecond)

	done := make(chan bool)

	// Concurrent track/untrack
	for i := 0; i < 10; i++ {
		go func(idx int) {
			session := NewUploadSession("session-"+string(rune('A'+idx)), protocol.UploadConfig{}, 1000, nil)
			for j := 0; j < 50; j++ {
				tracker.Track(session)
				tracker.GetSession(session.ID)
				tracker.Untrack(session.ID)
			}
			done <- true
		}(i)
	}

	// Wait for all goroutines
	for i := 0; i < 10; i++ {
		select {
		case <-done:
		case <-time.After(5 * time.Second):
			t.Fatal("Timeout waiting for concurrent operations")
		}
	}
}
