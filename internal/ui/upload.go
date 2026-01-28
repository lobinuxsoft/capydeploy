package ui

import (
	"fmt"
	"os"
	"path/filepath"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/widget"
)

var (
	selectedGamePath string
	gameNameEntry    *widget.Entry
	gameExeEntry     *widget.Entry
	launchOptsEntry  *widget.Entry
	progressBar      *widget.ProgressBar
	statusLabel      *widget.Label
)

// createUploadTab creates the game upload tab
func createUploadTab() fyne.CanvasObject {
	// Game name
	gameNameEntry = widget.NewEntry()
	gameNameEntry.SetPlaceHolder("My Game")

	// Local game folder
	localPathLabel := widget.NewLabel("No folder selected")
	selectFolderBtn := widget.NewButton("Select Game Folder", func() {
		dialog.ShowFolderOpen(func(uri fyne.ListableURI, err error) {
			if err != nil || uri == nil {
				return
			}
			selectedGamePath = uri.Path()
			localPathLabel.SetText(selectedGamePath)

			// Auto-fill game name from folder
			if gameNameEntry.Text == "" {
				gameNameEntry.SetText(filepath.Base(selectedGamePath))
			}
		}, State.Window)
	})

	// Executable path (relative to game folder)
	gameExeEntry = widget.NewEntry()
	gameExeEntry.SetPlaceHolder("game.exe or game.sh")

	// Launch options
	launchOptsEntry = widget.NewEntry()
	launchOptsEntry.SetPlaceHolder("Optional launch arguments")

	// Tags
	tagsEntry := widget.NewEntry()
	tagsEntry.SetPlaceHolder("tag1, tag2 (optional)")

	// Remote destination
	remotePathEntry := widget.NewEntry()
	remotePathEntry.SetText("~/devkit-games")

	// Form
	form := widget.NewForm(
		widget.NewFormItem("Game Name", gameNameEntry),
		widget.NewFormItem("Local Folder", container.NewHBox(localPathLabel, selectFolderBtn)),
		widget.NewFormItem("Executable", gameExeEntry),
		widget.NewFormItem("Launch Options", launchOptsEntry),
		widget.NewFormItem("Tags", tagsEntry),
		widget.NewFormItem("Remote Path", remotePathEntry),
	)

	// Progress
	progressBar = widget.NewProgressBar()
	progressBar.Hide()

	statusLabel = widget.NewLabel("")

	// Upload button
	uploadBtn := widget.NewButton("Upload & Create Shortcut", func() {
		if State.SelectedDevice == nil || !State.SelectedDevice.Connected {
			dialog.ShowError(fmt.Errorf("no device connected"), State.Window)
			return
		}
		if selectedGamePath == "" {
			dialog.ShowError(fmt.Errorf("no game folder selected"), State.Window)
			return
		}
		if gameNameEntry.Text == "" {
			dialog.ShowError(fmt.Errorf("game name is required"), State.Window)
			return
		}
		if gameExeEntry.Text == "" {
			dialog.ShowError(fmt.Errorf("executable path is required"), State.Window)
			return
		}

		go uploadGame(
			selectedGamePath,
			gameNameEntry.Text,
			gameExeEntry.Text,
			launchOptsEntry.Text,
			tagsEntry.Text,
			remotePathEntry.Text,
		)
	})

	return container.NewVBox(
		widget.NewLabel("Upload Game to Device"),
		widget.NewSeparator(),
		form,
		widget.NewSeparator(),
		uploadBtn,
		progressBar,
		statusLabel,
	)
}

// uploadGame uploads a game to the remote device and creates a shortcut
func uploadGame(localPath, gameName, exe, launchOpts, tags, remotePath string) {
	dev := State.SelectedDevice

	progressBar.Show()
	progressBar.SetValue(0)
	statusLabel.SetText("Preparing upload...")

	// Expand remote path
	remotePath = expandPath(remotePath)
	remoteGamePath := filepath.Join(remotePath, gameName)

	// Create remote directory
	statusLabel.SetText("Creating remote directory...")
	if err := dev.Client.MkdirAll(remoteGamePath); err != nil {
		showUploadError(err)
		return
	}

	// Get list of files to upload
	statusLabel.SetText("Scanning files...")
	files, err := getFilesToUpload(localPath)
	if err != nil {
		showUploadError(err)
		return
	}

	// Upload files
	totalFiles := len(files)
	for i, file := range files {
		relPath, _ := filepath.Rel(localPath, file)
		remoteDest := filepath.Join(remoteGamePath, relPath)

		// Ensure parent directory exists
		remoteDir := filepath.Dir(remoteDest)
		dev.Client.MkdirAll(remoteDir)

		statusLabel.SetText(fmt.Sprintf("Uploading: %s", relPath))
		progressBar.SetValue(float64(i) / float64(totalFiles))

		if err := dev.Client.UploadFile(file, remoteDest); err != nil {
			showUploadError(fmt.Errorf("failed to upload %s: %w", relPath, err))
			return
		}
	}

	progressBar.SetValue(0.9)
	statusLabel.SetText("Creating Steam shortcut...")

	// Create shortcut using steam-shortcut-manager
	exePath := filepath.Join(remoteGamePath, exe)
	if err := createShortcut(dev, gameName, exePath, remoteGamePath, launchOpts, tags); err != nil {
		showUploadError(err)
		return
	}

	progressBar.SetValue(1.0)
	statusLabel.SetText("Upload complete!")
	progressBar.Hide()

	dialog.ShowInformation("Success",
		fmt.Sprintf("Game '%s' uploaded and shortcut created!", gameName),
		State.Window)
}

// getFilesToUpload recursively gets all files in a directory
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

// createShortcut creates a Steam shortcut on the remote device
func createShortcut(dev *Device, name, exe, startDir, launchOpts, tags string) error {
	// Build the command to run steam-shortcut-manager on the remote device
	cmd := fmt.Sprintf("steam-shortcut-manager add %q %q --start-dir %q",
		name, exe, startDir)

	if launchOpts != "" {
		cmd += fmt.Sprintf(" --launch-options %q", launchOpts)
	}
	if tags != "" {
		cmd += fmt.Sprintf(" --tags %q", tags)
	}

	_, err := dev.Client.RunCommand(cmd)
	return err
}

// expandPath expands ~ to home directory
func expandPath(path string) string {
	if len(path) > 0 && path[0] == '~' {
		home, _ := os.UserHomeDir()
		return filepath.Join(home, path[1:])
	}
	return path
}

// showUploadError shows an error dialog and resets the progress
func showUploadError(err error) {
	progressBar.Hide()
	statusLabel.SetText(fmt.Sprintf("Error: %v", err))
	dialog.ShowError(err, State.Window)
}
