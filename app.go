package main

import (
	"context"
	"encoding/base64"
	"fmt"
	"io"
	"net"
	"net/http"
	"os"
	"path"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"github.com/wailsapp/wails/v2/pkg/runtime"

	"github.com/lobinuxsoft/bazzite-devkit/internal/config"
	"github.com/lobinuxsoft/bazzite-devkit/internal/device"
	"github.com/lobinuxsoft/bazzite-devkit/internal/shortcuts"
	"github.com/lobinuxsoft/bazzite-devkit/internal/steamgriddb"
)

// App struct holds the application state
type App struct {
	ctx             context.Context
	connectedDevice *ConnectedDevice
	mu              sync.RWMutex
}

// ConnectedDevice represents a connected device with its client
type ConnectedDevice struct {
	Config config.DeviceConfig
	Client *device.Client
}

// ConnectionStatus represents the current connection status
type ConnectionStatus struct {
	Connected  bool   `json:"connected"`
	DeviceName string `json:"deviceName"`
	Host       string `json:"host"`
	Port       int    `json:"port"`
}

// NetworkDevice represents a device found on the network
type NetworkDevice struct {
	IP       string `json:"ip"`
	Hostname string `json:"hostname"`
	HasSSH   bool   `json:"hasSSH"`
}

// InstalledGame represents a game installed on the remote device
type InstalledGame struct {
	Name string `json:"name"`
	Path string `json:"path"`
	Size string `json:"size"`
}

// UploadProgress represents upload progress data
type UploadProgress struct {
	Progress float64 `json:"progress"`
	Status   string  `json:"status"`
	Error    string  `json:"error,omitempty"`
	Done     bool    `json:"done"`
}

// NewApp creates a new App application struct
func NewApp() *App {
	return &App{}
}

// startup is called when the app starts
func (a *App) startup(ctx context.Context) {
	a.ctx = ctx
}

// shutdown is called when the app is closing
func (a *App) shutdown(ctx context.Context) {
	a.mu.Lock()
	defer a.mu.Unlock()
	if a.connectedDevice != nil && a.connectedDevice.Client != nil {
		a.connectedDevice.Client.Close()
	}
}

// =============================================================================
// Device Management
// =============================================================================

// GetDevices returns all saved devices
func (a *App) GetDevices() ([]config.DeviceConfig, error) {
	return config.GetDevices()
}

// AddDevice adds a new device
func (a *App) AddDevice(dev config.DeviceConfig) error {
	return config.AddDevice(dev)
}

// UpdateDevice updates an existing device
func (a *App) UpdateDevice(oldHost string, dev config.DeviceConfig) error {
	return config.UpdateDevice(oldHost, dev)
}

// RemoveDevice removes a device
func (a *App) RemoveDevice(host string) error {
	// Disconnect if this is the connected device
	a.mu.RLock()
	if a.connectedDevice != nil && a.connectedDevice.Config.Host == host {
		a.mu.RUnlock()
		a.DisconnectDevice()
	} else {
		a.mu.RUnlock()
	}
	return config.RemoveDevice(host)
}

// ConnectDevice connects to a device by host
func (a *App) ConnectDevice(host string) error {
	// Get device config
	devices, err := config.GetDevices()
	if err != nil {
		return fmt.Errorf("failed to get devices: %w", err)
	}

	var deviceCfg *config.DeviceConfig
	for _, d := range devices {
		if d.Host == host {
			deviceCfg = &d
			break
		}
	}

	if deviceCfg == nil {
		return fmt.Errorf("device not found: %s", host)
	}

	// Disconnect existing connection
	a.mu.Lock()
	if a.connectedDevice != nil && a.connectedDevice.Client != nil {
		a.connectedDevice.Client.Close()
		a.connectedDevice = nil
	}
	a.mu.Unlock()

	// Create and connect client
	client, err := device.NewClient(deviceCfg.Host, deviceCfg.Port, deviceCfg.User, deviceCfg.Password, deviceCfg.KeyFile)
	if err != nil {
		return fmt.Errorf("failed to create client: %w", err)
	}

	if err := client.Connect(); err != nil {
		return fmt.Errorf("connection failed: %w", err)
	}

	a.mu.Lock()
	a.connectedDevice = &ConnectedDevice{
		Config: *deviceCfg,
		Client: client,
	}
	a.mu.Unlock()

	// Emit connection status change
	runtime.EventsEmit(a.ctx, "connection:changed", a.GetConnectionStatus())

	return nil
}

