package tray

import (
	"fmt"
	"sync"

	"github.com/energye/systray"
)

// HubInfo contains information about a connected Hub.
type HubInfo struct {
	Name string
	IP   string
}

// Status represents the current agent status for the tray.
type Status struct {
	Running           bool
	AcceptConnections bool
	ConnectedHub      *HubInfo
	Name              string
	Address           string // "IP:Port"
}

// Config contains callbacks and getters for the tray.
type Config struct {
	OnOpenWebUI    func()
	OnCopyAddress  func() string
	OnToggleAccept func(bool)
	OnQuit         func()
	GetStatus      func() Status
}

// Tray manages the system tray icon and menu.
type Tray struct {
	config Config
	mu     sync.RWMutex
	status Status

	// Lifecycle
	closed   bool
	closeMu  sync.RWMutex
	quitOnce sync.Once

	// Menu items
	mTitle    *systray.MenuItem
	mStatus   *systray.MenuItem
	mToggle   *systray.MenuItem
	mOpenUI   *systray.MenuItem
	mCopyAddr *systray.MenuItem
	mQuit     *systray.MenuItem
}

// New creates a new Tray instance.
func New(cfg Config) *Tray {
	return &Tray{
		config: cfg,
	}
}

// Run starts the system tray. This is a blocking call that should run in a goroutine.
func (t *Tray) Run() {
	systray.Run(t.onReady, t.onExit)
}

// UpdateStatus updates the tray icon and menu based on new status.
func (t *Tray) UpdateStatus(s Status) {
	// Don't update if tray is closed
	t.closeMu.RLock()
	if t.closed {
		t.closeMu.RUnlock()
		return
	}
	t.closeMu.RUnlock()

	t.mu.Lock()
	t.status = s
	t.mu.Unlock()

	// Update icon based on state
	t.updateIcon()

	// Update tooltip
	t.updateTooltip()

	// Update menu items
	t.updateMenu()
}

// Quit signals the tray to exit.
func (t *Tray) Quit() {
	t.quitOnce.Do(func() {
		t.closeMu.Lock()
		t.closed = true
		t.closeMu.Unlock()
		systray.Quit()
	})
}

func (t *Tray) onReady() {
	// Set initial icon
	systray.SetIcon(IconWaiting())
	systray.SetTitle("CapyDeploy Agent")
	systray.SetTooltip("CapyDeploy Agent - Esperando conexión")

	// Build menu
	t.mTitle = systray.AddMenuItem("CapyDeploy Agent", "")
	t.mTitle.Disable()

	systray.AddSeparator()

	t.mStatus = systray.AddMenuItem("Sin conexión", "Estado de conexión al Hub")
	t.mStatus.Disable()

	systray.AddSeparator()

	t.mToggle = systray.AddMenuItemCheckbox("Aceptar conexiones", "Habilitar/deshabilitar conexiones entrantes", true)

	systray.AddSeparator()

	t.mOpenUI = systray.AddMenuItem("Abrir Web UI", "Abrir interfaz web en el navegador")
	t.mCopyAddr = systray.AddMenuItem("Copiar IP:Puerto", "Copiar dirección al portapapeles")

	systray.AddSeparator()

	t.mQuit = systray.AddMenuItem("Salir", "Cerrar CapyDeploy Agent")

	// Set up click handlers
	t.mToggle.Click(func() {
		t.mu.RLock()
		currentAccept := t.status.AcceptConnections
		t.mu.RUnlock()

		newAccept := !currentAccept
		if t.config.OnToggleAccept != nil {
			t.config.OnToggleAccept(newAccept)
		}

		// Update checkbox state
		if newAccept {
			t.mToggle.Check()
		} else {
			t.mToggle.Uncheck()
		}
	})

	t.mOpenUI.Click(func() {
		if t.config.OnOpenWebUI != nil {
			t.config.OnOpenWebUI()
		}
	})

	t.mCopyAddr.Click(func() {
		if t.config.OnCopyAddress != nil {
			addr := t.config.OnCopyAddress()
			if addr != "" {
				copyToClipboard(addr)
			}
		}
	})

	t.mQuit.Click(func() {
		if t.config.OnQuit != nil {
			t.config.OnQuit()
		}
		systray.Quit()
	})

	// Get initial status
	if t.config.GetStatus != nil {
		t.UpdateStatus(t.config.GetStatus())
	}
}

func (t *Tray) onExit() {
	// Mark as closed to prevent any further updates
	t.closeMu.Lock()
	t.closed = true
	t.closeMu.Unlock()
}

func (t *Tray) updateIcon() {
	t.mu.RLock()
	defer t.mu.RUnlock()

	if !t.status.AcceptConnections {
		systray.SetIcon(IconDisabled())
	} else if t.status.ConnectedHub != nil {
		systray.SetIcon(IconConnected())
	} else {
		systray.SetIcon(IconWaiting())
	}
}

func (t *Tray) updateTooltip() {
	t.mu.RLock()
	defer t.mu.RUnlock()

	var tooltip string
	if !t.status.AcceptConnections {
		tooltip = "CapyDeploy Agent - Conexiones deshabilitadas"
	} else if t.status.ConnectedHub != nil {
		tooltip = fmt.Sprintf("CapyDeploy Agent - Conectado a %s", t.status.ConnectedHub.Name)
	} else {
		tooltip = "CapyDeploy Agent - Esperando conexión"
	}

	systray.SetTooltip(tooltip)
}

func (t *Tray) updateMenu() {
	t.mu.RLock()
	defer t.mu.RUnlock()

	// Update status line
	if t.status.ConnectedHub != nil {
		t.mStatus.SetTitle(fmt.Sprintf("Conectado a: %s", t.status.ConnectedHub.Name))
	} else {
		t.mStatus.SetTitle("Sin conexión")
	}

	// Update toggle checkbox
	if t.status.AcceptConnections {
		t.mToggle.Check()
	} else {
		t.mToggle.Uncheck()
	}
}
