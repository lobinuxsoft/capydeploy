//go:build linux

package telemetry

import (
	"math"
	"os"
	"path/filepath"
	"strconv"
	"strings"
)

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

// readCPUTemp scans /sys/class/hwmon/ for k10temp or coretemp and returns degrees C.
func readCPUTemp() float64 {
	hwmonDir := "/sys/class/hwmon"
	entries, err := os.ReadDir(hwmonDir)
	if err != nil {
		return -1
	}

	for _, entry := range entries {
		namePath := filepath.Join(hwmonDir, entry.Name(), "name")
		nameBytes, err := os.ReadFile(namePath)
		if err != nil {
			continue
		}

		name := strings.TrimSpace(string(nameBytes))
		if name != "k10temp" && name != "coretemp" {
			continue
		}

		// Read temp1_input (millidegrees C)
		tempPath := filepath.Join(hwmonDir, entry.Name(), "temp1_input")
		tempBytes, err := os.ReadFile(tempPath)
		if err != nil {
			continue
		}

		millideg, err := strconv.ParseInt(strings.TrimSpace(string(tempBytes)), 10, 64)
		if err != nil {
			continue
		}

		return float64(millideg) / 1000.0
	}

	return -1
}

// readGPUUsage reads GPU busy percentage from /sys/class/drm/card*/device/gpu_busy_percent.
func readGPUUsage() float64 {
	matches, _ := filepath.Glob("/sys/class/drm/card*/device/gpu_busy_percent")
	for _, path := range matches {
		data, err := os.ReadFile(path)
		if err != nil {
			continue
		}

		val, err := strconv.ParseFloat(strings.TrimSpace(string(data)), 64)
		if err != nil {
			continue
		}

		return val
	}

	return -1
}

// readGPUTemp reads GPU temperature from /sys/class/drm/card*/device/hwmon/hwmon*/temp1_input.
func readGPUTemp() float64 {
	matches, _ := filepath.Glob("/sys/class/drm/card*/device/hwmon/hwmon*/temp1_input")
	for _, path := range matches {
		data, err := os.ReadFile(path)
		if err != nil {
			continue
		}

		millideg, err := strconv.ParseInt(strings.TrimSpace(string(data)), 10, 64)
		if err != nil {
			continue
		}

		return float64(millideg) / 1000.0
	}

	return -1
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

// readBattery reads battery capacity and status from /sys/class/power_supply/BAT*.
func readBattery() (capacity int, status string) {
	matches, _ := filepath.Glob("/sys/class/power_supply/BAT*")
	if len(matches) == 0 {
		return -1, ""
	}

	batPath := matches[0]

	capBytes, err := os.ReadFile(filepath.Join(batPath, "capacity"))
	if err != nil {
		return -1, ""
	}

	cap, err := strconv.Atoi(strings.TrimSpace(string(capBytes)))
	if err != nil {
		return -1, ""
	}

	statusBytes, err := os.ReadFile(filepath.Join(batPath, "status"))
	if err != nil {
		return cap, "Unknown"
	}

	return cap, strings.TrimSpace(string(statusBytes))
}

// readCPUFreq returns the average CPU frequency across all cores in MHz.
func readCPUFreq() float64 {
	matches, _ := filepath.Glob("/sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq")
	if len(matches) == 0 {
		return -1
	}

	var total float64
	var count int
	for _, path := range matches {
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
// The active frequency line is marked with *. If no line is marked,
// falls back to the highest level (last entry).
func readGPUFreq() float64 {
	matches, _ := filepath.Glob("/sys/class/drm/card*/device/pp_dpm_sclk")
	for _, path := range matches {
		data, err := os.ReadFile(path)
		if err != nil {
			continue
		}

		return parseDPMFreq(string(data))
	}

	return -1
}

// readPowerInfo reads TDP cap and current power draw from hwmon in watts.
// Scans for any hwmon with power1_cap or power1_average/power1_input.
func readPowerInfo() (tdpWatts, powerWatts float64) {
	tdpWatts = -1
	powerWatts = -1

	hwmonDir := "/sys/class/hwmon"
	entries, err := os.ReadDir(hwmonDir)
	if err != nil {
		return tdpWatts, powerWatts
	}

	for _, entry := range entries {
		base := filepath.Join(hwmonDir, entry.Name())

		// TDP cap (power1_cap) — in microwatts
		capPath := filepath.Join(base, "power1_cap")
		if capData, err := os.ReadFile(capPath); err == nil {
			if val, err := strconv.ParseInt(strings.TrimSpace(string(capData)), 10, 64); err == nil {
				tdpWatts = float64(val) / 1000000.0
			}
		}

		// Power draw: prefer power1_average, then power1_input — in microwatts
		for _, name := range []string{"power1_average", "power1_input"} {
			powerPath := filepath.Join(base, name)
			powerData, err := os.ReadFile(powerPath)
			if err != nil {
				continue
			}
			if val, err := strconv.ParseInt(strings.TrimSpace(string(powerData)), 10, 64); err == nil {
				powerWatts = float64(val) / 1000000.0
				break
			}
		}

		if tdpWatts > 0 || powerWatts > 0 {
			return tdpWatts, powerWatts
		}
	}

	return tdpWatts, powerWatts
}

// readVRAMInfo reads VRAM used and total bytes from AMDGPU sysfs.
func readVRAMInfo() (used, total int64) {
	matches, _ := filepath.Glob("/sys/class/drm/card*/device/mem_info_vram_total")
	for _, totalPath := range matches {
		totalData, err := os.ReadFile(totalPath)
		if err != nil {
			continue
		}
		totalVal, err := strconv.ParseInt(strings.TrimSpace(string(totalData)), 10, 64)
		if err != nil {
			continue
		}

		usedPath := strings.Replace(totalPath, "mem_info_vram_total", "mem_info_vram_used", 1)
		usedData, err := os.ReadFile(usedPath)
		if err != nil {
			return -1, totalVal
		}
		usedVal, err := strconv.ParseInt(strings.TrimSpace(string(usedData)), 10, 64)
		if err != nil {
			return -1, totalVal
		}

		return usedVal, totalVal
	}

	return -1, -1
}

// readGPUMemFreq reads GPU memory clock from pp_dpm_mclk (AMD) in MHz.
// The active frequency line is marked with *. If no line is marked,
// falls back to the highest level (last entry).
func readGPUMemFreq() float64 {
	matches, _ := filepath.Glob("/sys/class/drm/card*/device/pp_dpm_mclk")
	for _, path := range matches {
		data, err := os.ReadFile(path)
		if err != nil {
			continue
		}

		return parseDPMFreq(string(data))
	}

	return -1
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

// readFanSpeed reads the first available fan speed in RPM from hwmon.
func readFanSpeed() int {
	hwmonDir := "/sys/class/hwmon"
	entries, err := os.ReadDir(hwmonDir)
	if err != nil {
		return -1
	}

	for _, entry := range entries {
		fanPath := filepath.Join(hwmonDir, entry.Name(), "fan1_input")
		data, err := os.ReadFile(fanPath)
		if err != nil {
			continue
		}
		if val, err := strconv.Atoi(strings.TrimSpace(string(data))); err == nil {
			return val
		}
	}

	return -1
}
