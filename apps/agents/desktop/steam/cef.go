// Package steam provides Steam control and CEF client operations for the Agent.
// The CEF client connects to Steam's embedded Chromium (CEF) debugger via
// Chrome DevTools Protocol to execute JavaScript commands like SetCustomArtworkForApp.
package steam

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"path/filepath"
	"time"

	"github.com/gorilla/websocket"
	"github.com/lobinuxsoft/capydeploy/pkg/steam"
)

// CEF asset type constants matching SteamClient.Apps.SetCustomArtworkForApp.
const (
	CEFAssetGridPortrait  = 0 // 600x900
	CEFAssetHero          = 1 // 1920x620
	CEFAssetLogo          = 2
	CEFAssetGridLandscape = 3 // 920x430
	CEFAssetIcon          = 4
)

const (
	cefDebugEndpoint = "http://localhost:8080/json"
	cefHTTPTimeout   = 5 * time.Second
	cefWSHandshake   = 5 * time.Second
	cefWSRead        = 10 * time.Second
	cefDebugFile     = ".cef-enable-remote-debugging"
)

// CEFClient communicates with Steam's CEF debugger via Chrome DevTools Protocol.
type CEFClient struct {
	endpoint string
}

type cefTab struct {
	Title               string `json:"title"`
	Type                string `json:"type"`
	ID                  string `json:"id"`
	URL                 string `json:"url"`
	WebSocketDebuggerURL string `json:"webSocketDebuggerUrl"`
}

type cefMessage struct {
	ID     int                    `json:"id"`
	Method string                 `json:"method"`
	Params map[string]interface{} `json:"params"`
}

type cefResponse struct {
	ID     int           `json:"id"`
	Result cefEvalResult `json:"result"`
}

type cefEvalResult struct {
	Result           cefResultValue   `json:"result"`
	ExceptionDetails *json.RawMessage `json:"exceptionDetails,omitempty"`
}

type cefResultValue struct {
	Type  string          `json:"type"`
	Value json.RawMessage `json:"value"`
}

// NewCEFClient creates a new CEF client targeting the local Steam debugger.
func NewCEFClient() *CEFClient {
	return &CEFClient{endpoint: cefDebugEndpoint}
}

// getTabs fetches the list of debuggable tabs from CEF.
func (c *CEFClient) getTabs(ctx context.Context) ([]cefTab, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, c.endpoint, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	client := &http.Client{Timeout: cefHTTPTimeout}
	resp, err := client.Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to CEF debugger: %w", err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read CEF response: %w", err)
	}

	var tabs []cefTab
	if err := json.Unmarshal(body, &tabs); err != nil {
		return nil, fmt.Errorf("failed to parse CEF tabs: %w", err)
	}

	return tabs, nil
}

// findJSContext finds the best tab for JS evaluation.
// Prefers "SharedJSContext" over "SP" (Steam's main UI).
func (c *CEFClient) findJSContext(tabs []cefTab) (*cefTab, error) {
	var spTab *cefTab

	for i := range tabs {
		tab := &tabs[i]
		if tab.WebSocketDebuggerURL == "" {
			continue
		}
		if tab.Title == "SharedJSContext" {
			return tab, nil
		}
		if tab.Title == "SP" && spTab == nil {
			spTab = tab
		}
	}

	if spTab != nil {
		return spTab, nil
	}

	return nil, fmt.Errorf("no suitable JS context found (need SharedJSContext or SP tab)")
}

// evaluateAsync connects to a tab's WebSocket and evaluates a JS expression
// that returns a Promise, waiting for it to resolve via CDP's awaitPromise.
func (c *CEFClient) evaluateAsync(ctx context.Context, wsURL string, jsExpr string) (json.RawMessage, error) {
	dialer := websocket.Dialer{
		HandshakeTimeout: cefWSHandshake,
	}

	conn, _, err := dialer.DialContext(ctx, wsURL, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to CEF WebSocket: %w", err)
	}
	defer conn.Close()

	msg := cefMessage{
		ID:     1,
		Method: "Runtime.evaluate",
		Params: map[string]interface{}{
			"expression":    jsExpr,
			"returnByValue": true,
			"awaitPromise":  true,
		},
	}

	if err := conn.WriteJSON(msg); err != nil {
		return nil, fmt.Errorf("failed to send CEF message: %w", err)
	}

	conn.SetReadDeadline(time.Now().Add(cefWSRead))
	for {
		_, rawMsg, err := conn.ReadMessage()
		if err != nil {
			return nil, fmt.Errorf("failed to read CEF response: %w", err)
		}

		var resp cefResponse
		if err := json.Unmarshal(rawMsg, &resp); err != nil {
			continue
		}

		if resp.ID != 1 {
			continue
		}

		if resp.Result.ExceptionDetails != nil {
			return nil, fmt.Errorf("JS exception: %s", string(*resp.Result.ExceptionDetails))
		}

		return resp.Result.Result.Value, nil
	}
}

