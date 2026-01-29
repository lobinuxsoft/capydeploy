package ui

import (
	"fmt"
	"path"
	"strings"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"

	"github.com/lobinuxsoft/bazzite-devkit/internal/shortcuts"
)

// InstalledGame represents a game installed on the remote device
type InstalledGame struct {
	Name string
	Path string
	Size string
}

var (
	installedGames    []InstalledGame
	gamesListWidget   *widget.List
	selectedGame      *InstalledGame
	gamesRemotePath   string
	gamesStatusLabel  *widget.Label
)

// createGamesTab creates the installed games management tab
func createGamesTab() fyne.CanvasObject {
	gamesRemotePath = "~/devkit-games"

	remotePathEntry := widget.NewEntry()
	remotePathEntry.SetText(gamesRemotePath)
	remotePathEntry.OnChanged = func(s string) {
		gamesRemotePath = s
	}

	gamesStatusLabel = widget.NewLabel("Connect to a device and click Refresh")

	gamesListWidget = widget.NewList(
		func() int { return len(installedGames) },
		func() fyne.CanvasObject {
			return container.NewBorder(
				nil, nil,
				widget.NewIcon(theme.FolderIcon()),
				widget.NewLabel("Size"),
				widget.NewLabel("Game Name"),
			)
		},
		func(id widget.ListItemID, obj fyne.CanvasObject) {
			if id >= len(installedGames) {
				return
			}
			game := installedGames[id]
			box := obj.(*fyne.Container)
			nameLabel := box.Objects[0].(*widget.Label)
			sizeLabel := box.Objects[2].(*widget.Label)
			nameLabel.SetText(game.Name)
			sizeLabel.SetText(game.Size)
		},
	)

	gamesListWidget.OnSelected = func(id widget.ListItemID) {
		if id < len(installedGames) {
			selectedGame = &installedGames[id]
		}
	}

	refreshBtn := widget.NewButtonWithIcon("Refresh", theme.ViewRefreshIcon(), func() {
		if State.SelectedDevice == nil || !State.SelectedDevice.Connected {
			dialog.ShowError(fmt.Errorf("no device connected"), State.Window)
			return
		}
		go refreshInstalledGames()
	})

	deleteBtn := widget.NewButtonWithIcon("Delete Game", theme.DeleteIcon(), func() {
		if selectedGame == nil {
			dialog.ShowError(fmt.Errorf("no game selected"), State.Window)
			return
		}
		if State.SelectedDevice == nil || !State.SelectedDevice.Connected {
			dialog.ShowError(fmt.Errorf("no device connected"), State.Window)
			return
		}

		dialog.ShowConfirm("Delete Game",
			fmt.Sprintf("Are you sure you want to delete '%s'?\nThis will also remove the Steam shortcut.", selectedGame.Name),
			func(ok bool) {
				if ok {
					go deleteGame(selectedGame)
				}
			}, State.Window)
	})

	topBar := container.NewVBox(
		container.NewBorder(nil, nil, widget.NewLabel("Games Path:"), nil, remotePathEntry),
		container.NewHBox(refreshBtn, deleteBtn),
		gamesStatusLabel,
	)

	return container.NewBorder(
		topBar,
		nil, nil, nil,
		gamesListWidget,
	)
}

// refreshInstalledGames fetches the list of installed games from the remote device
func refreshInstalledGames() {
	dev := State.SelectedDevice
	gamesStatusLabel.SetText("Fetching games...")

	// Expand the remote path
	remotePath := gamesRemotePath
	if strings.HasPrefix(remotePath, "~") {
		homeDir, err := dev.Client.GetHomeDir()
		if err != nil {
			gamesStatusLabel.SetText(fmt.Sprintf("Error: %v", err))
			return
		}
		remotePath = strings.Replace(remotePath, "~", homeDir, 1)
	}

	// List directories in the games folder
	cmd := fmt.Sprintf("ls -1 %s 2>/dev/null || echo ''", remotePath)
	output, err := dev.Client.RunCommand(cmd)
	if err != nil {
		gamesStatusLabel.SetText("No games found or path doesn't exist")
		installedGames = []InstalledGame{}
		gamesListWidget.Refresh()
		return
	}

	// Parse the output
	lines := strings.Split(strings.TrimSpace(output), "\n")
	installedGames = []InstalledGame{}

	for _, line := range lines {
		name := strings.TrimSpace(line)
		if name == "" {
			continue
		}

		gamePath := path.Join(remotePath, name)

		// Get folder size
		sizeCmd := fmt.Sprintf("du -sh %q 2>/dev/null | cut -f1", gamePath)
		sizeOutput, _ := dev.Client.RunCommand(sizeCmd)
		size := strings.TrimSpace(sizeOutput)
		if size == "" {
			size = "Unknown"
		}

		installedGames = append(installedGames, InstalledGame{
			Name: name,
			Path: gamePath,
			Size: size,
		})
	}

	gamesStatusLabel.SetText(fmt.Sprintf("Found %d games", len(installedGames)))
	gamesListWidget.Refresh()
}

// deleteGame deletes a game from the remote device and removes its shortcut
func deleteGame(game *InstalledGame) {
	dev := State.SelectedDevice
	gamesStatusLabel.SetText(fmt.Sprintf("Deleting %s...", game.Name))

	var shortcutErr error

	// First, remove the Steam shortcut using local steam-shortcut-manager
	if err := removeShortcut(dev, game.Name); err != nil {
		shortcutErr = err
		gamesStatusLabel.SetText(fmt.Sprintf("Warning: %v", err))
	}

	// Delete the game folder
	cmd := fmt.Sprintf("rm -rf %q", game.Path)
	_, err := dev.Client.RunCommand(cmd)
	if err != nil {
		dialog.ShowError(fmt.Errorf("failed to delete game files: %w", err), State.Window)
		gamesStatusLabel.SetText("Error deleting game")
		return
	}

	selectedGame = nil

	// Refresh the list
	refreshInstalledGames()

	if shortcutErr != nil {
		dialog.ShowInformation("Partial Success",
			fmt.Sprintf("Game files deleted but shortcut removal failed:\n%v", shortcutErr),
			State.Window)
	} else {
		gamesStatusLabel.SetText(fmt.Sprintf("Deleted %s", game.Name))
		dialog.ShowInformation("Success", fmt.Sprintf("Game '%s' deleted successfully", game.Name), State.Window)
	}
}

// removeShortcut removes a Steam shortcut from the remote device
func removeShortcut(dev *Device, gameName string) error {
	cfg := &shortcuts.RemoteConfig{
		Host:     dev.Host,
		Port:     dev.Port,
		User:     dev.User,
		Password: dev.Password,
		KeyFile:  dev.KeyFile,
	}

	if err := shortcuts.RemoveShortcut(cfg, gameName); err != nil {
		return err
	}

	// Refresh Steam library so the shortcut disappears without restarting Steam
	shortcuts.RefreshSteamLibrary(cfg)

	return nil
}
