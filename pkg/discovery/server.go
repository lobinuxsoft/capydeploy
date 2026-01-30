package discovery

import (
	"context"
	"fmt"
	"net"
	"os"
	"runtime"
	"strings"

	"github.com/hashicorp/mdns"
)

// Server advertises an agent on the local network via mDNS.
type Server struct {
	info   ServiceInfo
	server *mdns.Server
}

// NewServer creates a new mDNS server for advertising an agent.
func NewServer(info ServiceInfo) *Server {
	return &Server{info: info}
}

// Start begins advertising the agent on the network.
func (s *Server) Start() error {
	if s.info.Port == 0 {
		s.info.Port = DefaultPort
	}

	// Get local IPs if not provided
	if len(s.info.IPs) == 0 {
		ips, err := getLocalIPs()
		if err != nil {
			return fmt.Errorf("failed to get local IPs: %w", err)
		}
		s.info.IPs = ips
	}

	// Build TXT records with agent info
	txt := []string{
		"id=" + s.info.ID,
		"name=" + s.info.Name,
		"platform=" + s.info.Platform,
		"version=" + s.info.Version,
	}

	// Create mDNS service
	service, err := mdns.NewMDNSService(
		s.info.ID,         // Instance name
		ServiceName,       // Service type
		"",                // Domain (default: local)
		"",                // Host (auto-detect)
		s.info.Port,       // Port
		s.info.IPs,        // IPs to advertise
		txt,               // TXT records
	)
	if err != nil {
		return fmt.Errorf("failed to create mDNS service: %w", err)
	}

	// Create and start server
	server, err := mdns.NewServer(&mdns.Config{Zone: service})
	if err != nil {
		return fmt.Errorf("failed to start mDNS server: %w", err)
	}

	s.server = server
	return nil
}

// Stop stops advertising the agent.
func (s *Server) Stop() error {
	if s.server != nil {
		return s.server.Shutdown()
	}
	return nil
}

// Info returns the service info being advertised.
func (s *Server) Info() ServiceInfo {
	return s.info
}

// getLocalIPs returns the local non-loopback IPv4 addresses.
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
			if ip == nil || ip.IsLoopback() || ip.To4() == nil {
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
	// This will be set by build tags in actual implementation
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

	// Check for common handheld devices (Linux only)
	if fileExists("/home/deck") {
		return "steamdeck"
	}
	if fileExists("/usr/share/plymouth/themes/legion-go") {
		return "legiongologo"
	}
	if fileExists("/usr/share/plymouth/themes/rogally") {
		return "rogally"
	}

	// Check OS release for more info
	if data, err := os.ReadFile("/etc/os-release"); err == nil {
		content := strings.ToLower(string(data))
		if strings.Contains(content, "steamos") {
			return "steamdeck"
		}
		if strings.Contains(content, "chimeraos") {
			return "chimeraos"
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
