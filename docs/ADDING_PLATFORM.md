# Adding a New Platform to CapyDeploy

This guide explains how to add support for a new platform (e.g., macOS, SteamOS) to CapyDeploy.

## Overview

CapyDeploy uses **build tags** to separate platform-specific code. Each platform needs implementations for:

1. **Steam paths** - Where Steam stores its files
2. **Steam controller** - How to start/stop/restart Steam
3. **Artwork handler** - Platform-specific artwork paths (if different)

## Files to Create/Modify

### 1. Steam Paths (`pkg/steam/paths_<platform>.go`)

Create a new file with the build tag for your platform:

```go
//go:build darwin

package steam

import (
    "os"
    "path/filepath"
)

// getBaseDir returns the Steam base directory on macOS.
func getBaseDir() (string, error) {
    home, err := os.UserHomeDir()
    if err != nil {
        return "", err
    }

    steamPath := filepath.Join(home, "Library", "Application Support", "Steam")
    if _, err := os.Stat(steamPath); err != nil {
        return "", ErrSteamNotFound
    }

    return steamPath, nil
}
```

**Key function**: `getBaseDir()` must return the root Steam installation directory.

### 2. Steam Controller (`apps/agent/steam/control_<platform>.go`)

Create the Steam process controller:

```go
//go:build darwin

package steam

import (
    "fmt"
    "os/exec"
    "strings"
    "time"
)

const shutdownTimeout = 10 * time.Second

// IsGamingMode returns true if running in a gaming/console mode.
func (c *Controller) IsGamingMode() bool {
    return false // macOS doesn't have gaming mode
}

// Start launches Steam if not running.
func (c *Controller) Start() error {
    if c.IsRunning() {
        return nil
    }
    return exec.Command("open", "-a", "Steam").Run()
}

// Shutdown gracefully closes Steam.
func (c *Controller) Shutdown() error {
    if !c.IsRunning() {
        return nil
    }

    // Try graceful shutdown
    exec.Command("osascript", "-e", `quit app "Steam"`).Run()

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
        // Force kill if graceful shutdown fails
        exec.Command("pkill", "-9", "steam").Run()
        time.Sleep(2 * time.Second)
    }

    if err := c.Start(); err != nil {
        return &RestartResult{
            Success: false,
            Message: fmt.Sprintf("Failed to start Steam: %v", err),
        }
    }

    time.Sleep(3 * time.Second)

    return &RestartResult{
        Success: true,
        Message: "Steam restarted successfully",
    }
}

// IsRunning checks if Steam is currently running.
func (c *Controller) IsRunning() bool {
    cmd := exec.Command("pgrep", "-x", "steam")
    output, _ := cmd.Output()
    return len(strings.TrimSpace(string(output))) > 0
}

// IsCEFAvailable returns whether CEF debugging is available.
func (c *Controller) IsCEFAvailable() bool {
    return false
}

// WaitForCEF waits for CEF to be available.
func (c *Controller) WaitForCEF() error {
    return nil
}

// EnsureRunning ensures Steam is running and ready.
func (c *Controller) EnsureRunning() error {
    if c.IsRunning() {
        return nil
    }
    return c.Start()
}
```

**Required methods**:
- `IsGamingMode()` - Console/gaming mode detection
- `Start()` - Launch Steam
- `Shutdown()` - Graceful shutdown
- `Restart()` - Full restart with result
- `IsRunning()` - Process detection
- `IsCEFAvailable()` / `WaitForCEF()` - CEF support (optional)
- `EnsureRunning()` - Start if not running

### 3. Artwork Handler (`apps/agent/artwork/artwork_<platform>.go`)

Only needed if artwork paths differ from the defaults:

```go
//go:build darwin

package artwork

// Platform-specific artwork handling if needed.
// Most platforms can use the default implementation in artwork.go
```

Usually the base implementation works, as Steam's grid folder structure is consistent.

## Testing Your Implementation

### 1. Build for the new platform

```bash
GOOS=darwin GOARCH=amd64 go build ./...
```

### 2. Verify Steam detection

```go
paths, err := steam.NewPaths()
if err != nil {
    log.Fatal("Steam not found:", err)
}
fmt.Println("Steam path:", paths.Base())
```

### 3. Test Steam control

```go
controller := steam.NewController()
fmt.Println("Running:", controller.IsRunning())
result := controller.Restart()
fmt.Println("Restart:", result.Message)
```

### 4. Test shortcut creation

```go
mgr, _ := shortcuts.NewManager()
users, _ := steam.GetUsers()
appID, err := mgr.Create(users[0].ID, protocol.ShortcutConfig{
    Name:     "Test Game",
    Exe:      "/path/to/game",
    StartDir: "/path/to",
})
```

## Hub Module (Optional)

If the new platform has specific client-side requirements, create a module in `apps/hub/modules/`:

```go
package modules

type DarwinModule struct{}

func (m *DarwinModule) Platform() string {
    return "darwin"
}

func (m *DarwinModule) NewClient(host string, port int) PlatformClient {
    return newBaseClient(host, port, "darwin")
}

func (m *DarwinModule) SupportedImageFormats() []string {
    return []string{"image/png", "image/jpeg", "image/webp", "image/gif"}
}
```

Register it in `apps/hub/modules/registry.go`.

## Checklist

- [ ] `pkg/steam/paths_<platform>.go` - Steam directory detection
- [ ] `apps/agent/steam/control_<platform>.go` - Steam process control
- [ ] `apps/agent/artwork/artwork_<platform>.go` - Artwork handling (if needed)
- [ ] Test Steam detection works
- [ ] Test Steam start/stop/restart works
- [ ] Test shortcut creation works
- [ ] Test artwork application works
- [ ] Add platform to `README.md` requirements table
- [ ] Add platform dependencies to `docs/index.html`

## Platform-Specific Notes

### Linux
- Steam can run in Gaming Mode (SteamOS/Bazzite) or Desktop Mode
- CEF debugging available on port 8080 in Desktop Mode
- Uses `systemctl` or direct process control

### Windows
- Uses Windows Registry for Steam path detection
- `tasklist` / `taskkill` for process management
- WebView2 required for Agent UI

### macOS (example)
- Steam in `~/Library/Application Support/Steam`
- Use `open -a Steam` to launch
- AppleScript for graceful shutdown

## Questions?

Open an issue on GitHub or check the existing platform implementations for reference.
