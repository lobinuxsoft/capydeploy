// Package consolelog streams console logs from Steam's CEF debugger via CDP.
package consolelog

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"sync"
	"time"

	"github.com/gorilla/websocket"
	"github.com/lobinuxsoft/capydeploy/pkg/protocol"
)

const (
	cefDebugEndpoint = "http://localhost:8080/json"
	cefHTTPTimeout   = 5 * time.Second
	cefWSHandshake   = 5 * time.Second
)

// cdpConn is a persistent WebSocket connection to a CDP endpoint.
type cdpConn struct {
	conn     *websocket.Conn
	msgID    int
	closeOnce sync.Once
}

type cefTab struct {
	Title                string `json:"title"`
	Type                 string `json:"type"`
	WebSocketDebuggerURL string `json:"webSocketDebuggerUrl"`
}

// cdpEvent represents a raw CDP event or response.
type cdpEvent struct {
	ID     int             `json:"id,omitempty"`
	Method string          `json:"method,omitempty"`
	Params json.RawMessage `json:"params,omitempty"`
}

// consoleAPICalledParams matches Runtime.consoleAPICalled event.
type consoleAPICalledParams struct {
	Type string          `json:"type"`
	Args json.RawMessage `json:"args"`
}

// consoleArg represents a single argument in Runtime.consoleAPICalled.
type consoleArg struct {
	Type  string `json:"type"`
	Value any    `json:"value,omitempty"`
	// For string values, description may also be present
	Description string `json:"description,omitempty"`
}

// logEntryAddedParams matches Log.entryAdded event.
type logEntryAddedParams struct {
	Entry logEntry `json:"entry"`
}

type logEntry struct {
	Source string `json:"source"`
	Level  string `json:"level"`
	Text   string `json:"text"`
	URL    string `json:"url,omitempty"`
	Line   int    `json:"lineNumber,omitempty"`
}

// getTabs fetches the list of debuggable tabs from CEF.
func getTabs(ctx context.Context) ([]cefTab, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, cefDebugEndpoint, nil)
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

// findJSContext finds the SharedJSContext or SP tab for console log subscription.
func findJSContext(tabs []cefTab) (*cefTab, error) {
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

// dialCDP connects to a CDP WebSocket endpoint.
func dialCDP(ctx context.Context) (*cdpConn, error) {
	tabs, err := getTabs(ctx)
	if err != nil {
		return nil, err
	}

	tab, err := findJSContext(tabs)
	if err != nil {
		return nil, err
	}

	dialer := websocket.Dialer{
		HandshakeTimeout: cefWSHandshake,
	}

	conn, _, err := dialer.DialContext(ctx, tab.WebSocketDebuggerURL, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to CDP: %w", err)
	}

	return &cdpConn{conn: conn}, nil
}

// sendCommand sends a CDP command and returns its ID.
func (c *cdpConn) sendCommand(method string, params map[string]any) (int, error) {
	c.msgID++
	msg := map[string]any{
		"id":     c.msgID,
		"method": method,
	}
	if params != nil {
		msg["params"] = params
	}

	if err := c.conn.WriteJSON(msg); err != nil {
		return 0, fmt.Errorf("failed to send CDP command %s: %w", method, err)
	}
	return c.msgID, nil
}

// enableRuntime sends Runtime.enable to receive consoleAPICalled events.
func (c *cdpConn) enableRuntime() error {
	_, err := c.sendCommand("Runtime.enable", nil)
	return err
}

// enableLog sends Log.enable to receive entryAdded events.
func (c *cdpConn) enableLog() error {
	_, err := c.sendCommand("Log.enable", nil)
	return err
}

// readEvent reads the next CDP event from the WebSocket.
// Returns the raw event for the caller to dispatch.
func (c *cdpConn) readEvent() (*cdpEvent, error) {
	var event cdpEvent
	if err := c.conn.ReadJSON(&event); err != nil {
		return nil, err
	}
	return &event, nil
}

// close closes the CDP WebSocket connection. Safe to call multiple times.
func (c *cdpConn) close() {
	c.closeOnce.Do(func() {
		c.conn.Close()
	})
}

// consoleArgsResult holds parsed console args with optional styled segments.
type consoleArgsResult struct {
	Text     string
	Segments []protocol.StyledSegment
}

// formatConsoleArgsRich converts Runtime.consoleAPICalled args to text + styled segments.
// Detects %c format directives and splits into segments with CSS.
func formatConsoleArgsRich(raw json.RawMessage) consoleArgsResult {
	var args []consoleArg
	if err := json.Unmarshal(raw, &args); err != nil {
		return consoleArgsResult{Text: string(raw)}
	}

	if len(args) == 0 {
		return consoleArgsResult{}
	}

	// Check if first arg is a string containing %c
	firstStr := ""
	if args[0].Type == "string" {
		if s, ok := args[0].Value.(string); ok {
			firstStr = s
		}
	}

	if firstStr == "" || !strings.Contains(firstStr, "%c") {
		// No %c formatting — produce plain text
		text := formatConsoleArgsPlain(args)
		return consoleArgsResult{Text: text}
	}

	// Split by %c, consume CSS args in order
	parts := strings.Split(firstStr, "%c")
	var segments []protocol.StyledSegment
	cssArgIdx := 1 // CSS args start at index 1

	for i, part := range parts {
		if i == 0 && part == "" {
			continue // leading empty before first %c
		}
		if i == 0 {
			// Text before the first %c — no CSS
			segments = append(segments, protocol.StyledSegment{Text: part})
		} else {
			css := ""
			if cssArgIdx < len(args) {
				css = formatArg(args[cssArgIdx])
			}
			cssArgIdx++
			segments = append(segments, protocol.StyledSegment{Text: part, CSS: css})
		}
	}

	// Append remaining non-CSS args as plain text
	for i := cssArgIdx; i < len(args); i++ {
		s := formatArg(args[i])
		if s != "" {
			segments = append(segments, protocol.StyledSegment{Text: " " + s})
		}
	}

	// Build plain text from all segments
	var sb strings.Builder
	for _, seg := range segments {
		sb.WriteString(seg.Text)
	}

	return consoleArgsResult{Text: sb.String(), Segments: segments}
}

// formatConsoleArgsPlain converts args to a plain string (no %c handling).
func formatConsoleArgsPlain(args []consoleArg) string {
	if len(args) == 1 {
		return formatArg(args[0])
	}
	result := ""
	for i, arg := range args {
		if i > 0 {
			result += " "
		}
		result += formatArg(arg)
	}
	return result
}

func formatArg(arg consoleArg) string {
	if arg.Description != "" {
		return arg.Description
	}
	if arg.Type == "string" {
		if s, ok := arg.Value.(string); ok {
			return s
		}
	}
	if arg.Value != nil {
		b, _ := json.Marshal(arg.Value)
		return string(b)
	}
	return fmt.Sprintf("[%s]", arg.Type)
}
