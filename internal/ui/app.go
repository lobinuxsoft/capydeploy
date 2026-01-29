package ui

import (
	"fmt"
	"image/color"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/layout"
	"fyne.io/fyne/v2/widget"
)

// AppState holds the global application state
type AppState struct {
	Devices        []*Device
	SelectedDevice *Device
	Window         fyne.Window
}

var State = &AppState{
	Devices: make([]*Device, 0),
}

// Connection status widgets
var (
	connectionStatusLabel *widget.Label
	connectionDot         *canvas.Circle
)

// Setup initializes the main UI
func Setup(w fyne.Window) {
	State.Window = w
	State.Devices = devices // Load saved devices

	// Create connection status indicator (top right)
	connectionDot = canvas.NewCircle(color.RGBA{128, 128, 128, 255}) // Gray when disconnected
	connectionDot.StrokeWidth = 1
	connectionDot.StrokeColor = color.RGBA{40, 40, 40, 255}

	// Use a min size rect to ensure the dot has proper size
	dotSpacer := canvas.NewRectangle(color.Transparent)
	dotSpacer.SetMinSize(fyne.NewSize(14, 14))
	dotContainer := container.NewStack(dotSpacer, connectionDot)

	connectionStatusLabel = widget.NewLabel("Not connected")
	connectionStatusLabel.TextStyle = fyne.TextStyle{Italic: true}

	statusIndicator := container.NewHBox(
		dotContainer,
		connectionStatusLabel,
	)

	// Create tabs for different sections
	tabs := container.NewAppTabs(
		container.NewTabItem("Devices", createDevicesTab()),
		container.NewTabItem("Upload Game", createUploadTab()),
		container.NewTabItem("Installed Games", createGamesTab()),
		container.NewTabItem("Settings", createSettingsTab()),
	)
	tabs.SetTabLocation(container.TabLocationTop)

	// Top bar with tabs on left and status on right
	topBar := container.NewBorder(
		nil, nil,
		nil,
		container.NewHBox(layout.NewSpacer(), statusIndicator),
		nil,
	)

	// Main layout
	mainContent := container.NewBorder(
		topBar,
		nil,
		nil, nil,
		tabs,
	)

	w.SetContent(mainContent)
}

// UpdateConnectionStatus updates the connection status indicator
func UpdateConnectionStatus() {
	if connectionStatusLabel == nil || connectionDot == nil {
		return
	}

	if State.SelectedDevice != nil && State.SelectedDevice.Connected {
		dev := State.SelectedDevice
		connectionStatusLabel.SetText(fmt.Sprintf("%s (%s:%d)", dev.Name, dev.Host, dev.Port))
		connectionDot.FillColor = color.RGBA{0, 200, 0, 255} // Green when connected
		connectionDot.Refresh()
	} else {
		connectionStatusLabel.SetText("Not connected")
		connectionDot.FillColor = color.RGBA{128, 128, 128, 255} // Gray when disconnected
		connectionDot.Refresh()
	}
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
