package modules

import (
	"fmt"
	"sync"
)

// Registry manages platform modules and provides automatic module selection.
type Registry struct {
	mu      sync.RWMutex
	modules map[string]PlatformModule
}

// NewRegistry creates a new module registry with default modules registered.
func NewRegistry() *Registry {
	r := &Registry{
		modules: make(map[string]PlatformModule),
	}

	// Register default modules
	r.Register(NewLinuxModule())
	r.Register(NewWindowsModule())

	return r
}

// Register adds a platform module to the registry.
func (r *Registry) Register(module PlatformModule) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.modules[module.Platform()] = module
}

// Get returns the module for a specific platform.
// Returns nil if the platform is not registered.
func (r *Registry) Get(platform string) PlatformModule {
	r.mu.RLock()
	defer r.mu.RUnlock()
	return r.modules[platform]
}

// GetClient creates a client for the specified platform.
// Returns an error if the platform is not supported.
func (r *Registry) GetClient(platform, host string, port int) (PlatformClient, error) {
	module := r.Get(platform)
	if module == nil {
		return nil, fmt.Errorf("unsupported platform: %s", platform)
	}
	return module.NewClient(host, port), nil
}

// Platforms returns a list of all registered platform names.
func (r *Registry) Platforms() []string {
	r.mu.RLock()
	defer r.mu.RUnlock()

	platforms := make([]string, 0, len(r.modules))
	for p := range r.modules {
		platforms = append(platforms, p)
	}
	return platforms
}

// IsSupported checks if a platform is supported.
func (r *Registry) IsSupported(platform string) bool {
	return r.Get(platform) != nil
}

// DefaultRegistry is the global default registry with standard modules.
var DefaultRegistry = NewRegistry()

// GetModule returns a module from the default registry.
func GetModule(platform string) PlatformModule {
	return DefaultRegistry.Get(platform)
}

// GetClientForPlatform creates a client from the default registry.
func GetClientForPlatform(platform, host string, port int) (PlatformClient, error) {
	return DefaultRegistry.GetClient(platform, host, port)
}

// IsPlatformSupported checks if a platform is supported in the default registry.
func IsPlatformSupported(platform string) bool {
	return DefaultRegistry.IsSupported(platform)
}
