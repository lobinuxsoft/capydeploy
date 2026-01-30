//go:build windows

package steam

import (
	"os"
	"os/exec"
	"strings"
	"time"
)

// Restart performs a soft restart of Steam.
// On Windows, it uses Steam's -shutdown flag and then relaunches.
func (c *Controller) Restart() *RestartResult {
	// Find Steam executable
	steamPaths := []string{
		`C:\Program Files (x86)\Steam\steam.exe`,
		`C:\Program Files\Steam\steam.exe`,
	}

	var steamExe string
	for _, path := range steamPaths {
		if _, err := os.Stat(path); err == nil {
			steamExe = path
			break
		}
	}

	if steamExe == "" {
		// Fallback: try to shutdown via protocol
		exec.Command("cmd", "/C", "start", "steam://exit").Run()
		time.Sleep(5 * time.Second)
		exec.Command("cmd", "/C", "start", "steam://open/main").Run()
		return &RestartResult{
			Success: true,
			Message: "Steam restart initiated via protocol",
		}
	}

	// Gracefully shutdown Steam
	exec.Command(steamExe, "-shutdown").Run()

	// Wait for Steam to fully close (up to 10 seconds)
	for i := 0; i < 20; i++ {
		time.Sleep(500 * time.Millisecond)
		if !c.IsRunning() {
			break
		}
	}

	// Extra delay to ensure Steam is fully closed
	time.Sleep(1 * time.Second)

	// Relaunch Steam
	cmd := exec.Command(steamExe)
	if err := cmd.Start(); err != nil {
		return &RestartResult{
			Success: false,
			Message: "Failed to relaunch Steam: " + err.Error(),
		}
	}

	return &RestartResult{
		Success: true,
		Message: "Steam restart completed",
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
