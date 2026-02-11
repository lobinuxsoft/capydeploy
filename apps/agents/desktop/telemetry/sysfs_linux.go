//go:build linux

package telemetry

import (
	"math"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
)

// sysfsCache holds resolved sysfs paths so we don't re-glob every tick.
// Linux hwmon numbering is stable per boot.
type sysfsCache struct {
	once sync.Once

	cpuTempPath  string
	gpuBusyPath  string
	gpuTempPath  string
	gpuFreqPath  string
	gpuMemFreq   string
	vramUsedPath string
	vramTotal    string
	powerCapPath string
	powerAvgPath string
	fanPath      string
	batteryPath  string
	cpuFreqPaths []string
}

var paths sysfsCache

func (c *sysfsCache) resolve() {
	c.once.Do(func() {
		// CPU temperature: k10temp (AMD) or coretemp (Intel)
		hwmonDir := "/sys/class/hwmon"
		entries, err := os.ReadDir(hwmonDir)
		if err == nil {
			for _, entry := range entries {
				base := filepath.Join(hwmonDir, entry.Name())
				nameBytes, err := os.ReadFile(filepath.Join(base, "name"))
				if err != nil {
					continue
				}
				name := strings.TrimSpace(string(nameBytes))

				if (name == "k10temp" || name == "coretemp") && c.cpuTempPath == "" {
					c.cpuTempPath = filepath.Join(base, "temp1_input")
				}
				if fanPath := filepath.Join(base, "fan1_input"); fileExists(fanPath) && c.fanPath == "" {
					c.fanPath = fanPath
				}
				if capPath := filepath.Join(base, "power1_cap"); fileExists(capPath) && c.powerCapPath == "" {
					c.powerCapPath = capPath
				}
				for _, pName := range []string{"power1_average", "power1_input"} {
					pPath := filepath.Join(base, pName)
					if fileExists(pPath) && c.powerAvgPath == "" {
						c.powerAvgPath = pPath
						break
					}
				}
			}
		}

		// GPU paths (AMDGPU — card0/card1)
		cards, _ := filepath.Glob("/sys/class/drm/card[0-9]")
		for _, card := range cards {
			busy := filepath.Join(card, "device", "gpu_busy_percent")
			if !fileExists(busy) {
				continue
			}
			c.gpuBusyPath = busy

			hwmons, _ := filepath.Glob(filepath.Join(card, "device", "hwmon", "hwmon*"))
			for _, hwmon := range hwmons {
				temp := filepath.Join(hwmon, "temp1_input")
				if fileExists(temp) {
					c.gpuTempPath = temp
					break
				}
			}

			if freq := filepath.Join(card, "device", "pp_dpm_sclk"); fileExists(freq) {
				c.gpuFreqPath = freq
			}
			if mclk := filepath.Join(card, "device", "pp_dpm_mclk"); fileExists(mclk) {
				c.gpuMemFreq = mclk
			}
			if vt := filepath.Join(card, "device", "mem_info_vram_total"); fileExists(vt) {
				c.vramTotal = vt
			}
			if vu := filepath.Join(card, "device", "mem_info_vram_used"); fileExists(vu) {
				c.vramUsedPath = vu
			}
			break
		}

		// Battery
		bats, _ := filepath.Glob("/sys/class/power_supply/BAT*")
		if len(bats) > 0 {
			c.batteryPath = bats[0]
		}

		// CPU frequency paths
		c.cpuFreqPaths, _ = filepath.Glob("/sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq")
	})
}

func fileExists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}

// readCPUTimes parses /proc/stat to get aggregate CPU idle and total jiffies.
func readCPUTimes() (idle, total uint64) {
	data, err := os.ReadFile("/proc/stat")
	if err != nil {
		return 0, 0
	}

	// First line: "cpu  user nice system idle iowait irq softirq steal ..."
	lines := strings.SplitN(string(data), "\n", 2)
	if len(lines) == 0 {
		return 0, 0
	}

	fields := strings.Fields(lines[0])
	if len(fields) < 5 || fields[0] != "cpu" {
		return 0, 0
	}

	var sum uint64
	for i := 1; i < len(fields); i++ {
		v, err := strconv.ParseUint(fields[i], 10, 64)
		if err != nil {
			continue
		}
		sum += v
		if i == 4 { // idle is the 4th value (index 4 after "cpu")
			idle = v
		}
	}

	return idle, sum
}

