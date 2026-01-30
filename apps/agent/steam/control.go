// Package steam provides Steam control operations for the Agent.
package steam

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
