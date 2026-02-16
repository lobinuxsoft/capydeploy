// Package steam provides Steam control operations for the Agent.
package steam

import (
	"fmt"
	"net/http"
	"time"
)

const (
	// cefTimeout is the max time to wait for CEF to be available.
	cefTimeout = 30 * time.Second
	// cefCheckInterval is the interval between CEF availability checks.
	cefCheckInterval = 2 * time.Second
	// shutdownTimeout is the max time to wait for Steam to close.
	shutdownTimeout = 10 * time.Second
)

// Controller manages Steam process operations.
type Controller struct{}

// NewController creates a new Steam controller.
func NewController() *Controller {
	return &Controller{}
}

// RestartResult contains the result of a Steam restart operation.
type RestartResult struct {
	Success bool   `json:"success"`
	Message string `json:"message"`
}

// IsCEFAvailable checks if Steam's CEF debugger is responding.
func (c *Controller) IsCEFAvailable() bool {
	client := &http.Client{Timeout: 2 * time.Second}
	resp, err := client.Get(cefDebugEndpoint)
	if err != nil {
		return false
	}
	defer resp.Body.Close()
	return resp.StatusCode == http.StatusOK
}

// WaitForCEF waits until Steam's CEF debugger is available or timeout.
func (c *Controller) WaitForCEF() error {
	deadline := time.Now().Add(cefTimeout)

	for time.Now().Before(deadline) {
		if c.IsCEFAvailable() {
			return nil
		}
		time.Sleep(cefCheckInterval)
	}

	return fmt.Errorf("timeout waiting for Steam CEF (waited %v)", cefTimeout)
}

// EnsureRunning makes sure Steam is running and CEF is available.
// If Steam is not running, it starts it and waits for CEF.
func (c *Controller) EnsureRunning() error {
	if c.IsCEFAvailable() {
		return nil
	}

	if !c.IsRunning() {
		if err := c.Start(); err != nil {
			return err
		}
	}

	return c.WaitForCEF()
}
