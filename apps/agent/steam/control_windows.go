//go:build windows

package steam

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
	"time"
)

const (
	shutdownTimeout = 10 * time.Second
)

// getSteamExe finds the Steam executable path
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

// IsGamingMode returns false on Windows (no Gaming Mode)
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
		return exec.Command("cmd", "/C", "start", "steam://open/main").Run()
	}

	cmd := exec.Command(steamExe)
	return cmd.Start()
}

// IsCEFAvailable returns false on Windows (CEF not supported).
func (c *Controller) IsCEFAvailable() bool {
	return false
}

// WaitForCEF is a no-op on Windows (CEF not supported).
func (c *Controller) WaitForCEF() error {
	return nil
}

// EnsureRunning makes sure Steam is running.
// On Windows, CEF is not available so we just ensure Steam is running.
func (c *Controller) EnsureRunning() error {
	if c.IsRunning() {
		return nil
	}
	return c.Start()
}

// Shutdown gracefully closes Steam.
func (c *Controller) Shutdown() error {
	if !c.IsRunning() {
		return nil
	}

	steamExe := getSteamExe()
	if steamExe != "" {
		exec.Command(steamExe, "-shutdown").Run()
	} else {
		exec.Command("cmd", "/C", "start", "steam://exit").Run()
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
		exec.Command("taskkill", "/F", "/IM", "steam.exe").Run()
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

// IsRunning checks if Steam is currently running.
func (c *Controller) IsRunning() bool {
	cmd := exec.Command("tasklist", "/FI", "IMAGENAME eq steam.exe", "/NH")
	output, err := cmd.Output()
	if err != nil {
		return false
	}
	return strings.Contains(string(output), "steam.exe")
}
