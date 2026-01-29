package ui

import (
	"fmt"
	"os"
	"path"
	"path/filepath"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"

	"github.com/lobinuxsoft/bazzite-devkit/internal/config"
	"github.com/lobinuxsoft/bazzite-devkit/internal/shortcuts"
)

// GameSetup represents a saved game installation setup
type GameSetup struct {
	ID            string
	Name          string
	LocalPath     string
	Executable    string
	LaunchOptions string
	Tags          string
	RemotePath    string
}

var (
	gameSetups      []*GameSetup
	setupListWidget *widget.List
	selectedSetup   *GameSetup
	progressBar     *widget.ProgressBar
	statusLabel     *widget.Label
)

func init() {
	loadSavedGameSetups()
}

// loadSavedGameSetups loads game setups from config
func loadSavedGameSetups() {
	saved, err := config.GetGameSetups()
	if err != nil {
		return
	}

	gameSetups = make([]*GameSetup, len(saved))
	for i, s := range saved {
		gameSetups[i] = &GameSetup{
			ID:            s.ID,
			Name:          s.Name,
			LocalPath:     s.LocalPath,
			Executable:    s.Executable,
			LaunchOptions: s.LaunchOptions,
			Tags:          s.Tags,
			RemotePath:    s.RemotePath,
		}
	}
}

// setupRowData stores widget references for a game setup list row
type setupRowData struct {
	nameLabel *widget.Label
	pathLabel *widget.Label
	uploadBtn *widget.Button
	editBtn   *widget.Button
	deleteBtn *widget.Button
}

// Map to store row data by container pointer
var setupRowCache = make(map[fyne.CanvasObject]*setupRowData)

// createUploadTab creates the game upload tab
func createUploadTab() fyne.CanvasObject {
	// Setup list with inline buttons
	setupListWidget = widget.NewList(
		func() int { return len(gameSetups) },
		func() fyne.CanvasObject {
			nameLabel := widget.NewLabel("Game Name")
			pathLabel := widget.NewLabel("Path")
			pathLabel.TextStyle = fyne.TextStyle{Italic: true}

			uploadBtn := widget.NewButtonWithIcon("", theme.UploadIcon(), nil)
			editBtn := widget.NewButtonWithIcon("", theme.DocumentCreateIcon(), nil)
			deleteBtn := widget.NewButtonWithIcon("", theme.DeleteIcon(), nil)

			buttons := container.NewHBox(uploadBtn, editBtn, deleteBtn)

			c := container.NewBorder(
				nil, nil,
				widget.NewIcon(theme.FolderIcon()),
				buttons,
				container.NewVBox(nameLabel, pathLabel),
			)

			// Store widget references
			setupRowCache[c] = &setupRowData{
				nameLabel: nameLabel,
				pathLabel: pathLabel,
				uploadBtn: uploadBtn,
				editBtn:   editBtn,
				deleteBtn: deleteBtn,
			}

			return c
		},
		func(id widget.ListItemID, obj fyne.CanvasObject) {
			if id >= len(gameSetups) {
				return
			}
			setup := gameSetups[id]

			// Get cached widget references
			row, ok := setupRowCache[obj]
			if !ok {
				return
			}

			row.nameLabel.SetText(setup.Name)
			row.pathLabel.SetText(truncatePath(setup.LocalPath, 50))

			row.uploadBtn.OnTapped = func() {
				if State.SelectedDevice == nil || !State.SelectedDevice.Connected {
					dialog.ShowError(fmt.Errorf("no device connected"), State.Window)
					return
				}
				go uploadGame(setup)
			}

			row.editBtn.OnTapped = func() {
				showGameSetupForm(setup)
			}

			row.deleteBtn.OnTapped = func() {
				dialog.ShowConfirm("Delete Setup",
					fmt.Sprintf("Delete setup for '%s'?", setup.Name),
					func(ok bool) {
						if ok {
							removeGameSetup(setup)
						}
					}, State.Window)
			}
		},
	)

	setupListWidget.OnSelected = func(id widget.ListItemID) {
		if id < len(gameSetups) {
			selectedSetup = gameSetups[id]
		}
	}

	// Top buttons
	addBtn := widget.NewButtonWithIcon("New Game Setup", theme.ContentAddIcon(), func() {
		showGameSetupForm(nil)
	})

	// Progress section
	progressBar = widget.NewProgressBar()
	progressBar.Hide()
	statusLabel = widget.NewLabel("")

	topBar := container.NewVBox(
		container.NewHBox(addBtn),
		widget.NewSeparator(),
		widget.NewLabel("Saved Game Setups (click upload icon to install):"),
	)

	bottomBar := container.NewVBox(
		widget.NewSeparator(),
		progressBar,
		statusLabel,
	)

	return container.NewBorder(
		topBar,
		bottomBar,
		nil, nil,
		setupListWidget,
	)
}

