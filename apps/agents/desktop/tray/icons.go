package tray

import (
	"embed"
	"runtime"
)

//go:embed icons/*.ico icons/*.png
var iconsFS embed.FS

// Icon returns the appropriate icon bytes for the current platform.
// Windows uses .ico, Linux/macOS use .png.
func getIcon(name string) []byte {
	var ext string
	if runtime.GOOS == "windows" {
		ext = ".ico"
	} else {
		ext = ".png"
	}

	data, err := iconsFS.ReadFile("icons/" + name + ext)
	if err != nil {
		return nil
	}
	return data
}

// IconWaiting returns the blue circle icon (waiting for connection).
func IconWaiting() []byte {
	return getIcon("waiting")
}

// IconConnected returns the green circle icon (connected to Hub).
func IconConnected() []byte {
	return getIcon("connected")
}

// IconDisabled returns the red circle icon (connections disabled).
func IconDisabled() []byte {
	return getIcon("disabled")
}
