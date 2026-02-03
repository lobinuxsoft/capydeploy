//go:build linux

package steam

import (
	"os"
	"os/exec"
	"strings"
	"time"
)

// Restart performs a soft restart of Steam.
// On Linux/SteamOS Gaming Mode, Steam will automatically relaunch.
// On Desktop mode, we need to manually restart it.
func (c *Controller) Restart() *RestartResult {
	// Check if we're in Gaming Mode (session type gamescope)
	isGamingMode := os.Getenv("XDG_CURRENT_DESKTOP") == "gamescope"

	// Use steam -shutdown which gracefully closes Steam
	cmd := exec.Command("steam", "-shutdown")
	err := cmd.Run()

	if err != nil {
		// Try alternative method
		exec.Command("sh", "-c", "steam -shutdown >/dev/null 2>&1 || true").Run()
	}

	// In Desktop mode, manually restart Steam after shutdown
	if !isGamingMode {
		go func() {
			// Wait for Steam to fully close
			time.Sleep(3 * time.Second)
			exec.Command("steam").Start()
		}()
	}

	return &RestartResult{
		Success: true,
		Message: "Steam restart initiated",
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