// calculateCPUUsage computes CPU usage percentage from two snapshots.
func calculateCPUUsage(prevIdle, prevTotal, currIdle, currTotal uint64) float64 {
	deltaTotal := currTotal - prevTotal
	deltaIdle := currIdle - prevIdle

	if deltaTotal == 0 {
		return 0
	}

	usage := (1.0 - float64(deltaIdle)/float64(deltaTotal)) * 100
	return math.Round(usage*10) / 10
}

// readCPUTemp reads CPU temperature in degrees C from cached hwmon path.
func readCPUTemp() float64 {
	paths.resolve()
	if paths.cpuTempPath == "" {
		return -1
	}

	tempBytes, err := os.ReadFile(paths.cpuTempPath)
	if err != nil {
		return -1
	}

	millideg, err := strconv.ParseInt(strings.TrimSpace(string(tempBytes)), 10, 64)
	if err != nil {
		return -1
	}

	return float64(millideg) / 1000.0
}

// readGPUUsage reads GPU busy percentage from cached sysfs path.
func readGPUUsage() float64 {
	paths.resolve()
	if paths.gpuBusyPath == "" {
		return -1
	}

	data, err := os.ReadFile(paths.gpuBusyPath)
	if err != nil {
		return -1
	}

	val, err := strconv.ParseFloat(strings.TrimSpace(string(data)), 64)
	if err != nil {
		return -1
	}

	return val
}

// readGPUTemp reads GPU temperature from cached hwmon path.
func readGPUTemp() float64 {
	paths.resolve()
	if paths.gpuTempPath == "" {
		return -1
	}

	data, err := os.ReadFile(paths.gpuTempPath)
	if err != nil {
		return -1
	}

	millideg, err := strconv.ParseInt(strings.TrimSpace(string(data)), 10, 64)
	if err != nil {
		return -1
	}

	return float64(millideg) / 1000.0
}

// readMemInfo parses /proc/meminfo to get total, available, swap total and swap free in bytes.
func readMemInfo() (total, available, swapTotal, swapFree int64) {
	data, err := os.ReadFile("/proc/meminfo")
	if err != nil {
		return -1, -1, -1, -1
	}

	var gotTotal, gotAvailable, gotSwapTotal, gotSwapFree bool

	for _, line := range strings.Split(string(data), "\n") {
		fields := strings.Fields(line)
		if len(fields) < 2 {
			continue
		}

		val, err := strconv.ParseInt(fields[1], 10, 64)
		if err != nil {
			continue
		}

		// Values in /proc/meminfo are in kB
		switch fields[0] {
		case "MemTotal:":
			total = val * 1024
			gotTotal = true
		case "MemAvailable:":
			available = val * 1024
			gotAvailable = true
		case "SwapTotal:":
			swapTotal = val * 1024
			gotSwapTotal = true
		case "SwapFree:":
			swapFree = val * 1024
			gotSwapFree = true
		}

		if gotTotal && gotAvailable && gotSwapTotal && gotSwapFree {
			break
		}
	}

	if !gotTotal {
		return -1, -1, -1, -1
	}
	if !gotSwapTotal {
		swapTotal = 0
	}
	if !gotSwapFree {
		swapFree = 0
	}

	return total, available, swapTotal, swapFree
}

// readBattery reads battery capacity and status from cached path.
func readBattery() (capacity int, status string) {
	paths.resolve()
	if paths.batteryPath == "" {
		return -1, ""
	}

	capBytes, err := os.ReadFile(filepath.Join(paths.batteryPath, "capacity"))
	if err != nil {
		return -1, ""
	}

	cap, err := strconv.Atoi(strings.TrimSpace(string(capBytes)))
	if err != nil {
		return -1, ""
	}

	statusBytes, err := os.ReadFile(filepath.Join(paths.batteryPath, "status"))
	if err != nil {
		return cap, "Unknown"
	}

	return cap, strings.TrimSpace(string(statusBytes))
}

