//go:build windows

package steam

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
	"syscall"
	"time"
	"unsafe"
)

var (
	kernel32                     = syscall.NewLazyDLL("kernel32.dll")
	procCreateToolhelp32Snapshot = kernel32.NewProc("CreateToolhelp32Snapshot")
	procProcess32FirstW          = kernel32.NewProc("Process32FirstW")
	procProcess32NextW           = kernel32.NewProc("Process32NextW")
)

const th32csSnapProcess = 0x00000002

// processEntry32W mirrors the Windows PROCESSENTRY32W structure.
type processEntry32W struct {
	Size              uint32
	Usage             uint32
	ProcessID         uint32
	DefaultHeapID     uintptr
	ModuleID          uint32
	Threads           uint32
	ParentProcessID   uint32
	PriClassBase      int32
	Flags             uint32
	ExeFile           [260]uint16
}

// processRunning checks if a process with the given name is running
// using the Windows Toolhelp API (no console window).
func processRunning(name string) bool {
	handle, _, _ := procCreateToolhelp32Snapshot.Call(th32csSnapProcess, 0)
	if handle == uintptr(syscall.InvalidHandle) {
		return false
	}
	defer syscall.CloseHandle(syscall.Handle(handle))

	var entry processEntry32W
	entry.Size = uint32(unsafe.Sizeof(entry))

	ret, _, _ := procProcess32FirstW.Call(handle, uintptr(unsafe.Pointer(&entry)))
	if ret == 0 {
		return false
	}

	for {
		exeName := syscall.UTF16ToString(entry.ExeFile[:])
		if strings.EqualFold(exeName, name) {
			return true
		}
		entry.Size = uint32(unsafe.Sizeof(entry))
		ret, _, _ = procProcess32NextW.Call(handle, uintptr(unsafe.Pointer(&entry)))
		if ret == 0 {
			break
		}
	}
	return false
}

// hiddenCmd returns an exec.Cmd that won't flash a console window.
func hiddenCmd(name string, args ...string) *exec.Cmd {
	cmd := exec.Command(name, args...)
	cmd.SysProcAttr = &syscall.SysProcAttr{HideWindow: true}
	return cmd
}

// getSteamExe finds the Steam executable path.
func getSteamExe() string {
	steamPaths := []string{
		`C:\Program Files (x86)\Steam\steam.exe`,
		`C:\Program Files\Steam\steam.exe`,
	}

	for _, path := range steamPaths {
		if _, err := os.Stat(path); err == nil {
			return path
		}
	}
	return ""
}

// IsGamingMode returns false on Windows (no Gaming Mode).
func (c *Controller) IsGamingMode() bool {
	return false
}

// Start launches Steam if it's not already running.
func (c *Controller) Start() error {
	if c.IsRunning() {
		return nil
	}

	steamExe := getSteamExe()
	if steamExe == "" {
		return hiddenCmd("cmd", "/C", "start", "steam://open/main").Run()
	}

	cmd := exec.Command(steamExe)
	return cmd.Start()
}

// Shutdown gracefully closes Steam.
func (c *Controller) Shutdown() error {
	if !c.IsRunning() {
		return nil
	}

	steamExe := getSteamExe()
	if steamExe != "" {
		hiddenCmd(steamExe, "-shutdown").Run()
	} else {
		hiddenCmd("cmd", "/C", "start", "steam://exit").Run()
	}

	deadline := time.Now().Add(shutdownTimeout)
	for time.Now().Before(deadline) {
		if !c.IsRunning() {
			return nil
		}
		time.Sleep(500 * time.Millisecond)
	}

	return fmt.Errorf("timeout waiting for Steam to close")
}

// Restart performs a full restart of Steam.
func (c *Controller) Restart() *RestartResult {
	if err := c.Shutdown(); err != nil {
		hiddenCmd("taskkill", "/F", "/IM", "steam.exe").Run()
		time.Sleep(2 * time.Second)
	}

	if err := c.Start(); err != nil {
		return &RestartResult{
			Success: false,
			Message: fmt.Sprintf("Failed to start Steam: %v", err),
		}
	}

	// Give Steam a moment to initialize
	time.Sleep(3 * time.Second)

	return &RestartResult{
		Success: true,
		Message: "Steam restarted successfully",
	}
}

// IsRunning checks if Steam is currently running using the Toolhelp API.
func (c *Controller) IsRunning() bool {
	return processRunning("steam.exe")
}