// jsString produces a safely escaped JS string literal using JSON encoding.
// Handles paths with backslashes (Windows), quotes, spaces, and special chars.
func jsString(s string) string {
	b, _ := json.Marshal(s)
	return string(b)
}

// SetCustomArtwork applies custom artwork to a Steam app via CEF API.
func (c *CEFClient) SetCustomArtwork(ctx context.Context, appID uint32, base64Data string, assetType int) error {
	tabs, err := c.getTabs(ctx)
	if err != nil {
		return err
	}

	tab, err := c.findJSContext(tabs)
	if err != nil {
		return err
	}

	js := fmt.Sprintf(
		`SteamClient.Apps.SetCustomArtworkForApp(%d, "%s", "png", %d)`,
		appID, base64Data, assetType,
	)

	// SetCustomArtworkForApp returns a Promise — must await it to ensure
	// the operation completes before applying the next artwork type.
	_, err = c.evaluateAsync(ctx, tab.WebSocketDebuggerURL, js)
	return err
}

// ClearCustomArtwork removes custom artwork from a Steam app via CEF API.
func (c *CEFClient) ClearCustomArtwork(ctx context.Context, appID uint32, assetType int) error {
	tabs, err := c.getTabs(ctx)
	if err != nil {
		return err
	}

	tab, err := c.findJSContext(tabs)
	if err != nil {
		return err
	}

	js := fmt.Sprintf(
		`SteamClient.Apps.ClearCustomArtworkForApp(%d, %d)`,
		appID, assetType,
	)

	// ClearCustomArtworkForApp returns a Promise — must await it.
	_, err = c.evaluateAsync(ctx, tab.WebSocketDebuggerURL, js)
	return err
}

// AddShortcut creates a Steam shortcut via CEF API and returns the new AppID.
// Uses SteamClient.Apps.AddShortcut(name, exe, startDir, launchOptions) which
// returns a Promise<number>.
func (c *CEFClient) AddShortcut(ctx context.Context, name, exe, startDir, launchOptions string) (uint32, error) {
	tabs, err := c.getTabs(ctx)
	if err != nil {
		return 0, err
	}

	tab, err := c.findJSContext(tabs)
	if err != nil {
		return 0, err
	}

	js := fmt.Sprintf(
		`SteamClient.Apps.AddShortcut(%s, %s, %s, %s)`,
		jsString(name), jsString(exe), jsString(startDir), jsString(launchOptions),
	)

	raw, err := c.evaluateAsync(ctx, tab.WebSocketDebuggerURL, js)
	if err != nil {
		return 0, fmt.Errorf("AddShortcut failed: %w", err)
	}

	var appID float64
	if err := json.Unmarshal(raw, &appID); err != nil {
		return 0, fmt.Errorf("failed to parse AddShortcut result: %w (raw: %s)", err, string(raw))
	}

	if appID <= 0 {
		return 0, fmt.Errorf("AddShortcut returned invalid appID: %v", appID)
	}

	return uint32(appID), nil
}

// RemoveShortcut removes a Steam shortcut via CEF API.
func (c *CEFClient) RemoveShortcut(ctx context.Context, appID uint32) error {
	tabs, err := c.getTabs(ctx)
	if err != nil {
		return err
	}

	tab, err := c.findJSContext(tabs)
	if err != nil {
		return err
	}

	js := fmt.Sprintf(`SteamClient.Apps.RemoveShortcut(%d)`, appID)

	_, err = c.evaluateAsync(ctx, tab.WebSocketDebuggerURL, js)
	return err
}

