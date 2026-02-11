// Package telemetry collects hardware metrics and streams them to the Hub.
package telemetry

import (
	"context"
	"log"
	"sync"
	"time"

	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

// SteamStatusFunc returns Steam running state and gaming mode.
type SteamStatusFunc func() (running bool, gamingMode bool)

// Collector reads hardware metrics on a ticker and sends them via a callback.
type Collector struct {
	mu     sync.Mutex
	sendFn func(protocol.TelemetryData)

	cancel context.CancelFunc

	// Previous CPU counters for delta calculation.
	prevIdle  uint64
	prevTotal uint64
	primed    bool

	// Optional Steam status callback.
	steamStatusFn SteamStatusFunc
}

// NewCollector creates a new telemetry collector.
// sendFn is called each tick with the collected metrics.
func NewCollector(sendFn func(protocol.TelemetryData)) *Collector {
	return &Collector{sendFn: sendFn}
}

// SetSteamStatusFunc sets the callback for reading Steam status.
func (c *Collector) SetSteamStatusFunc(fn SteamStatusFunc) {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.steamStatusFn = fn
}

// Start begins collecting metrics at the given interval (seconds).
// If already running, this is a no-op.
func (c *Collector) Start(intervalSec int) {
	c.mu.Lock()
	defer c.mu.Unlock()

	if c.cancel != nil {
		return // already running
	}

	if intervalSec < 1 {
		intervalSec = 2
	}

	ctx, cancel := context.WithCancel(context.Background())
	c.cancel = cancel

	// Prime CPU counters so the first real tick has a valid delta.
	idle, total := readCPUTimes()
	c.prevIdle = idle
	c.prevTotal = total
	c.primed = true

	go c.loop(ctx, time.Duration(intervalSec)*time.Second)
}

// Stop halts the collection loop.
func (c *Collector) Stop() {
	c.mu.Lock()
	defer c.mu.Unlock()

	if c.cancel != nil {
		c.cancel()
		c.cancel = nil
	}
	c.primed = false
}

// UpdateInterval restarts the collector with a new interval.
func (c *Collector) UpdateInterval(intervalSec int) {
	c.Stop()
	c.Start(intervalSec)
}

// IsRunning returns true if the collector is currently active.
func (c *Collector) IsRunning() bool {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.cancel != nil
}

// loop is the main ticker goroutine.
func (c *Collector) loop(ctx context.Context, interval time.Duration) {
	ticker := time.NewTicker(interval)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
			data := c.collect()
			if c.sendFn != nil {
				c.sendFn(data)
			}
		}
	}
}

// collect reads all available metrics and returns a TelemetryData snapshot.
func (c *Collector) collect() protocol.TelemetryData {
	data := protocol.TelemetryData{
		Timestamp: time.Now().UnixMilli(),
	}

	// CPU usage (delta-based) + frequency
	idle, total := readCPUTimes()
	c.mu.Lock()
	if c.primed && total > c.prevTotal {
		usage := calculateCPUUsage(c.prevIdle, c.prevTotal, idle, total)
		temp := readCPUTemp()
		freq := readCPUFreq()
		data.CPU = &protocol.CPUMetrics{
			UsagePercent: usage,
			TempCelsius:  temp,
			FreqMHz:      freq,
		}
	}
	c.prevIdle = idle
	c.prevTotal = total
	c.primed = true
	steamFn := c.steamStatusFn
	c.mu.Unlock()

	// GPU + frequency + VRAM
	gpuUsage := readGPUUsage()
	gpuTemp := readGPUTemp()
	gpuFreq := readGPUFreq()
	gpuMemFreq := readGPUMemFreq()
	vramUsed, vramTotal := readVRAMInfo()
	if gpuUsage >= 0 || gpuTemp >= 0 || gpuFreq >= 0 {
		data.GPU = &protocol.GPUMetrics{
			UsagePercent:   gpuUsage,
			TempCelsius:    gpuTemp,
			FreqMHz:        gpuFreq,
			MemFreqMHz:     gpuMemFreq,
			VRAMUsedBytes:  vramUsed,
			VRAMTotalBytes: vramTotal,
		}
	}

	// Memory + Swap
	memTotal, memAvailable, swapTotal, swapFree := readMemInfo()
	if memTotal > 0 {
		usagePercent := float64(memTotal-memAvailable) / float64(memTotal) * 100
		data.Memory = &protocol.MemoryMetrics{
			TotalBytes:     memTotal,
			AvailableBytes: memAvailable,
			UsagePercent:   usagePercent,
			SwapTotalBytes: swapTotal,
			SwapFreeBytes:  swapFree,
		}
	}

	// Battery
	capacity, status := readBattery()
	if capacity >= 0 {
		data.Battery = &protocol.BatteryMetrics{
			Capacity: capacity,
			Status:   status,
		}
	}

	// Power (TDP + draw)
	tdp, power := readPowerInfo()
	if tdp > 0 || power > 0 {
		data.Power = &protocol.PowerMetrics{
			TDPWatts:   tdp,
			PowerWatts: power,
		}
	}

	// Fan
	rpm := readFanSpeed()
	if rpm >= 0 {
		data.Fan = &protocol.FanMetrics{
			RPM: rpm,
		}
	}

	// Steam status
	if steamFn != nil {
		running, gamingMode := steamFn()
		data.Steam = &protocol.SteamStatus{
			Running:    running,
			GamingMode: gamingMode,
		}
	}

	if data.CPU != nil || data.GPU != nil || data.Memory != nil {
		log.Printf("Telemetry: CPU=%.1f%% GPU=%.1f%% MEM=%.1f%%",
			safePercent(data.CPU), safeGPU(data.GPU), safeMem(data.Memory))
	}

	return data
}

func safePercent(cpu *protocol.CPUMetrics) float64 {
	if cpu == nil {
		return -1
	}
	return cpu.UsagePercent
}

func safeGPU(gpu *protocol.GPUMetrics) float64 {
	if gpu == nil {
		return -1
	}
	return gpu.UsagePercent
}

func safeMem(mem *protocol.MemoryMetrics) float64 {
	if mem == nil {
		return -1
	}
	return mem.UsagePercent
}