// showGameSetupForm shows a form to add or edit a game setup
func showGameSetupForm(existingSetup *GameSetup) {
	isEdit := existingSetup != nil
	title := "New Game Setup"
	if isEdit {
		title = "Edit Game Setup"
	}

	formWindow := fyne.CurrentApp().NewWindow(title)
	formWindow.Resize(fyne.NewSize(550, 450))

	nameEntry := widget.NewEntry()
	localPathLabel := widget.NewLabel("No folder selected")
	exeEntry := widget.NewEntry()
	launchOptsEntry := widget.NewEntry()
	tagsEntry := widget.NewEntry()
	remotePathEntry := widget.NewEntry()

	var localPath string

	if isEdit {
		nameEntry.SetText(existingSetup.Name)
		localPath = existingSetup.LocalPath
		localPathLabel.SetText(truncatePath(localPath, 40))
		exeEntry.SetText(existingSetup.Executable)
		launchOptsEntry.SetText(existingSetup.LaunchOptions)
		tagsEntry.SetText(existingSetup.Tags)
		remotePathEntry.SetText(existingSetup.RemotePath)
	} else {
		nameEntry.SetPlaceHolder("My Game")
		exeEntry.SetPlaceHolder("game.x86_64 or game.sh")
		launchOptsEntry.SetPlaceHolder("Optional launch arguments")
		tagsEntry.SetPlaceHolder("tag1, tag2 (optional)")
		remotePathEntry.SetText("~/devkit-games")
	}

	selectFolderBtn := widget.NewButtonWithIcon("Browse", theme.FolderOpenIcon(), func() {
		dialog.ShowFolderOpen(func(uri fyne.ListableURI, err error) {
			if err != nil || uri == nil {
				return
			}
			localPath = uri.Path()
			localPathLabel.SetText(truncatePath(localPath, 40))

			if nameEntry.Text == "" {
				nameEntry.SetText(filepath.Base(localPath))
			}
		}, State.Window)
	})

	form := widget.NewForm(
		widget.NewFormItem("Game Name", nameEntry),
		widget.NewFormItem("Local Folder", container.NewBorder(nil, nil, nil, selectFolderBtn, localPathLabel)),
		widget.NewFormItem("Executable", exeEntry),
		widget.NewFormItem("Launch Options", launchOptsEntry),
		widget.NewFormItem("Tags", tagsEntry),
		widget.NewFormItem("Remote Path", remotePathEntry),
	)

	saveBtn := widget.NewButtonWithIcon("Save Setup", theme.ConfirmIcon(), func() {
		if nameEntry.Text == "" {
			dialog.ShowError(fmt.Errorf("game name is required"), formWindow)
			return
		}
		if localPath == "" {
			dialog.ShowError(fmt.Errorf("local folder is required"), formWindow)
			return
		}
		if exeEntry.Text == "" {
			dialog.ShowError(fmt.Errorf("executable is required"), formWindow)
			return
		}

		if isEdit {
			existingSetup.Name = nameEntry.Text
			existingSetup.LocalPath = localPath
			existingSetup.Executable = exeEntry.Text
			existingSetup.LaunchOptions = launchOptsEntry.Text
			existingSetup.Tags = tagsEntry.Text
			existingSetup.RemotePath = remotePathEntry.Text
			updateGameSetup(existingSetup)
		} else {
			setup := &GameSetup{
				Name:          nameEntry.Text,
				LocalPath:     localPath,
				Executable:    exeEntry.Text,
				LaunchOptions: launchOptsEntry.Text,
				Tags:          tagsEntry.Text,
				RemotePath:    remotePathEntry.Text,
			}
			addGameSetup(setup)
		}

		setupListWidget.Refresh()
		formWindow.Close()
	})

	cancelBtn := widget.NewButtonWithIcon("Cancel", theme.CancelIcon(), func() {
		formWindow.Close()
	})

	buttons := container.NewHBox(cancelBtn, saveBtn)

	content := container.NewVBox(
		widget.NewLabelWithStyle("Game Installation Setup", fyne.TextAlignCenter, fyne.TextStyle{Bold: true}),
		widget.NewSeparator(),
		form,
		widget.NewSeparator(),
		container.NewCenter(buttons),
	)

	formWindow.SetContent(container.NewPadded(content))
	formWindow.Show()
}

// addGameSetup adds a new game setup
func addGameSetup(setup *GameSetup) {
	gameSetups = append(gameSetups, setup)
	config.AddGameSetup(config.GameSetup{
		Name:          setup.Name,
		LocalPath:     setup.LocalPath,
		Executable:    setup.Executable,
		LaunchOptions: setup.LaunchOptions,
		Tags:          setup.Tags,
		RemotePath:    setup.RemotePath,
	})
}

// updateGameSetup updates an existing game setup
func updateGameSetup(setup *GameSetup) {
	config.UpdateGameSetup(setup.ID, config.GameSetup{
		ID:            setup.ID,
		Name:          setup.Name,
		LocalPath:     setup.LocalPath,
		Executable:    setup.Executable,
		LaunchOptions: setup.LaunchOptions,
		Tags:          setup.Tags,
		RemotePath:    setup.RemotePath,
	})
}

