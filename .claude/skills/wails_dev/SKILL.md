---
name: wails_dev
description: Wails framework development (Go + Web).
---
# Wails v2

- **Docs**: https://wails.io/docs/introduction
- **Architecture**: Go backend + Web frontend in single binary
- **Template**: Svelte 5 + Tailwind + shadcn-svelte

## Project Structure
```
bazzite-devkit/
├── app.go              # Main App struct with Wails bindings
├── main.go             # Wails app initialization
├── internal/           # Go backend packages (unchanged)
│   ├── config/
│   ├── device/
│   └── shortcuts/
├── frontend/           # Svelte app
│   ├── src/
│   ├── wailsjs/        # Auto-generated bindings
│   └── package.json
├── wails.json          # Wails config
└── build/              # Build output
```

## CLI Commands
```bash
wails dev              # Dev mode with hot reload
wails build            # Production build
wails build -platform windows/amd64
wails build -platform linux/amd64
wails generate module  # Regenerate frontend bindings
```

## Go -> Frontend Communication

### Bindings (Frontend calls Go)
```go
// app.go - Public methods are auto-exposed
func (a *App) ScanDevices() ([]Device, error) { ... }
```
```typescript
// frontend - Auto-generated in wailsjs/go/main/App
import { ScanDevices } from '../wailsjs/go/main/App';
const devices = await ScanDevices();
```

### Events (Go pushes to Frontend)
```go
// Go side
runtime.EventsEmit(a.ctx, "device:connected", deviceInfo)
```
```typescript
// Frontend side
import { EventsOn } from '../wailsjs/runtime/runtime';
EventsOn("device:connected", (data) => { ... });
```

## Frontend -> Go Communication
- Call bound functions directly (returns Promise)
- Handle errors with try/catch
- Events for real-time updates (progress, status changes)

## Build Configuration (wails.json)
```json
{
  "name": "bazzite-devkit",
  "frontend:install": "npm install",
  "frontend:build": "npm run build",
  "frontend:dev:watcher": "npm run dev",
  "frontend:dev:serverUrl": "auto"
}
```

## Platform Specifics
- **Windows**: Requires WebView2 (auto-installed on Win10+)
- **Linux**: Requires webkit2gtk
- **Cross-compile**: Build on target OS (same as before)
