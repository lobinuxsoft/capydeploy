//go:build linux

package steam

import (
	"os/exec"
	"strings"
)

// Restart performs a soft restart of Steam.
// On Linux/SteamOS Gaming Mode, Steam will automatically relaunch.
func (c *Controller) Restart() *RestartResult {
	// Use steam -shutdown which gracefully closes Steam
	// On Bazzite/SteamOS Gaming Mode, the session manager will restart Steam automatically
	cmd := exec.Command("steam", "-shutdown")
	err := cmd.Run()

	if err != nil {
		// Try alternative method
		exec.Command("sh", "-c", "steam -shutdown >/dev/null 2>&1 || true").Run()
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
