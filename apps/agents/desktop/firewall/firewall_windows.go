//go:build windows

package firewall

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
)

const (
	ruleName     = "CapyDeploy Agent"
	ruleNameMDNS = "CapyDeploy Agent mDNS"
)

// EnsureRules ensures the necessary firewall rules exist for the Agent.
// Uses program-based rules instead of port-based rules to support dynamic ports.
// Returns nil if rules exist or were created successfully.
// Returns an error if rules couldn't be created (e.g., no admin rights).
func EnsureRules(_ int) error {
	var errors []string

	// Check/create program rule (allows Agent on any port)
	if !ruleExists(ruleName) {
		if err := createProgramRule(); err != nil {
			errors = append(errors, fmt.Sprintf("program rule: %v", err))
		}
	}

	// Check/create mDNS rule
	if !ruleExists(ruleNameMDNS) {
		if err := createMDNSRule(); err != nil {
			errors = append(errors, fmt.Sprintf("mDNS rule: %v", err))
		}
	}

	if len(errors) > 0 {
		return fmt.Errorf("failed to create firewall rules: %s", strings.Join(errors, "; "))
	}
	return nil
}

// ruleExists checks if a firewall rule with the given name exists.
func ruleExists(name string) bool {
	cmd := exec.Command("netsh", "advfirewall", "firewall", "show", "rule", fmt.Sprintf("name=%s", name))
	output, _ := cmd.CombinedOutput()
	return !strings.Contains(string(output), "No rules match")
}

// createProgramRule creates a firewall rule based on the program executable.
// This allows dynamic ports without needing to update the firewall rule.
func createProgramRule() error {
	exePath, err := os.Executable()
	if err != nil {
		return fmt.Errorf("failed to get executable path: %w", err)
	}

	cmd := exec.Command("netsh", "advfirewall", "firewall", "add", "rule",
		fmt.Sprintf("name=%s", ruleName),
		"dir=in",
		"action=allow",
		fmt.Sprintf("program=%s", exePath),
		"profile=any",
		"description=Allow incoming connections to CapyDeploy Agent",
	)
	output, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("%v: %s", err, string(output))
	}
	return nil
}

// createMDNSRule creates firewall rules for mDNS discovery.
func createMDNSRule() error {
	// Inbound UDP 5353 for mDNS
	cmd := exec.Command("netsh", "advfirewall", "firewall", "add", "rule",
		fmt.Sprintf("name=%s", ruleNameMDNS),
		"dir=in",
		"action=allow",
		"protocol=UDP",
		"localport=5353",
		"profile=any",
		"description=Allow mDNS discovery for CapyDeploy Agent",
	)
	output, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("%v: %s", err, string(output))
	}
	return nil
}

// RemoveRules removes the firewall rules created by the Agent.
func RemoveRules() error {
	var errors []string

	if ruleExists(ruleName) {
		cmd := exec.Command("netsh", "advfirewall", "firewall", "delete", "rule", fmt.Sprintf("name=%s", ruleName))
		if output, err := cmd.CombinedOutput(); err != nil {
			errors = append(errors, fmt.Sprintf("program rule: %v: %s", err, string(output)))
		}
	}

	if ruleExists(ruleNameMDNS) {
		cmd := exec.Command("netsh", "advfirewall", "firewall", "delete", "rule", fmt.Sprintf("name=%s", ruleNameMDNS))
		if output, err := cmd.CombinedOutput(); err != nil {
			errors = append(errors, fmt.Sprintf("mDNS rule: %v: %s", err, string(output)))
		}
	}

	if len(errors) > 0 {
		return fmt.Errorf("failed to remove firewall rules: %s", strings.Join(errors, "; "))
	}
	return nil
}
