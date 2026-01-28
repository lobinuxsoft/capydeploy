package device

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"

	"github.com/pkg/sftp"
	"golang.org/x/crypto/ssh"
)

// Client handles SSH/SFTP connections to a remote device
type Client struct {
	host       string
	port       int
	user       string
	password   string
	keyFile    string
	sshClient  *ssh.Client
	sftpClient *sftp.Client
}

// NewClient creates a new device client
func NewClient(host string, port int, user, password, keyFile string) (*Client, error) {
	return &Client{
		host:     host,
		port:     port,
		user:     user,
		password: password,
		keyFile:  keyFile,
	}, nil
}

// Connect establishes SSH and SFTP connections
func (c *Client) Connect() error {
	config := &ssh.ClientConfig{
		User:            c.user,
		HostKeyCallback: ssh.InsecureIgnoreHostKey(),
	}

	// Try key-based auth first
	if c.keyFile != "" {
		keyPath := expandPath(c.keyFile)
		key, err := os.ReadFile(keyPath)
		if err == nil {
			signer, err := ssh.ParsePrivateKey(key)
			if err == nil {
				config.Auth = append(config.Auth, ssh.PublicKeys(signer))
			}
		}
	}

	// Add password auth
	if c.password != "" {
		config.Auth = append(config.Auth, ssh.Password(c.password))
	}

	// Connect SSH
	addr := fmt.Sprintf("%s:%d", c.host, c.port)
	sshClient, err := ssh.Dial("tcp", addr, config)
	if err != nil {
		return fmt.Errorf("SSH connection failed: %w", err)
	}
	c.sshClient = sshClient

	// Create SFTP client
	sftpClient, err := sftp.NewClient(sshClient)
	if err != nil {
		sshClient.Close()
		return fmt.Errorf("SFTP connection failed: %w", err)
	}
	c.sftpClient = sftpClient

	return nil
}

// Close closes all connections
func (c *Client) Close() {
	if c.sftpClient != nil {
		c.sftpClient.Close()
		c.sftpClient = nil
	}
	if c.sshClient != nil {
		c.sshClient.Close()
		c.sshClient = nil
	}
}

// MkdirAll creates a directory and all parent directories on the remote host
func (c *Client) MkdirAll(remotePath string) error {
	// Normalize path separators for Unix
	remotePath = strings.ReplaceAll(remotePath, "\\", "/")
	return c.sftpClient.MkdirAll(remotePath)
}

// UploadFile uploads a single file to the remote host
func (c *Client) UploadFile(localPath, remotePath string) error {
	// Normalize remote path for Unix
	remotePath = strings.ReplaceAll(remotePath, "\\", "/")

	// Open local file
	localFile, err := os.Open(localPath)
	if err != nil {
		return fmt.Errorf("failed to open local file: %w", err)
	}
	defer localFile.Close()

	// Get file info for permissions
	localInfo, err := localFile.Stat()
	if err != nil {
		return fmt.Errorf("failed to stat local file: %w", err)
	}

	// Create remote file
	remoteFile, err := c.sftpClient.Create(remotePath)
	if err != nil {
		return fmt.Errorf("failed to create remote file: %w", err)
	}
	defer remoteFile.Close()

	// Copy contents
	_, err = io.Copy(remoteFile, localFile)
	if err != nil {
		return fmt.Errorf("failed to copy file: %w", err)
	}

	// Set permissions (preserve executable bit)
	mode := localInfo.Mode()
	if err := c.sftpClient.Chmod(remotePath, mode); err != nil {
		// Non-fatal, just log
		fmt.Printf("Warning: failed to set permissions on %s: %v\n", remotePath, err)
	}

	return nil
}

// DownloadFile downloads a file from the remote host
func (c *Client) DownloadFile(remotePath, localPath string) error {
	// Normalize remote path for Unix
	remotePath = strings.ReplaceAll(remotePath, "\\", "/")

	// Open remote file
	remoteFile, err := c.sftpClient.Open(remotePath)
	if err != nil {
		return fmt.Errorf("failed to open remote file: %w", err)
	}
	defer remoteFile.Close()

	// Create local directory if needed
	localDir := filepath.Dir(localPath)
	if err := os.MkdirAll(localDir, 0755); err != nil {
		return fmt.Errorf("failed to create local directory: %w", err)
	}

	// Create local file
	localFile, err := os.Create(localPath)
	if err != nil {
		return fmt.Errorf("failed to create local file: %w", err)
	}
	defer localFile.Close()

	// Copy contents
	_, err = io.Copy(localFile, remoteFile)
	if err != nil {
		return fmt.Errorf("failed to copy file: %w", err)
	}

	return nil
}

// RunCommand executes a command on the remote host
func (c *Client) RunCommand(cmd string) (string, error) {
	session, err := c.sshClient.NewSession()
	if err != nil {
		return "", fmt.Errorf("failed to create session: %w", err)
	}
	defer session.Close()

	output, err := session.CombinedOutput(cmd)
	if err != nil {
		return string(output), fmt.Errorf("command failed: %w\nOutput: %s", err, output)
	}

	return string(output), nil
}

// FileExists checks if a file exists on the remote host
func (c *Client) FileExists(remotePath string) bool {
	remotePath = strings.ReplaceAll(remotePath, "\\", "/")
	_, err := c.sftpClient.Stat(remotePath)
	return err == nil
}

// GetHomeDir returns the home directory on the remote host
func (c *Client) GetHomeDir() (string, error) {
	output, err := c.RunCommand("echo $HOME")
	if err != nil {
		return "", err
	}
	return strings.TrimSpace(output), nil
}

// expandPath expands ~ to home directory
func expandPath(path string) string {
	if len(path) > 0 && path[0] == '~' {
		home, _ := os.UserHomeDir()
		return filepath.Join(home, path[1:])
	}
	return path
}
