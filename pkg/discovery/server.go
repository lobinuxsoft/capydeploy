package discovery

import (
	"context"
	"fmt"
	"net"
	"os"
	"runtime"
	"strings"

	"github.com/grandcat/zeroconf"
)

// Server advertises an agent on the local network via mDNS/DNS-SD.
type Server struct {
	info   ServiceInfo
	server *zeroconf.Server
}

// NewServer creates a new mDNS server for advertising an agent.
func NewServer(info ServiceInfo) *Server {
	return &Server{info: info}
}

// Start begins advertising the agent on the network.
// The port must be set before calling Start (no longer defaults to DefaultPort).
func (s *Server) Start() error {
	if s.info.Port == 0 {
		return fmt.Errorf("port must be set before starting mDNS server")
	}

	// Build TXT records with agent info
	txt := []string{
		"id=" + s.info.ID,
		"name=" + s.info.Name,
		"platform=" + s.info.Platform,
		"version=" + s.info.Version,
	}

	// Get local IPs, filtering out loopback and link-local
	ips, err := getLocalIPs()
	if err != nil || len(ips) == 0 {
		return fmt.Errorf("no valid network IPs found: %w", err)
	}

	// Convert to string slice for RegisterProxy
	ipStrings := make([]string, len(ips))
	for i, ip := range ips {
		ipStrings[i] = ip.String()
	}

	// Get hostname for mDNS
	hostname := GetHostname()

	// Register service using zeroconf with explicit IPs (no loopback)
	server, err := zeroconf.RegisterProxy(
		s.info.ID,      // Instance name
		ServiceName,    // Service type (_capydeploy._tcp)
		"local.",       // Domain
		s.info.Port,    // Port
		hostname,       // Host
		ipStrings,      // IPs (filtered, no loopback)
		txt,            // TXT records
		nil,            // Interfaces (nil = all)
	)
	if err != nil {
		return fmt.Errorf("failed to register mDNS service: %w", err)
	}

	s.server = server
	return nil
}

// Stop stops advertising the agent.
func (s *Server) Stop() error {
	if s.server != nil {
		s.server.Shutdown()
		s.server = nil
	}
	return nil
}

// Info returns the service info being advertised.
func (s *Server) Info() ServiceInfo {
	return s.info
}

// getLocalIPs returns the local non-loopback IPv4 addresses, excluding link-local (APIPA).
func getLocalIPs() ([]net.IP, error) {
	var ips []net.IP

	interfaces, err := net.Interfaces()
	if err != nil {
		return nil, err
	}

	for _, iface := range interfaces {
		// Skip down or loopback interfaces
		if iface.Flags&net.FlagUp == 0 || iface.Flags&net.FlagLoopback != 0 {
			continue
		}

		addrs, err := iface.Addrs()
		if err != nil {
			continue
		}

		for _, addr := range addrs {
			var ip net.IP
			switch v := addr.(type) {
			case *net.IPNet:
				ip = v.IP
			case *net.IPAddr:
				ip = v.IP
			}

			// Only include IPv4 addresses
			ip4 := ip.To4()
			if ip4 == nil || ip.IsLoopback() {
				continue
			}

			// Skip link-local addresses (169.254.x.x / APIPA)
			if ip4[0] == 169 && ip4[1] == 254 {
				continue
			}

			ips = append(ips, ip)
		}
	}

	return ips, nil
}

// GetHostname returns the local hostname.
func GetHostname() string {
	hostname, err := os.Hostname()
	if err != nil {
		return "unknown"
	}
	return hostname
}

// GetPlatform returns the current platform identifier.
func GetPlatform() string {
	return detectPlatform()
}

// detectPlatform attempts to detect the running platform.
func detectPlatform() string {
	// Check OS first
	if runtime.GOOS == "windows" {
		return "windows"
	}

	if runtime.GOOS != "linux" {
		return runtime.GOOS
	}

	// Check OS release first (most reliable method)
	if data, err := os.ReadFile("/etc/os-release"); err == nil {
		content := strings.ToLower(string(data))
		// SteamOS is the real Steam Deck
		if strings.Contains(content, "steamos") {
			return "steamdeck"
		}
		// ChimeraOS
		if strings.Contains(content, "chimeraos") {
			return "chimeraos"
		}
		// Bazzite (Fedora-based gaming distro, NOT a Steam Deck)
		if strings.Contains(content, "bazzite") {
			return "linux"
		}
	}

	// Check for handheld-specific files (fallback)
	if fileExists("/usr/share/plymouth/themes/legion-go") {
		return "legiongologo"
	}
	if fileExists("/usr/share/plymouth/themes/rogally") {
		return "rogally"
	}

	// Only check /home/deck if it's a real directory (not a symlink)
	// This avoids false positives on Bazzite which symlinks /home/deck
	if info, err := os.Lstat("/home/deck"); err == nil {
		if info.Mode()&os.ModeSymlink == 0 && info.IsDir() {
			return "steamdeck"
		}
	}

	return "linux"
}

func fileExists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}

// RunContext runs the server until the context is cancelled.
func (s *Server) RunContext(ctx context.Context) error {
	if err := s.Start(); err != nil {
		return err
	}

	<-ctx.Done()
	return s.Stop()
}
