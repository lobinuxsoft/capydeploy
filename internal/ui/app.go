package ui

import (
	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/widget"
)

// AppState holds the global application state
type AppState struct {
	Devices       []*Device
	SelectedDevice *Device
	Window        fyne.Window
}

var State = &AppState{
	Devices: make([]*Device, 0),
}

// Setup initializes the main UI
func Setup(w fyne.Window) {
	State.Window = w

	// Create tabs for different sections
	tabs := container.NewAppTabs(
		container.NewTabItem("Devices", createDevicesTab()),
		container.NewTabItem("Upload Game", createUploadTab()),
		container.NewTabItem("Shortcuts", createShortcutsTab()),
		container.NewTabItem("Settings", createSettingsTab()),
	)
	tabs.SetTabLocation(container.TabLocationTop)

	w.SetContent(tabs)
}

// createShortcutsTab creates the shortcuts management tab
func createShortcutsTab() fyne.CanvasObject {
	refreshBtn := widget.NewButton("Refresh Shortcuts", func() {
		// TODO: List shortcuts from remote device
	})

	shortcutsList := widget.NewList(
		func() int { return 0 },
		func() fyne.CanvasObject {
			return widget.NewLabel("Shortcut")
		},
		func(id widget.ListItemID, obj fyne.CanvasObject) {},
	)

	return container.NewBorder(
		container.NewHBox(refreshBtn),
		nil, nil, nil,
		shortcutsList,
	)
}

// createSettingsTab creates the settings tab
func createSettingsTab() fyne.CanvasObject {
	// Default paths
	steamPathEntry := widget.NewEntry()
	steamPathEntry.SetPlaceHolder("~/.steam/steam")

	gamePathEntry := widget.NewEntry()
	gamePathEntry.SetPlaceHolder("~/devkit-games")

	form := widget.NewForm(
		widget.NewFormItem("Steam Path", steamPathEntry),
		widget.NewFormItem("Games Path", gamePathEntry),
	)

	saveBtn := widget.NewButton("Save Settings", func() {
		// TODO: Save settings
	})

	return container.NewVBox(
		widget.NewLabel("Settings"),
		widget.NewSeparator(),
		form,
		saveBtn,
	)
}
