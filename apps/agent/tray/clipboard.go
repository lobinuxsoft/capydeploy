package tray

import (
	"os/exec"
	"runtime"
	"strings"
)

// copyToClipboard copies text to the system clipboard.
func copyToClipboard(text string) error {
	var cmd *exec.Cmd

	switch runtime.GOOS {
	case "windows":
		// Use PowerShell's Set-Clipboard
		cmd = exec.Command("powershell", "-Command", "Set-Clipboard", "-Value", text)
	case "darwin":
		cmd = exec.Command("pbcopy")
		cmd.Stdin = strings.NewReader(text)
	case "linux":
		// Try xclip first, then xsel
		if _, err := exec.LookPath("xclip"); err == nil {
			cmd = exec.Command("xclip", "-selection", "clipboard")
			cmd.Stdin = strings.NewReader(text)
		} else if _, err := exec.LookPath("xsel"); err == nil {
			cmd = exec.Command("xsel", "--clipboard", "--input")
			cmd.Stdin = strings.NewReader(text)
		} else if _, err := exec.LookPath("wl-copy"); err == nil {
			// Wayland support
			cmd = exec.Command("wl-copy")
			cmd.Stdin = strings.NewReader(text)
		} else {
			return nil // No clipboard tool available
		}
	default:
		return nil
	}

	return cmd.Run()
}
