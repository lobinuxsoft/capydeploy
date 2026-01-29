package ui

import (
	"fmt"
	"image/color"
	"net"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"fyne.io/fyne/v2"
	"fyne.io/fyne/v2/canvas"
	"fyne.io/fyne/v2/container"
	"fyne.io/fyne/v2/dialog"
	"fyne.io/fyne/v2/theme"
	"fyne.io/fyne/v2/widget"

	"github.com/lobinuxsoft/bazzite-devkit/internal/config"
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

// NetworkDevice represents a device found on the network
type NetworkDevice struct {
	IP       string
	Hostname string
	HasSSH   bool
}

var deviceList *widget.List
var devices []*Device

func init() {
	devices = make([]*Device, 0)
	loadSavedDevices()
}

// loadSavedDevices loads devices from the config file
func loadSavedDevices() {
	savedDevices, err := config.GetDevices()
	if err != nil {
		return
	}

	for _, d := range savedDevices {
		devices = append(devices, &Device{
			Name:     d.Name,
			Host:     d.Host,
			Port:     d.Port,
			User:     d.User,
			KeyFile:  d.KeyFile,
			Password: d.Password,
		})
	}
}

// saveDevice saves a device to the config file
func saveDevice(dev *Device) {
	config.AddDevice(config.DeviceConfig{
		Name:     dev.Name,
		Host:     dev.Host,
		Port:     dev.Port,
		User:     dev.User,
		KeyFile:  dev.KeyFile,
		Password: dev.Password,
	})
}

// updateDeviceConfig updates a device in the config file
func updateDeviceConfig(oldHost string, dev *Device) {
	config.UpdateDevice(oldHost, config.DeviceConfig{
		Name:     dev.Name,
		Host:     dev.Host,
		Port:     dev.Port,
		User:     dev.User,
		KeyFile:  dev.KeyFile,
		Password: dev.Password,
	})
}


// badgeLayout positions a small badge in the bottom-right corner
type badgeLayout struct{}

func (b *badgeLayout) MinSize(objects []fyne.CanvasObject) fyne.Size {
	return fyne.NewSize(24, 24)
}

func (b *badgeLayout) Layout(objects []fyne.CanvasObject, size fyne.Size) {
	for _, obj := range objects {
		// Position badge in bottom-right corner
		badgeSize := fyne.NewSize(8, 8)
		obj.Resize(badgeSize)
		obj.Move(fyne.NewPos(size.Width-badgeSize.Width, size.Height-badgeSize.Height))
	}
}

// deviceRowData stores widget references for a device list row
type deviceRowData struct {
	nameLabel   *widget.Label
	statusLabel *widget.Label
	statusDot   *canvas.Circle
	connectBtn  *widget.Button
	editBtn     *widget.Button
	deleteBtn   *widget.Button
}

// Map to store row data by container pointer
var deviceRowCache = make(map[fyne.CanvasObject]*deviceRowData)

// createDevicesTab creates the devices management tab
func createDevicesTab() fyne.CanvasObject {
	// Device list with inline buttons
	deviceList = widget.NewList(
		func() int { return len(devices) },
		func() fyne.CanvasObject {
			nameLabel := widget.NewLabel("Device Name")
			statusLabel := widget.NewLabel("Disconnected")
			statusLabel.TextStyle = fyne.TextStyle{Italic: true}

			// Status badge (small dot) overlaid on icon corner
			statusDot := canvas.NewCircle(color.RGBA{128, 128, 128, 255})
			statusDot.StrokeWidth = 1
			statusDot.StrokeColor = color.RGBA{40, 40, 40, 255}

			// Icon with status badge overlay
			icon := widget.NewIcon(theme.ComputerIcon())
			iconWithBadge := container.NewStack(
				icon,
				container.NewPadded(container.New(&badgeLayout{}, statusDot)),
			)

			connectBtn := widget.NewButtonWithIcon("", theme.LoginIcon(), nil)
			editBtn := widget.NewButtonWithIcon("", theme.DocumentCreateIcon(), nil)
			deleteBtn := widget.NewButtonWithIcon("", theme.DeleteIcon(), nil)

			buttons := container.NewHBox(connectBtn, editBtn, deleteBtn)
			infoRow := container.NewHBox(
				iconWithBadge,
				nameLabel,
				widget.NewLabel("   "),
				statusLabel,
			)

			c := container.NewBorder(nil, nil, nil, buttons, infoRow)

			// Store widget references
			deviceRowCache[c] = &deviceRowData{
				nameLabel:   nameLabel,
				statusLabel: statusLabel,
				statusDot:   statusDot,
				connectBtn:  connectBtn,
				editBtn:     editBtn,
				deleteBtn:   deleteBtn,
			}

			return c
		},
		func(id widget.ListItemID, obj fyne.CanvasObject) {
			if id >= len(devices) {
				return
			}
			dev := devices[id]

			// Get cached widget references
			row, ok := deviceRowCache[obj]
			if !ok {
				return
			}

			// Update labels
			row.nameLabel.SetText(fmt.Sprintf("%s (%s@%s)", dev.Name, dev.User, dev.Host))
			if dev.Connected {
				row.statusLabel.SetText("Connected")
				row.statusDot.FillColor = color.RGBA{0, 200, 0, 255} // Green
				row.connectBtn.SetIcon(theme.LogoutIcon())
				row.connectBtn.Importance = widget.DangerImportance
			} else {
				row.statusLabel.SetText("Disconnected")
				row.statusDot.FillColor = color.RGBA{128, 128, 128, 255} // Gray
				row.connectBtn.SetIcon(theme.LoginIcon())
				row.connectBtn.Importance = widget.HighImportance
			}
			row.statusDot.Refresh()
			row.connectBtn.Refresh()

			// Set button actions
			row.connectBtn.OnTapped = func() {
				if dev.Connected {
					disconnectDevice(dev)
				} else {
					go connectToDevice(dev)
				}
			}

			row.editBtn.OnTapped = func() {
				showEditDeviceWindow(dev)
			}

			row.deleteBtn.OnTapped = func() {
				dialog.ShowConfirm("Delete Device",
					fmt.Sprintf("Are you sure you want to delete '%s'?", dev.Name),
					func(ok bool) {
						if ok {
							removeDevice(dev)
						}
					}, State.Window)
			}
		},
	)

	deviceList.OnSelected = func(id widget.ListItemID) {
		if id < len(devices) {
			State.SelectedDevice = devices[id]
		}
	}

	// Top buttons - only Scan and Add
	scanBtn := widget.NewButtonWithIcon("Scan Network", theme.SearchIcon(), func() {
		showScanNetworkWindow()
	})

	addBtn := widget.NewButtonWithIcon("Add Device", theme.ContentAddIcon(), func() {
		showAddDeviceWindow()
	})

	buttons := container.NewHBox(scanBtn, addBtn)

	return container.NewBorder(
		buttons,
		nil, nil, nil,
		deviceList,
	)
}

// showScanNetworkWindow shows a window to scan and select network devices
func showScanNetworkWindow() {
	scanWindow := fyne.CurrentApp().NewWindow("Scan Network")
	scanWindow.Resize(fyne.NewSize(500, 400))

	var foundDevices []NetworkDevice
	var networkList *widget.List
	scanningLabel := widget.NewLabel("Click 'Scan' to find devices with SSH...")
	progressBar := widget.NewProgressBarInfinite()
	progressBar.Hide()

	networkList = widget.NewList(
		func() int { return len(foundDevices) },
		func() fyne.CanvasObject {
			return container.NewHBox(
				widget.NewIcon(theme.ComputerIcon()),
				widget.NewLabel("IP Address"),
				widget.NewLabel("Hostname"),
			)
		},
		func(id widget.ListItemID, obj fyne.CanvasObject) {
			if id >= len(foundDevices) {
				return
			}
			dev := foundDevices[id]
			box := obj.(*fyne.Container)
			ipLabel := box.Objects[1].(*widget.Label)
			hostLabel := box.Objects[2].(*widget.Label)
			ipLabel.SetText(dev.IP)
			if dev.Hostname != "" {
				hostLabel.SetText(fmt.Sprintf("(%s)", dev.Hostname))
			} else {
				hostLabel.SetText("")
			}
		},
	)

	var selectedNetDevice *NetworkDevice
	networkList.OnSelected = func(id widget.ListItemID) {
		if id < len(foundDevices) {
			selectedNetDevice = &foundDevices[id]
		}
	}

	scanBtn := widget.NewButtonWithIcon("Scan", theme.SearchIcon(), func() {
		progressBar.Show()
		scanningLabel.SetText("Scanning network for SSH devices...")
		foundDevices = []NetworkDevice{}
		networkList.Refresh()

		go func() {
			found := scanNetworkForSSH()
			foundDevices = found
			progressBar.Hide()
			scanningLabel.SetText(fmt.Sprintf("Found %d devices with SSH", len(found)))
			networkList.Refresh()
		}()
	})

	selectBtn := widget.NewButtonWithIcon("Select & Configure", theme.ConfirmIcon(), func() {
		if selectedNetDevice != nil {
			scanWindow.Close()
			showAddDeviceWindowWithIP(selectedNetDevice.IP, selectedNetDevice.Hostname)
		}
	})

	topBar := container.NewVBox(
		container.NewHBox(scanBtn, selectBtn),
		scanningLabel,
		progressBar,
	)

	scanWindow.SetContent(container.NewBorder(
		topBar,
		nil, nil, nil,
		networkList,
	))

	scanWindow.Show()
}

// showAddDeviceWindow shows a separate window to add a device
func showAddDeviceWindow() {
	showAddDeviceWindowWithIP("", "")
}

// showAddDeviceWindowWithIP shows the add device window with pre-filled IP
func showAddDeviceWindowWithIP(ip, hostname string) {
	showDeviceForm(nil, ip, hostname)
}

// showEditDeviceWindow shows the edit device window
func showEditDeviceWindow(dev *Device) {
	showDeviceForm(dev, dev.Host, dev.Name)
}

// showDeviceForm shows a form to add or edit a device
func showDeviceForm(existingDev *Device, ip, hostname string) {
	isEdit := existingDev != nil
	title := "Add Device"
	if isEdit {
		title = "Edit Device"
	}

	formWindow := fyne.CurrentApp().NewWindow(title)
	formWindow.Resize(fyne.NewSize(500, 450))

	nameEntry := widget.NewEntry()
	hostEntry := widget.NewEntry()
	portEntry := widget.NewEntry()
	userEntry := widget.NewEntry()
	passwordEntry := widget.NewPasswordEntry()
	keyFileEntry := widget.NewEntry()

	if isEdit {
		nameEntry.SetText(existingDev.Name)
		hostEntry.SetText(existingDev.Host)
		portEntry.SetText(fmt.Sprintf("%d", existingDev.Port))
		userEntry.SetText(existingDev.User)
		passwordEntry.SetText(existingDev.Password)
		keyFileEntry.SetText(existingDev.KeyFile)
	} else {
		if hostname != "" {
			nameEntry.SetText(hostname)
		} else {
			nameEntry.SetPlaceHolder("My Bazzite Device")
		}
		if ip != "" {
			hostEntry.SetText(ip)
		} else {
			hostEntry.SetPlaceHolder("192.168.1.100")
		}
		portEntry.SetText("22")
		userEntry.SetText("deck")
		keyFileEntry.SetPlaceHolder("~/.ssh/id_ed25519")
		passwordEntry.SetPlaceHolder("SSH Password")
	}

	// Detect existing SSH keys
	existingKeys := findExistingSSHKeys()
	keySelect := widget.NewSelect(existingKeys, func(selected string) {
		keyFileEntry.SetText(selected)
	})
	if len(existingKeys) > 0 && keyFileEntry.Text == "" {
		keySelect.SetSelected(existingKeys[0])
		keyFileEntry.SetText(existingKeys[0])
	}

	// Auth method containers
	passwordContainer := container.NewVBox(
		widget.NewLabel("Password:"),
		passwordEntry,
	)

	keyContainer := container.NewVBox(
		widget.NewLabel("Select SSH Key:"),
		keySelect,
		widget.NewLabel("Or enter path manually:"),
		keyFileEntry,
	)
	keyContainer.Hide()

	// Auth type selector
	authType := widget.NewRadioGroup([]string{"Password", "SSH Key"}, func(selected string) {
		if selected == "Password" {
			passwordContainer.Show()
			keyContainer.Hide()
		} else {
			passwordContainer.Hide()
			keyContainer.Show()
		}
	})

	// Set initial auth type based on existing device
	if isEdit && existingDev.KeyFile != "" {
		authType.SetSelected("SSH Key")
	} else {
		authType.SetSelected("Password")
	}

	// Basic info form
	basicForm := widget.NewForm(
		widget.NewFormItem("Name", nameEntry),
		widget.NewFormItem("Host/IP", hostEntry),
		widget.NewFormItem("Port", portEntry),
		widget.NewFormItem("User", userEntry),
	)

	saveBtn := widget.NewButtonWithIcon("Save", theme.ConfirmIcon(), func() {
		port := 22
		fmt.Sscanf(portEntry.Text, "%d", &port)

		name := nameEntry.Text
		if name == "" {
			name = hostEntry.Text
		}

		var password, keyFile string
		if authType.Selected == "Password" {
			password = passwordEntry.Text
		} else {
			keyFile = keyFileEntry.Text
		}

		if isEdit {
			oldHost := existingDev.Host
			existingDev.Name = name
			existingDev.Host = hostEntry.Text
			existingDev.Port = port
			existingDev.User = userEntry.Text
			existingDev.KeyFile = keyFile
			existingDev.Password = password
			updateDeviceConfig(oldHost, existingDev)
		} else {
			dev := &Device{
				Name:     name,
				Host:     hostEntry.Text,
				Port:     port,
				User:     userEntry.Text,
				KeyFile:  keyFile,
				Password: password,
			}
			devices = append(devices, dev)
			State.Devices = devices
			saveDevice(dev)
		}

		deviceList.Refresh()
		formWindow.Close()
	})

	cancelBtn := widget.NewButtonWithIcon("Cancel", theme.CancelIcon(), func() {
		formWindow.Close()
	})

	buttons := container.NewHBox(cancelBtn, saveBtn)

	content := container.NewVBox(
		widget.NewLabelWithStyle("Configure SSH Connection", fyne.TextAlignCenter, fyne.TextStyle{Bold: true}),
		widget.NewSeparator(),
		basicForm,
		widget.NewSeparator(),
		widget.NewLabelWithStyle("Authentication Method", fyne.TextAlignLeading, fyne.TextStyle{Bold: true}),
		authType,
		passwordContainer,
		keyContainer,
		widget.NewSeparator(),
		container.NewCenter(buttons),
	)

	formWindow.SetContent(container.NewPadded(content))
	formWindow.Show()
}

// findExistingSSHKeys looks for SSH keys in ~/.ssh/
func findExistingSSHKeys() []string {
	var keys []string
	home, err := os.UserHomeDir()
	if err != nil {
		return keys
	}

	sshDir := filepath.Join(home, ".ssh")
	keyNames := []string{"id_ed25519", "id_rsa", "id_ecdsa", "id_dsa"}

	for _, name := range keyNames {
		keyPath := filepath.Join(sshDir, name)
		if _, err := os.Stat(keyPath); err == nil {
			keys = append(keys, keyPath)
		}
	}

	return keys
}

// scanNetworkForSSH scans the local network for devices with SSH (port 22) open
func scanNetworkForSSH() []NetworkDevice {
	var found []NetworkDevice
	var mu sync.Mutex
	var wg sync.WaitGroup

	localIP := getLocalIP()
	if localIP == "" {
		return found
	}

	parts := strings.Split(localIP, ".")
	if len(parts) != 4 {
		return found
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
	return found
}

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
	State.SelectedDevice = dev
	deviceList.Refresh()
	UpdateConnectionStatus()

	dialog.ShowInformation("Connected", fmt.Sprintf("Connected to %s", dev.Name), State.Window)
}

func disconnectDevice(dev *Device) {
	if dev.Client != nil {
		dev.Client.Close()
		dev.Client = nil
	}
	dev.Connected = false
	if State.SelectedDevice == dev {
		State.SelectedDevice = nil
	}
	deviceList.Refresh()
	UpdateConnectionStatus()
}

func removeDevice(dev *Device) {
	disconnectDevice(dev)
	config.RemoveDevice(dev.Host)
	for i, d := range devices {
		if d == dev {
			devices = append(devices[:i], devices[i+1:]...)
			break
		}
	}
	State.Devices = devices
	if State.SelectedDevice == dev {
		State.SelectedDevice = nil
	}
	deviceList.Refresh()
}