// removeGameSetup removes a game setup
func removeGameSetup(setup *GameSetup) {
	config.RemoveGameSetup(setup.ID)
	for i, s := range gameSetups {
		if s == setup {
			gameSetups = append(gameSetups[:i], gameSetups[i+1:]...)
			break
		}
	}
	selectedSetup = nil
	setupListWidget.Refresh()
}

// truncatePath truncates a path for display
func truncatePath(p string, maxLen int) string {
	if len(p) <= maxLen {
		return p
	}
	return "..." + p[len(p)-maxLen+3:]
}

// uploadGame uploads a game to the remote device and creates a shortcut
func uploadGame(setup *GameSetup) {
	dev := State.SelectedDevice

	progressBar.Show()
	progressBar.SetValue(0)
	statusLabel.SetText("Preparing upload...")

	// Expand remote path (~ to remote home directory)
	remotePath, err := expandRemotePath(dev, setup.RemotePath)
	if err != nil {
		showUploadError(fmt.Errorf("failed to expand remote path: %w", err))
		return
	}

	// Use path.Join for Linux-style paths (forward slashes)
	remoteGamePath := path.Join(remotePath, setup.Name)

	// Create remote directory
	statusLabel.SetText("Creating remote directory...")
	if err := dev.Client.MkdirAll(remoteGamePath); err != nil {
		showUploadError(err)
		return
	}

	// Get list of files to upload
	statusLabel.SetText("Scanning files...")
	files, err := getFilesToUpload(setup.LocalPath)
	if err != nil {
		showUploadError(err)
		return
	}

	// Upload files
	totalFiles := len(files)
	for i, file := range files {
		// Get relative path from local folder
		relPath, _ := filepath.Rel(setup.LocalPath, file)
		// Convert Windows backslashes to forward slashes for Linux
		relPath = strings.ReplaceAll(relPath, "\\", "/")
		// Build remote destination path
		remoteDest := path.Join(remoteGamePath, relPath)

		// Ensure parent directory exists
		remoteDir := path.Dir(remoteDest)
		dev.Client.MkdirAll(remoteDir)

		statusLabel.SetText(fmt.Sprintf("Uploading: %s", relPath))
		progressBar.SetValue(float64(i) / float64(totalFiles))

		if err := dev.Client.UploadFile(file, remoteDest); err != nil {
			showUploadError(fmt.Errorf("failed to upload %s: %w", relPath, err))
			return
		}
	}

	progressBar.SetValue(0.85)
	statusLabel.SetText("Setting executable permissions...")

	// Create shortcut using steam-shortcut-manager (use Linux paths)
	exePath := path.Join(remoteGamePath, setup.Executable)

	// Set executable permissions on the main executable
	chmodCmd := fmt.Sprintf("chmod +x %q", exePath)
	if _, err := dev.Client.RunCommand(chmodCmd); err != nil {
		showUploadError(fmt.Errorf("failed to set executable permissions: %w", err))
		return
	}

	// Also set executable permissions on all .sh files and common executable extensions
	chmodAllCmd := fmt.Sprintf("find %q -type f \\( -name '*.sh' -o -name '*.x86_64' -o -name '*.x86' \\) -exec chmod +x {} \\;", remoteGamePath)
	dev.Client.RunCommand(chmodAllCmd) // Ignore errors, this is optional

	progressBar.SetValue(0.9)
	statusLabel.SetText("Creating Steam shortcut...")

	if err := createShortcut(dev, setup.Name, exePath, remoteGamePath, setup.LaunchOptions, setup.Tags); err != nil {
		showUploadError(err)
		return
	}

	progressBar.SetValue(1.0)
	statusLabel.SetText("Upload complete!")
	progressBar.Hide()

	dialog.ShowInformation("Success",
		fmt.Sprintf("Game '%s' uploaded and shortcut created!", setup.Name),
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
	cfg := &shortcuts.RemoteConfig{
		Host:     dev.Host,
		Port:     dev.Port,
		User:     dev.User,
		Password: dev.Password,
		KeyFile:  dev.KeyFile,
	}

	tagsList := shortcuts.ParseTags(tags)

	if err := shortcuts.AddShortcut(cfg, name, exe, startDir, launchOpts, tagsList); err != nil {
		return err
	}

	// Refresh Steam library so the shortcut appears without restarting Steam
	shortcuts.RefreshSteamLibrary(cfg)

	return nil
}

// expandRemotePath expands ~ to the remote home directory
func expandRemotePath(dev *Device, remotePath string) (string, error) {
	if strings.HasPrefix(remotePath, "~") {
		homeDir, err := dev.Client.GetHomeDir()
		if err != nil {
			return "", err
		}
		remotePath = strings.Replace(remotePath, "~", homeDir, 1)
	}
	return remotePath, nil
}

// showUploadError shows an error dialog and resets the progress
func showUploadError(err error) {
	progressBar.Hide()
	statusLabel.SetText(fmt.Sprintf("Error: %v", err))
	dialog.ShowError(err, State.Window)
}
