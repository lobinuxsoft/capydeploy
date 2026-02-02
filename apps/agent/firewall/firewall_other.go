//go:build !windows

package firewall

// EnsureRules is a no-op on non-Windows platforms.
func EnsureRules(httpPort int) error {
	return nil
}

// RemoveRules is a no-op on non-Windows platforms.
func RemoveRules() error {
	return nil
}
