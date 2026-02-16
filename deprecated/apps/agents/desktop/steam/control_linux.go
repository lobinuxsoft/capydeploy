//go:build linux

package steam

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
	"time"
)

// IsGamingMode returns true if running in SteamOS/Bazzite Gaming Mode
func (c *Controller) IsGamingMode() bool {
	return os.Getenv("XDG_CURRENT_DESKTOP") == "gamescope"
}

// Start launches Steam if it's not already running.
func (c *Controller) Start() error {
	if c.IsRunning() {
		return nil
	}

	cmd := exec.Command("steam")
	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to start Steam: %w", err)
	}

	return nil
}

// Shutdown gracefully closes Steam and waits for it to exit.
func (c *Controller) Shutdown() error {
	if !c.IsRunning() {
		return nil
	}

	// Use steam -shutdown which gracefully closes Steam
	cmd := exec.Command("steam", "-shutdown")
	if err := cmd.Run(); err != nil {
		// Try alternative method
		exec.Command("sh", "-c", "steam -shutdown >/dev/null 2>&1 || true").Run()
	}

	// Wait for Steam to fully close
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
// Shuts down Steam, waits for it to close, starts it again, and waits for CEF.
func (c *Controller) Restart() *RestartResult {
	// Shutdown Steam
	if err := c.Shutdown(); err != nil {
		// Force kill if graceful shutdown failed
		exec.Command("pkill", "-9", "steam").Run()
		time.Sleep(2 * time.Second)
	}

	// In Gaming Mode, session manager restarts Steam automatically
	if c.IsGamingMode() {
		// Just wait for CEF to come back
		if err := c.WaitForCEF(); err != nil {
			return &RestartResult{
				Success: false,
				Message: fmt.Sprintf("Steam restart failed: %v", err),
			}
		}
		return &RestartResult{
			Success: true,
			Message: "Steam restarted (Gaming Mode)",
		}
	}

	// Desktop mode: manually restart Steam
	if err := c.Start(); err != nil {
		return &RestartResult{
			Success: false,
			Message: fmt.Sprintf("Failed to start Steam: %v", err),
		}
	}

	// Wait for CEF to be available
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
	cmd := exec.Command("pgrep", "-x", "steam")
	output, err := cmd.Output()
	if err != nil {
		return false
	}
	return strings.TrimSpace(string(output)) != ""
}