// readCPUFreq returns the average CPU frequency across all cores in MHz.
func readCPUFreq() float64 {
	paths.resolve()
	if len(paths.cpuFreqPaths) == 0 {
		return -1
	}

	var total float64
	var count int
	for _, path := range paths.cpuFreqPaths {
		data, err := os.ReadFile(path)
		if err != nil {
			continue
		}
		// Value is in kHz
		val, err := strconv.ParseFloat(strings.TrimSpace(string(data)), 64)
		if err != nil {
			continue
		}
		total += val
		count++
	}

	if count == 0 {
		return -1
	}

	return math.Round(total / float64(count) / 1000)
}

// readGPUFreq reads the current GPU frequency from pp_dpm_sclk (AMD) in MHz.
func readGPUFreq() float64 {
	paths.resolve()
	if paths.gpuFreqPath == "" {
		return -1
	}

	data, err := os.ReadFile(paths.gpuFreqPath)
	if err != nil {
		return -1
	}

	return parseDPMFreq(string(data))
}

// readPowerInfo reads TDP cap and current power draw from cached hwmon paths in watts.
func readPowerInfo() (tdpWatts, powerWatts float64) {
	paths.resolve()
	tdpWatts = -1
	powerWatts = -1

	if paths.powerCapPath != "" {
		if capData, err := os.ReadFile(paths.powerCapPath); err == nil {
			if val, err := strconv.ParseInt(strings.TrimSpace(string(capData)), 10, 64); err == nil {
				tdpWatts = float64(val) / 1000000.0
			}
		}
	}

	if paths.powerAvgPath != "" {
		if powerData, err := os.ReadFile(paths.powerAvgPath); err == nil {
			if val, err := strconv.ParseInt(strings.TrimSpace(string(powerData)), 10, 64); err == nil {
				powerWatts = float64(val) / 1000000.0
			}
		}
	}

	return tdpWatts, powerWatts
}

// readVRAMInfo reads VRAM used and total bytes from cached sysfs paths.
func readVRAMInfo() (used, total int64) {
	paths.resolve()
	if paths.vramTotal == "" {
		return -1, -1
	}

	totalData, err := os.ReadFile(paths.vramTotal)
	if err != nil {
		return -1, -1
	}
	totalVal, err := strconv.ParseInt(strings.TrimSpace(string(totalData)), 10, 64)
	if err != nil {
		return -1, -1
	}

	if paths.vramUsedPath == "" {
		return -1, totalVal
	}

	usedData, err := os.ReadFile(paths.vramUsedPath)
	if err != nil {
		return -1, totalVal
	}
	usedVal, err := strconv.ParseInt(strings.TrimSpace(string(usedData)), 10, 64)
	if err != nil {
		return -1, totalVal
	}

	return usedVal, totalVal
}

// readGPUMemFreq reads GPU memory clock from pp_dpm_mclk (AMD) in MHz.
func readGPUMemFreq() float64 {
	paths.resolve()
	if paths.gpuMemFreq == "" {
		return -1
	}

	data, err := os.ReadFile(paths.gpuMemFreq)
	if err != nil {
		return -1
	}

	return parseDPMFreq(string(data))
}

// parseDPMFreq extracts the active frequency from a pp_dpm_* file.
// Looks for the line marked with *, falls back to the last entry.
func parseDPMFreq(content string) float64 {
	var lastFreq float64 = -1

	for _, line := range strings.Split(content, "\n") {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}

		fields := strings.Fields(line)
		if len(fields) < 2 {
			continue
		}

		freqStr := strings.TrimSuffix(fields[1], "Mhz")
		val, err := strconv.ParseFloat(freqStr, 64)
		if err != nil {
			continue
		}

		if strings.Contains(line, "*") {
			return val
		}
		lastFreq = val
	}

	return lastFreq
}

// readFanSpeed reads the first available fan speed in RPM from cached hwmon path.
func readFanSpeed() int {
	paths.resolve()
	if paths.fanPath == "" {
		return -1
	}

	data, err := os.ReadFile(paths.fanPath)
	if err != nil {
		return -1
	}

	if val, err := strconv.Atoi(strings.TrimSpace(string(data))); err == nil {
		return val
	}

	return -1
}
