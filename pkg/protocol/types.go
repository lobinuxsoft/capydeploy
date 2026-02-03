// Package protocol defines shared types and messages for Hub-Agent communication.
package protocol

import "time"

// Capability represents a feature supported by an agent.
// Capabilities are used by the Hub to dynamically show/hide UI sections
// based on what the connected Agent supports.
type Capability string

// Available capabilities that an agent can report.
//
// File capabilities:
//   - CapFileUpload: Agent can receive game file uploads from the Hub
//   - CapFileList: Agent can list installed games/files
//
// Steam capabilities (require Steam client on agent machine):
//   - CapSteamShortcuts: Agent can create/delete Steam non-Steam game shortcuts
//   - CapSteamArtwork: Agent can apply custom artwork to shortcuts
//   - CapSteamUsers: Agent can list local Steam users
//   - CapSteamRestart: Agent can restart the Steam client
//
// Platform support:
//   - PC (Windows/Linux/macOS) with Steam: all capabilities
//   - PC without Steam: file_upload, file_list only
//   - Android (future): file_upload, file_list only
const (
	CapFileUpload     Capability = "file_upload"
	CapFileList       Capability = "file_list"
	CapSteamShortcuts Capability = "steam_shortcuts"
	CapSteamArtwork   Capability = "steam_artwork"
	CapSteamUsers     Capability = "steam_users"
	CapSteamRestart   Capability = "steam_restart"
)

// AgentInfo contains information about a discovered agent.
type AgentInfo struct {
	ID                    string       `json:"id"`
	Name                  string       `json:"name"`
	Platform              string       `json:"platform"`
	Version               string       `json:"version"`
	AcceptConnections     bool         `json:"acceptConnections"`
	SupportedImageFormats []string     `json:"supportedImageFormats"`
	Capabilities          []Capability `json:"capabilities"`
}

// UploadConfig defines the configuration for uploading a game.
type UploadConfig struct {
	GameName      string `json:"gameName"`
	InstallPath   string `json:"installPath"`
	Executable    string `json:"executable"`
	LaunchOptions string `json:"launchOptions,omitempty"`
	Tags          string `json:"tags,omitempty"`
}

// ShortcutConfig defines the configuration for creating a Steam shortcut.
type ShortcutConfig struct {
	Name          string         `json:"name"`
	Exe           string         `json:"exe"`
	StartDir      string         `json:"startDir"`
	LaunchOptions string         `json:"launchOptions,omitempty"`
	Tags          []string       `json:"tags,omitempty"`
	Artwork       *ArtworkConfig `json:"artwork,omitempty"`
}

// ArtworkConfig defines artwork paths for a shortcut.
type ArtworkConfig struct {
	Grid   string `json:"grid,omitempty"`   // 600x900 portrait
	Hero   string `json:"hero,omitempty"`   // 1920x620 header
	Logo   string `json:"logo,omitempty"`   // transparent logo
	Icon   string `json:"icon,omitempty"`   // square icon
	Banner string `json:"banner,omitempty"` // 460x215 horizontal
}

// ShortcutInfo contains information about an existing shortcut.
type ShortcutInfo struct {
	AppID         uint32   `json:"appId"`
	Name          string   `json:"name"`
	Exe           string   `json:"exe"`
	StartDir      string   `json:"startDir"`
	LaunchOptions string   `json:"launchOptions,omitempty"`
	Tags          []string `json:"tags,omitempty"`
	LastPlayed    int64    `json:"lastPlayed,omitempty"`
}

// UploadStatus represents the current state of an upload.
type UploadStatus string

const (
	UploadStatusPending    UploadStatus = "pending"
	UploadStatusInProgress UploadStatus = "in_progress"
	UploadStatusCompleted  UploadStatus = "completed"
	UploadStatusFailed     UploadStatus = "failed"
	UploadStatusCancelled  UploadStatus = "cancelled"
)

// UploadProgress contains progress information for an active upload.
type UploadProgress struct {
	UploadID       string       `json:"uploadId"`
	Status         UploadStatus `json:"status"`
	TotalBytes     int64        `json:"totalBytes"`
	TransferredBytes int64      `json:"transferredBytes"`
	CurrentFile    string       `json:"currentFile,omitempty"`
	StartedAt      time.Time    `json:"startedAt"`
	UpdatedAt      time.Time    `json:"updatedAt"`
	Error          string       `json:"error,omitempty"`
}

// Percentage returns the upload progress as a percentage (0-100).
func (p *UploadProgress) Percentage() float64 {
	if p.TotalBytes == 0 {
		return 0
	}
	return float64(p.TransferredBytes) / float64(p.TotalBytes) * 100
}
