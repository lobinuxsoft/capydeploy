package ui

import (
	"fmt"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/widget"

	"github.com/lobinuxsoft/bazzite-devkit/internal/device"
)

// Device represents a remote device
type Device struct {
	Name      string
	Host      string
	Port      int
	User      string
	KeyFile   string
	Password  string
	Connected bool
	Client    *device.Client
}

var deviceList *widget.List
var devices []*Device

func init() {
	devices = make([]*Device, 0)
}

// createDevicesTab creates the devices management tab
func createDevicesTab() fyne.CanvasObject {
	// Device list
	deviceList = widget.NewList(
		func() int { return len(devices) },
		func() fyne.CanvasObject {
			return container.NewHBox(
				widget.NewIcon(nil),
				widget.NewLabel("Device Name"),
				widget.NewLabel("Status"),
			)
		},
		func(id widget.ListItemID, obj fyne.CanvasObject) {
			if id >= len(devices) {
				return
			}
			dev := devices[id]
			box := obj.(*fyne.Container)
			nameLabel := box.Objects[1].(*widget.Label)
			statusLabel := box.Objects[2].(*widget.Label)

			nameLabel.SetText(fmt.Sprintf("%s (%s@%s)", dev.Name, dev.User, dev.Host))
			if dev.Connected {
				statusLabel.SetText("Connected")
			} else {
				statusLabel.SetText("Disconnected")
			}
		},
	)

	deviceList.OnSelected = func(id widget.ListItemID) {
		if id < len(devices) {
			State.SelectedDevice = devices[id]
		}
	}

	// Buttons
	addBtn := widget.NewButton("Add Device", func() {
		showAddDeviceDialog()
	})

	connectBtn := widget.NewButton("Connect", func() {
		if State.SelectedDevice != nil {
			connectToDevice(State.SelectedDevice)
		}
	})

	disconnectBtn := widget.NewButton("Disconnect", func() {
		if State.SelectedDevice != nil && State.SelectedDevice.Connected {
			disconnectDevice(State.SelectedDevice)
		}
	})

	removeBtn := widget.NewButton("Remove", func() {
		if State.SelectedDevice != nil {
			removeDevice(State.SelectedDevice)
		}
	})

	buttons := container.NewHBox(addBtn, connectBtn, disconnectBtn, removeBtn)

	return container.NewBorder(
		buttons,
		nil, nil, nil,
		deviceList,
	)
}

// showAddDeviceDialog shows the dialog to add a new device
func showAddDeviceDialog() {
	nameEntry := widget.NewEntry()
	nameEntry.SetPlaceHolder("My Bazzite Device")

	hostEntry := widget.NewEntry()
	hostEntry.SetPlaceHolder("192.168.1.100")

	portEntry := widget.NewEntry()
	portEntry.SetText("22")

	userEntry := widget.NewEntry()
	userEntry.SetPlaceHolder("deck")

	keyFileEntry := widget.NewEntry()
	keyFileEntry.SetPlaceHolder("~/.ssh/id_rsa (optional)")

	passwordEntry := widget.NewPasswordEntry()
	passwordEntry.SetPlaceHolder("Password (if no key)")

	form := widget.NewForm(
		widget.NewFormItem("Name", nameEntry),
		widget.NewFormItem("Host", hostEntry),
		widget.NewFormItem("Port", portEntry),
		widget.NewFormItem("User", userEntry),
		widget.NewFormItem("SSH Key", keyFileEntry),
		widget.NewFormItem("Password", passwordEntry),
	)

	dialog.ShowCustomConfirm("Add Device", "Add", "Cancel", form, func(ok bool) {
		if !ok {
			return
		}

		port := 22
		fmt.Sscanf(portEntry.Text, "%d", &port)

		dev := &Device{
			Name:     nameEntry.Text,
			Host:     hostEntry.Text,
			Port:     port,
			User:     userEntry.Text,
			KeyFile:  keyFileEntry.Text,
			Password: passwordEntry.Text,
		}
		devices = append(devices, dev)
		State.Devices = devices
		deviceList.Refresh()
	}, State.Window)
}

// connectToDevice connects to the selected device
func connectToDevice(dev *Device) {
	client, err := device.NewClient(dev.Host, dev.Port, dev.User, dev.Password, dev.KeyFile)
	if err != nil {
		dialog.ShowError(err, State.Window)
		return
	}

	if err := client.Connect(); err != nil {
		dialog.ShowError(err, State.Window)
		return
	}

	dev.Client = client
	dev.Connected = true
	deviceList.Refresh()

	dialog.ShowInformation("Connected", fmt.Sprintf("Connected to %s", dev.Name), State.Window)
}

// disconnectDevice disconnects from the device
func disconnectDevice(dev *Device) {
	if dev.Client != nil {
		dev.Client.Close()
		dev.Client = nil
	}
	dev.Connected = false
	deviceList.Refresh()
}

// removeDevice removes a device from the list
func removeDevice(dev *Device) {
	disconnectDevice(dev)
	for i, d := range devices {
		if d == dev {
			devices = append(devices[:i], devices[i+1:]...)
			break
		}
	}
	State.Devices = devices
	State.SelectedDevice = nil
	deviceList.Refresh()
}
