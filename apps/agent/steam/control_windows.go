//go:build windows

package steam

import (
	"os/exec"
	"strings"
	"time"
)

// Restart performs a soft restart of Steam.
// On Windows, it uses taskkill to close Steam and then relaunches it.
func (c *Controller) Restart() *RestartResult {
	// First, gracefully close Steam
	exec.Command("taskkill", "/IM", "steam.exe").Run()

	// Wait a moment for Steam to close
	time.Sleep(2 * time.Second)

	// Relaunch Steam
	// Steam typically installs to Program Files (x86)
	steamPaths := []string{
		`C:\Program Files (x86)\Steam\steam.exe`,
		`C:\Program Files\Steam\steam.exe`,
	}

	for _, path := range steamPaths {
		cmd := exec.Command(path)
		if err := cmd.Start(); err == nil {
			return &RestartResult{
				Success: true,
				Message: "Steam restart initiated",
			}
		}
	}

	// Try using start command as fallback
	exec.Command("cmd", "/C", "start", "steam://").Run()

	return &RestartResult{
		Success: true,
		Message: "Steam restart initiated",
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