// DisconnectDevice disconnects from the current device
func (a *App) DisconnectDevice() {
	a.mu.Lock()
	if a.connectedDevice != nil && a.connectedDevice.Client != nil {
		a.connectedDevice.Client.Close()
	}
	a.connectedDevice = nil
	a.mu.Unlock()

	// Emit connection status change
	runtime.EventsEmit(a.ctx, "connection:changed", a.GetConnectionStatus())
}

// GetConnectionStatus returns the current connection status
func (a *App) GetConnectionStatus() ConnectionStatus {
	a.mu.RLock()
	defer a.mu.RUnlock()

	if a.connectedDevice == nil {
		return ConnectionStatus{Connected: false}
	}

	return ConnectionStatus{
		Connected:  true,
		DeviceName: a.connectedDevice.Config.Name,
		Host:       a.connectedDevice.Config.Host,
		Port:       a.connectedDevice.Config.Port,
	}
}

// ScanNetwork scans the local network for devices with SSH
func (a *App) ScanNetwork() ([]NetworkDevice, error) {
	var found []NetworkDevice
	var mu sync.Mutex
	var wg sync.WaitGroup

	localIP := getLocalIP()
	if localIP == "" {
		return nil, fmt.Errorf("could not determine local IP address")
	}

	parts := strings.Split(localIP, ".")
	if len(parts) != 4 {
		return nil, fmt.Errorf("invalid local IP format")
	}
	baseIP := strings.Join(parts[:3], ".")

	semaphore := make(chan struct{}, 50)

	for i := 1; i <= 254; i++ {
		wg.Add(1)
		go func(ip string) {
			defer wg.Done()
			semaphore <- struct{}{}
			defer func() { <-semaphore }()

			if hasSSH(ip) {
				hostname := getHostname(ip)
				mu.Lock()
				found = append(found, NetworkDevice{
					IP:       ip,
					Hostname: hostname,
					HasSSH:   true,
				})
				mu.Unlock()
			}
		}(fmt.Sprintf("%s.%d", baseIP, i))
	}

	wg.Wait()
	return found, nil
}

// =============================================================================
// Game Setup Management
// =============================================================================

// GetGameSetups returns all saved game setups
func (a *App) GetGameSetups() ([]config.GameSetup, error) {
	return config.GetGameSetups()
}

// AddGameSetup adds a new game setup
func (a *App) AddGameSetup(setup config.GameSetup) error {
	return config.AddGameSetup(setup)
}

// UpdateGameSetup updates an existing game setup
func (a *App) UpdateGameSetup(id string, setup config.GameSetup) error {
	return config.UpdateGameSetup(id, setup)
}

// RemoveGameSetup removes a game setup
func (a *App) RemoveGameSetup(id string) error {
	return config.RemoveGameSetup(id)
}

// SelectFolder opens a folder selection dialog
func (a *App) SelectFolder() (string, error) {
	return runtime.OpenDirectoryDialog(a.ctx, runtime.OpenDialogOptions{
		Title: "Select Game Folder",
	})
}

// UploadGame uploads a game to the remote device
func (a *App) UploadGame(setupID string) error {
	a.mu.RLock()
	if a.connectedDevice == nil || a.connectedDevice.Client == nil {
		a.mu.RUnlock()
		return fmt.Errorf("no device connected")
	}
	client := a.connectedDevice.Client
	deviceCfg := a.connectedDevice.Config
	a.mu.RUnlock()

	// Get the game setup
	setups, err := config.GetGameSetups()
	if err != nil {
		return fmt.Errorf("failed to get game setups: %w", err)
	}

	var setup *config.GameSetup
	for _, s := range setups {
		if s.ID == setupID {
			setup = &s
			break
		}
	}

	if setup == nil {
		return fmt.Errorf("game setup not found: %s", setupID)
	}

	// Start upload in goroutine
	go a.performUpload(client, &deviceCfg, setup)

	return nil
}

