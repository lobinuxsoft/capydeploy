//go:build !linux && !windows

package telemetry

// readCPUTimes is a stub for unsupported platforms.
func readCPUTimes() (idle, total uint64) {
	return 0, 0
}

// calculateCPUUsage is a stub for unsupported platforms.
func calculateCPUUsage(prevIdle, prevTotal, currIdle, currTotal uint64) float64 {
	return -1
}

// readCPUTemp is a stub for unsupported platforms.
func readCPUTemp() float64 {
	return -1
}

// readGPUUsage is a stub for unsupported platforms.
func readGPUUsage() float64 {
	return -1
}

// readGPUTemp is a stub for unsupported platforms.
func readGPUTemp() float64 {
	return -1
}

// readMemInfo is a stub for unsupported platforms.
func readMemInfo() (total, available int64) {
	return -1, -1
}

// readBattery is a stub for unsupported platforms.
func readBattery() (capacity int, status string) {
	return -1, "Unknown"
}

// readCPUFreq is a stub for unsupported platforms.
func readCPUFreq() float64 {
	return -1
}

// readGPUFreq is a stub for unsupported platforms.
func readGPUFreq() float64 {
	return -1
}

// readPowerInfo is a stub for unsupported platforms.
func readPowerInfo() (tdpWatts, powerWatts float64) {
	return -1, -1
}

// readFanSpeed is a stub for unsupported platforms.
func readFanSpeed() int {
	return -1
}