// SetShortcutName renames a shortcut via CEF API.
// AddShortcut ignores the name parameter and uses the executable filename,
// so this must be called after creation to set the correct name.
func (c *CEFClient) SetShortcutName(ctx context.Context, appID uint32, name string) error {
	tabs, err := c.getTabs(ctx)
	if err != nil {
		return err
	}

	tab, err := c.findJSContext(tabs)
	if err != nil {
		return err
	}

	js := fmt.Sprintf(`SteamClient.Apps.SetShortcutName(%d, %s)`, appID, jsString(name))

	_, err = c.evaluateAsync(ctx, tab.WebSocketDebuggerURL, js)
	return err
}

// SetShortcutLaunchOptions sets launch options for a shortcut via CEF API.
func (c *CEFClient) SetShortcutLaunchOptions(ctx context.Context, appID uint32, options string) error {
	tabs, err := c.getTabs(ctx)
	if err != nil {
		return err
	}

	tab, err := c.findJSContext(tabs)
	if err != nil {
		return err
	}

	js := fmt.Sprintf(`SteamClient.Apps.SetShortcutLaunchOptions(%d, %s)`, appID, jsString(options))

	_, err = c.evaluateAsync(ctx, tab.WebSocketDebuggerURL, js)
	return err
}

// SpecifyCompatTool sets the compatibility tool (e.g. Proton) for a shortcut via CEF API.
// Uses SteamClient.Apps.SpecifyCompatTool(appID, toolName).
func (c *CEFClient) SpecifyCompatTool(ctx context.Context, appID uint32, toolName string) error {
	tabs, err := c.getTabs(ctx)
	if err != nil {
		return err
	}

	tab, err := c.findJSContext(tabs)
	if err != nil {
		return err
	}

	js := fmt.Sprintf(`SteamClient.Apps.SpecifyCompatTool(%d, %s)`, appID, jsString(toolName))

	_, err = c.evaluateAsync(ctx, tab.WebSocketDebuggerURL, js)
	return err
}

// ArtworkTypeToCEFAsset maps artwork type strings to CEF asset type constants.
func ArtworkTypeToCEFAsset(artworkType string) (int, bool) {
	switch artworkType {
	case "grid":
		return CEFAssetGridPortrait, true
	case "banner":
		return CEFAssetGridLandscape, true
	case "hero":
		return CEFAssetHero, true
	case "logo":
		return CEFAssetLogo, true
	case "icon":
		return CEFAssetIcon, true
	default:
		return 0, false
	}
}

// EnsureCEFDebugFile creates the .cef-enable-remote-debugging file in Steam's
// base directory if it doesn't exist. This file tells Steam to enable the CEF
// remote debugger on port 8080. Returns true if the file was just created
// (meaning Steam needs a restart for CEF to become available).
func EnsureCEFDebugFile() (created bool, err error) {
	paths, err := steam.NewPaths()
	if err != nil {
		return false, fmt.Errorf("failed to get Steam paths: %w", err)
	}
	baseDir := paths.BaseDir()

	debugPath := filepath.Join(baseDir, cefDebugFile)

	if _, err := os.Stat(debugPath); err == nil {
		return false, nil // already exists
	}

	if err := os.WriteFile(debugPath, []byte{}, 0644); err != nil {
		return false, fmt.Errorf("failed to create CEF debug file: %w", err)
	}

	return true, nil
}

// EnsureCEFReady guarantees that the CEF debugger is available.
// CEF only works when Steam is running with the debug file present at startup.
// Flow: check CEF → ensure debug file → restart/start Steam as needed.
func EnsureCEFReady() error {
	ctrl := NewController()

	// Check if CEF is already available — fast path
	if ctrl.IsCEFAvailable() {
		return nil
	}

	// Ensure the debug file exists so CEF activates on next Steam start
	if _, err := EnsureCEFDebugFile(); err != nil {
		return fmt.Errorf("failed to ensure CEF debug file: %w", err)
	}

	// CEF is not responding. Steam only reads the debug file at startup,
	// so if Steam is already running without CEF, it must be restarted.
	if ctrl.IsRunning() {
		log.Printf("[cef] Steam running without CEF, restarting to enable debugger")
		result := ctrl.Restart()
		if !result.Success {
			return fmt.Errorf("failed to restart Steam for CEF: %s", result.Message)
		}
		return nil // Restart already waits for CEF
	}

	// Steam not running — start it and wait for CEF
	log.Printf("[cef] starting Steam with CEF debugger enabled")
	if err := ctrl.Start(); err != nil {
		return fmt.Errorf("failed to start Steam: %w", err)
	}
	return ctrl.WaitForCEF()
}