func (a *App) performUpload(client *device.Client, deviceCfg *config.DeviceConfig, setup *config.GameSetup) {
	emitProgress := func(progress float64, status string, err string, done bool) {
		runtime.EventsEmit(a.ctx, "upload:progress", UploadProgress{
			Progress: progress,
			Status:   status,
			Error:    err,
			Done:     done,
		})
	}

	emitProgress(0, "Preparing upload...", "", false)

	// Expand remote path
	remotePath := setup.RemotePath
	if strings.HasPrefix(remotePath, "~") {
		homeDir, err := client.GetHomeDir()
		if err != nil {
			emitProgress(0, "", fmt.Sprintf("Failed to expand remote path: %v", err), true)
			return
		}
		remotePath = strings.Replace(remotePath, "~", homeDir, 1)
	}

	remoteGamePath := path.Join(remotePath, setup.Name)

	// Create remote directory
	emitProgress(0.05, "Creating remote directory...", "", false)
	if err := client.MkdirAll(remoteGamePath); err != nil {
		emitProgress(0, "", fmt.Sprintf("Failed to create directory: %v", err), true)
		return
	}

	// Get list of files
	emitProgress(0.1, "Scanning files...", "", false)
	files, err := getFilesToUpload(setup.LocalPath)
	if err != nil {
		emitProgress(0, "", fmt.Sprintf("Failed to scan files: %v", err), true)
		return
	}

	// Upload files
	totalFiles := len(files)
	for i, file := range files {
		relPath, _ := filepath.Rel(setup.LocalPath, file)
		relPath = strings.ReplaceAll(relPath, "\\", "/")
		remoteDest := path.Join(remoteGamePath, relPath)

		remoteDir := path.Dir(remoteDest)
		client.MkdirAll(remoteDir)

		progress := 0.1 + (float64(i)/float64(totalFiles))*0.75
		emitProgress(progress, fmt.Sprintf("Uploading: %s", relPath), "", false)

		if err := client.UploadFile(file, remoteDest); err != nil {
			emitProgress(0, "", fmt.Sprintf("Failed to upload %s: %v", relPath, err), true)
			return
		}
	}

	emitProgress(0.85, "Setting executable permissions...", "", false)

	exePath := path.Join(remoteGamePath, setup.Executable)
	chmodCmd := fmt.Sprintf("chmod +x %q", exePath)
	if _, err := client.RunCommand(chmodCmd); err != nil {
		emitProgress(0, "", fmt.Sprintf("Failed to set permissions: %v", err), true)
		return
	}

	// Set executable permissions on common executable files
	chmodAllCmd := fmt.Sprintf("find %q -type f \\( -name '*.sh' -o -name '*.x86_64' -o -name '*.x86' \\) -exec chmod +x {} \\;", remoteGamePath)
	client.RunCommand(chmodAllCmd)

	emitProgress(0.9, "Creating Steam shortcut...", "", false)

	// Prepare artwork config
	var artworkCfg *shortcuts.ArtworkConfig
	if setup.GridPortrait != "" || setup.GridLandscape != "" || setup.HeroImage != "" ||
		setup.LogoImage != "" || setup.IconImage != "" {
		artworkCfg = &shortcuts.ArtworkConfig{
			GridPortrait:  setup.GridPortrait,
			GridLandscape: setup.GridLandscape,
			HeroImage:     setup.HeroImage,
			LogoImage:     setup.LogoImage,
			IconImage:     setup.IconImage,
		}
		// Debug: log artwork URLs being used
		fmt.Printf("[DEBUG] Setup artwork config from GameSetup:\n")
		fmt.Printf("  Name: %s\n", setup.Name)
		fmt.Printf("  GridDBGameID: %d\n", setup.GridDBGameID)
		fmt.Printf("  GridPortrait: %s\n", setup.GridPortrait)
		fmt.Printf("  GridLandscape: %s\n", setup.GridLandscape)
		fmt.Printf("  HeroImage: %s\n", setup.HeroImage)
		fmt.Printf("  LogoImage: %s\n", setup.LogoImage)
		fmt.Printf("  IconImage: %s\n", setup.IconImage)
	}

	remoteCfg := &shortcuts.RemoteConfig{
		Host:     deviceCfg.Host,
		Port:     deviceCfg.Port,
		User:     deviceCfg.User,
		Password: deviceCfg.Password,
		KeyFile:  deviceCfg.KeyFile,
	}

	tags := shortcuts.ParseTags(setup.Tags)
	if err := shortcuts.AddShortcutWithArtwork(remoteCfg, setup.Name, exePath, remoteGamePath, setup.LaunchOptions, tags, artworkCfg); err != nil {
		emitProgress(0, "", fmt.Sprintf("Failed to create shortcut: %v", err), true)
		return
	}

	shortcuts.RefreshSteamLibrary(remoteCfg)

	emitProgress(1.0, "Upload complete!", "", true)
}

