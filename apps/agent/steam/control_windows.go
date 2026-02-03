//go:build windows

package steam

import (
	"fmt"
	"net/http"
	"os"
	"os/exec"
	"strings"
	"time"
)

const (
	cefEndpoint      = "http://localhost:8080/json"
	cefTimeout       = 30 * time.Second
	cefCheckInterval = 2 * time.Second
	shutdownTimeout  = 10 * time.Second
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
		// Try via protocol
		return exec.Command("cmd", "/C", "start", "steam://open/main").Run()
	}

	cmd := exec.Command(steamExe)
	return cmd.Start()
}

// IsCEFAvailable checks if Steam's CEF debugger is responding.
func (c *Controller) IsCEFAvailable() bool {
	client := &http.Client{Timeout: 2 * time.Second}
	resp, err := client.Get(cefEndpoint)
	if err != nil {
		return false
	}
	defer resp.Body.Close()
	return resp.StatusCode == http.StatusOK
}

// WaitForCEF waits until Steam's CEF debugger is available or timeout.
func (c *Controller) WaitForCEF() error {
	deadline := time.Now().Add(cefTimeout)

	for time.Now().Before(deadline) {
		if c.IsCEFAvailable() {
			return nil
		}
		time.Sleep(cefCheckInterval)
	}

	return fmt.Errorf("timeout waiting for Steam CEF (waited %v)", cefTimeout)
}

// EnsureRunning makes sure Steam is running and CEF is available.
func (c *Controller) EnsureRunning() error {
	if c.IsCEFAvailable() {
		return nil
	}

	if !c.IsRunning() {
		if err := c.Start(); err != nil {
			return err
		}
	}

	return c.WaitForCEF()
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

	if err := c.WaitForCEF(); err != nil {
		return &RestartResult{
			Success: false,
			Message: fmt.Sprintf("Steam started but CEF not available: %v", err),
		}
	}

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
