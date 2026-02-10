//go:build windows

package telemetry

import (
	"math"
	"runtime"
	"syscall"
	"unsafe"
)

var (
	kernel32                 = syscall.NewLazyDLL("kernel32.dll")
	procGetSystemTimes       = kernel32.NewProc("GetSystemTimes")
	procGlobalMemoryStatusEx = kernel32.NewProc("GlobalMemoryStatusEx")
	procGetSystemPowerStatus = kernel32.NewProc("GetSystemPowerStatus")

	powrprof                   = syscall.NewLazyDLL("powrprof.dll")
	procCallNtPowerInformation = powrprof.NewProc("CallNtPowerInformation")
)

// ── CPU ─────────────────────────────────────────────────────────────────────

// fileTime mirrors the Windows FILETIME structure.
type fileTime struct {
	LowDateTime  uint32
	HighDateTime uint32
}

func fileTimeToUint64(ft fileTime) uint64 {
	return uint64(ft.HighDateTime)<<32 | uint64(ft.LowDateTime)
}

// readCPUTimes uses GetSystemTimes to get aggregate idle and total CPU time.
func readCPUTimes() (idle, total uint64) {
	var idleTime, kernelTime, userTime fileTime

	ret, _, _ := procGetSystemTimes.Call(
		uintptr(unsafe.Pointer(&idleTime)),
		uintptr(unsafe.Pointer(&kernelTime)),
		uintptr(unsafe.Pointer(&userTime)),
	)
	if ret == 0 {
		return 0, 0
	}

	idleVal := fileTimeToUint64(idleTime)
	kernelVal := fileTimeToUint64(kernelTime) // kernel time includes idle
	userVal := fileTimeToUint64(userTime)

	return idleVal, kernelVal + userVal
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

// readCPUTemp is not available on Windows without a kernel driver (Ring 0 MSR access).
func readCPUTemp() float64 {
	return -1
}

// processorPowerInformation mirrors PROCESSOR_POWER_INFORMATION from powrprof.
type processorPowerInformation struct {
	Number           uint32
	MaxMhz           uint32
	CurrentMhz       uint32
	MhzLimit         uint32
	MaxIdleState     uint32
	CurrentIdleState uint32
}

// readCPUFreq uses CallNtPowerInformation(ProcessorInformation) to get current CPU frequency.
func readCPUFreq() float64 {
	const processorInformation = 11

	numCPU := runtime.NumCPU()
	buf := make([]processorPowerInformation, numCPU)
	bufSize := uintptr(numCPU) * unsafe.Sizeof(processorPowerInformation{})

	ret, _, _ := procCallNtPowerInformation.Call(
		uintptr(processorInformation),
		0, 0,
		uintptr(unsafe.Pointer(&buf[0])),
		bufSize,
	)
	if ret != 0 {
		return -1
	}

	// Average frequency across all cores.
	var totalMhz uint64
	for i := 0; i < numCPU; i++ {
		totalMhz += uint64(buf[i].CurrentMhz)
	}
	return float64(totalMhz) / float64(numCPU)
}

// ── GPU ─────────────────────────────────────────────────────────────────────
// GPU metrics on Windows require vendor-specific SDKs:
//   - AMD:    ADLX (amdadlx64.dll) — COM-based C++ API
//   - NVIDIA: NVML (nvml.dll) — C API
//   - Intel:  IGCL
// The D3DKMT ADAPTERPERFDATA API (used by Task Manager) does not work on
// WDDM 4.x (Windows 11 25H2+) with modern drivers (e.g. AMD RDNA 4).
// Vendor SDK integration may be added in a future phase.

func readGPUUsage() float64 { return -1 }
func readGPUTemp() float64  { return -1 }
func readGPUFreq() float64  { return -1 }

// ── Memory ──────────────────────────────────────────────────────────────────

// memoryStatusEx mirrors the Windows MEMORYSTATUSEX structure.
type memoryStatusEx struct {
	Length               uint32
	MemoryLoad           uint32
	TotalPhys            uint64
	AvailPhys            uint64
	TotalPageFile        uint64
	AvailPageFile        uint64
	TotalVirtual         uint64
	AvailVirtual         uint64
	AvailExtendedVirtual uint64
}

// readMemInfo uses GlobalMemoryStatusEx to get physical memory info.
func readMemInfo() (total, available int64) {
	var ms memoryStatusEx
	ms.Length = uint32(unsafe.Sizeof(ms))

	ret, _, _ := procGlobalMemoryStatusEx.Call(uintptr(unsafe.Pointer(&ms)))
	if ret == 0 {
		return -1, -1
	}

	return int64(ms.TotalPhys), int64(ms.AvailPhys)
}

// ── Battery ─────────────────────────────────────────────────────────────────

// systemPowerStatus mirrors the Windows SYSTEM_POWER_STATUS structure.
type systemPowerStatus struct {
	ACLineStatus        byte
	BatteryFlag         byte
	BatteryLifePercent  byte
	SystemStatusFlag    byte
	BatteryLifeTime     uint32
	BatteryFullLifeTime uint32
}

// readBattery uses GetSystemPowerStatus to get battery info.
func readBattery() (capacity int, status string) {
	var ps systemPowerStatus

	ret, _, _ := procGetSystemPowerStatus.Call(uintptr(unsafe.Pointer(&ps)))
	if ret == 0 {
		return -1, "Unknown"
	}

	// 255 means no battery / unknown
	if ps.BatteryLifePercent == 255 {
		return -1, ""
	}

	cap := int(ps.BatteryLifePercent)

	switch {
	case ps.BatteryFlag&8 != 0:
		return cap, "Charging"
	case ps.ACLineStatus == 1:
		return cap, "AC Connected"
	case ps.ACLineStatus == 0:
		return cap, "Discharging"
	default:
		return cap, "Unknown"
	}
}

// ── Power / Fan ─────────────────────────────────────────────────────────────

func readPowerInfo() (tdpWatts, powerWatts float64) { return -1, -1 }
func readFanSpeed() int                              { return -1 }
