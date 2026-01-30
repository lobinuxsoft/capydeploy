// Package main provides the entry point for CapyDeploy Agent.
// Agent runs on remote devices (handhelds) and exposes HTTP/WebSocket endpoints
// for the Hub to discover and communicate with.
package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"

	"github.com/lobinuxsoft/capydeploy/apps/agent/server"
	"github.com/lobinuxsoft/capydeploy/pkg/discovery"
)

// Version is set at build time.
var Version = "dev"

func main() {
	var (
		port    int
		name    string
		verbose bool
	)

	flag.IntVar(&port, "port", discovery.DefaultPort, "HTTP server port")
	flag.StringVar(&name, "name", "", "Agent name (default: hostname)")
	flag.BoolVar(&verbose, "verbose", false, "Enable verbose logging")
	flag.Parse()

	if name == "" {
		name = discovery.GetHostname()
	}

	// Setup context with signal handling
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, os.Interrupt, syscall.SIGTERM)
	go func() {
		<-sigCh
		log.Println("Shutting down...")
		cancel()
	}()

	// Create and configure agent server
	cfg := server.Config{
		Port:     port,
		Name:     name,
		Version:  Version,
		Platform: discovery.GetPlatform(),
		Verbose:  verbose,
	}

	agent, err := server.New(cfg)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error creating agent: %v\n", err)
		os.Exit(1)
	}

	log.Printf("CapyDeploy Agent v%s starting on port %d", Version, port)
	log.Printf("Platform: %s, Name: %s", cfg.Platform, cfg.Name)

	if err := agent.Run(ctx); err != nil && err != context.Canceled {
		fmt.Fprintf(os.Stderr, "Error running agent: %v\n", err)
		os.Exit(1)
	}

	log.Println("Agent stopped")
}