// =============================================================================
// Installed Games Management
// =============================================================================

// GetInstalledGames returns games installed on the remote device
func (a *App) GetInstalledGames(remotePath string) ([]InstalledGame, error) {
	a.mu.RLock()
	if a.connectedDevice == nil || a.connectedDevice.Client == nil {
		a.mu.RUnlock()
		return nil, fmt.Errorf("no device connected")
	}
	client := a.connectedDevice.Client
	a.mu.RUnlock()

	// Expand remote path
	if strings.HasPrefix(remotePath, "~") {
		homeDir, err := client.GetHomeDir()
		if err != nil {
			return nil, fmt.Errorf("failed to expand path: %w", err)
		}
		remotePath = strings.Replace(remotePath, "~", homeDir, 1)
	}

	// List directories
	cmd := fmt.Sprintf("ls -1 %s 2>/dev/null || echo ''", remotePath)
	output, err := client.RunCommand(cmd)
	if err != nil {
		return []InstalledGame{}, nil
	}

	lines := strings.Split(strings.TrimSpace(output), "\n")
	var games []InstalledGame

	for _, line := range lines {
		name := strings.TrimSpace(line)
		if name == "" {
			continue
		}

		gamePath := path.Join(remotePath, name)

		sizeCmd := fmt.Sprintf("du -sh %q 2>/dev/null | cut -f1", gamePath)
		sizeOutput, _ := client.RunCommand(sizeCmd)
		size := strings.TrimSpace(sizeOutput)
		if size == "" {
			size = "Unknown"
		}

		games = append(games, InstalledGame{
			Name: name,
			Path: gamePath,
			Size: size,
		})
	}

	return games, nil
}

// DeleteGame deletes a game from the remote device
func (a *App) DeleteGame(name, gamePath string) error {
	a.mu.RLock()
	if a.connectedDevice == nil || a.connectedDevice.Client == nil {
		a.mu.RUnlock()
		return fmt.Errorf("no device connected")
	}
	client := a.connectedDevice.Client
	deviceCfg := a.connectedDevice.Config
	a.mu.RUnlock()

	// Remove Steam shortcut
	remoteCfg := &shortcuts.RemoteConfig{
		Host:     deviceCfg.Host,
		Port:     deviceCfg.Port,
		User:     deviceCfg.User,
		Password: deviceCfg.Password,
		KeyFile:  deviceCfg.KeyFile,
	}

	shortcuts.RemoveShortcut(remoteCfg, name)
	shortcuts.RefreshSteamLibrary(remoteCfg)

	// Delete game files
	cmd := fmt.Sprintf("rm -rf %q", gamePath)
	_, err := client.RunCommand(cmd)
	if err != nil {
		return fmt.Errorf("failed to delete game files: %w", err)
	}

	return nil
}

// =============================================================================
// Settings
// =============================================================================

// GetSteamGridDBAPIKey returns the SteamGridDB API key
func (a *App) GetSteamGridDBAPIKey() (string, error) {
	return config.GetSteamGridDBAPIKey()
}

// SetSteamGridDBAPIKey saves the SteamGridDB API key
func (a *App) SetSteamGridDBAPIKey(apiKey string) error {
	return config.SetSteamGridDBAPIKey(apiKey)
}

// GetCacheSize returns the size of the image cache
func (a *App) GetCacheSize() (int64, error) {
	return steamgriddb.GetCacheSize()
}

// ClearImageCache clears the image cache
func (a *App) ClearImageCache() error {
	return steamgriddb.ClearImageCache()
}

// OpenCacheFolder opens the cache folder in the file explorer
func (a *App) OpenCacheFolder() error {
	cacheDir, err := steamgriddb.GetImageCacheDir()
	if err != nil {
		return err
	}
	runtime.BrowserOpenURL(a.ctx, "file://"+cacheDir)
	return nil
}

// =============================================================================
// SteamGridDB
// =============================================================================

// SearchGames searches for games on SteamGridDB
func (a *App) SearchGames(query string) ([]steamgriddb.SearchResult, error) {
	apiKey, err := config.GetSteamGridDBAPIKey()
	if err != nil || apiKey == "" {
		return nil, fmt.Errorf("SteamGridDB API key not configured")
	}

	client := steamgriddb.NewClient(apiKey)
	return client.Search(query)
}

