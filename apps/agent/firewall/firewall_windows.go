//go:build windows

package firewall

import (
	"fmt"
	"os/exec"
	"strings"
)

const (
	ruleName     = "CapyDeploy Agent"
	ruleNameMDNS = "CapyDeploy Agent mDNS"
)

// EnsureRules ensures the necessary firewall rules exist for the Agent.
// Returns nil if rules exist or were created successfully.
// Returns an error if rules couldn't be created (e.g., no admin rights).
func EnsureRules(httpPort int) error {
	var errors []string

	// Check/create HTTP rule
	if !ruleExists(ruleName) {
		if err := createHTTPRule(httpPort); err != nil {
			errors = append(errors, fmt.Sprintf("HTTP rule: %v", err))
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

// createHTTPRule creates a firewall rule for the HTTP server.
func createHTTPRule(port int) error {
	cmd := exec.Command("netsh", "advfirewall", "firewall", "add", "rule",
		fmt.Sprintf("name=%s", ruleName),
		"dir=in",
		"action=allow",
		"protocol=TCP",
		fmt.Sprintf("localport=%d", port),
		"profile=any",
		fmt.Sprintf("description=Allow incoming connections to CapyDeploy Agent on port %d", port),
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
			errors = append(errors, fmt.Sprintf("HTTP rule: %v: %s", err, string(output)))
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