// GetGrids returns grid images for a game
func (a *App) GetGrids(gameID int, filters steamgriddb.ImageFilters, page int) ([]steamgriddb.GridData, error) {
	apiKey, err := config.GetSteamGridDBAPIKey()
	if err != nil || apiKey == "" {
		return nil, fmt.Errorf("SteamGridDB API key not configured")
	}

	client := steamgriddb.NewClient(apiKey)
	return client.GetGrids(gameID, &filters, page)
}

// GetHeroes returns hero images for a game
func (a *App) GetHeroes(gameID int, filters steamgriddb.ImageFilters, page int) ([]steamgriddb.ImageData, error) {
	apiKey, err := config.GetSteamGridDBAPIKey()
	if err != nil || apiKey == "" {
		return nil, fmt.Errorf("SteamGridDB API key not configured")
	}

	client := steamgriddb.NewClient(apiKey)
	return client.GetHeroes(gameID, &filters, page)
}

// GetLogos returns logo images for a game
func (a *App) GetLogos(gameID int, filters steamgriddb.ImageFilters, page int) ([]steamgriddb.ImageData, error) {
	apiKey, err := config.GetSteamGridDBAPIKey()
	if err != nil || apiKey == "" {
		return nil, fmt.Errorf("SteamGridDB API key not configured")
	}

	client := steamgriddb.NewClient(apiKey)
	return client.GetLogos(gameID, &filters, page)
}

// GetIcons returns icon images for a game
func (a *App) GetIcons(gameID int, filters steamgriddb.ImageFilters, page int) ([]steamgriddb.ImageData, error) {
	apiKey, err := config.GetSteamGridDBAPIKey()
	if err != nil || apiKey == "" {
		return nil, fmt.Errorf("SteamGridDB API key not configured")
	}

	client := steamgriddb.NewClient(apiKey)
	return client.GetIcons(gameID, &filters, page)
}

// ProxyImage fetches an image from URL and returns it as a base64 data URL
// This is needed because WebView2 may block external images
func (a *App) ProxyImage(imageURL string) (string, error) {
	if imageURL == "" {
		return "", fmt.Errorf("empty URL")
	}

	// Fetch the image
	resp, err := http.Get(imageURL)
	if err != nil {
		return "", fmt.Errorf("failed to fetch image: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("HTTP error: %d", resp.StatusCode)
	}

	// Read image data
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("failed to read image: %w", err)
	}

	// Determine MIME type
	contentType := resp.Header.Get("Content-Type")
	if contentType == "" {
		// Try to detect from URL
		if strings.HasSuffix(strings.ToLower(imageURL), ".png") {
			contentType = "image/png"
		} else if strings.HasSuffix(strings.ToLower(imageURL), ".webp") {
			contentType = "image/webp"
		} else if strings.HasSuffix(strings.ToLower(imageURL), ".gif") {
			contentType = "image/gif"
		} else {
			contentType = "image/jpeg"
		}
	}

	// Create data URL
	base64Data := base64.StdEncoding.EncodeToString(data)
	dataURL := fmt.Sprintf("data:%s;base64,%s", contentType, base64Data)

	return dataURL, nil
}

// =============================================================================
// Helper functions
// =============================================================================

func getLocalIP() string {
	addrs, err := net.InterfaceAddrs()
	if err != nil {
		return ""
	}

	for _, addr := range addrs {
		if ipnet, ok := addr.(*net.IPNet); ok && !ipnet.IP.IsLoopback() {
			if ipnet.IP.To4() != nil {
				return ipnet.IP.String()
			}
		}
	}
	return ""
}

func hasSSH(ip string) bool {
	conn, err := net.DialTimeout("tcp", fmt.Sprintf("%s:22", ip), 500*time.Millisecond)
	if err != nil {
		return false
	}
	conn.Close()
	return true
}

func getHostname(ip string) string {
	names, err := net.LookupAddr(ip)
	if err != nil || len(names) == 0 {
		return ""
	}
	hostname := strings.TrimSuffix(names[0], ".")
	return hostname
}

func getFilesToUpload(root string) ([]string, error) {
	var files []string
	err := filepath.Walk(root, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if !info.IsDir() {
			files = append(files, path)
		}
		return nil
	})
	return files, err
}
