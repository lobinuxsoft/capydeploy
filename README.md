# CapyDeploy

<div align="center">
  <img src="docs/mascot.gif" alt="CapyDeploy" width="200">

  **Deploy games to your handheld devices with the chill energy of a capybara.**

  [![License](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](LICENSE)
  [![Go](https://img.shields.io/badge/Go-1.24+-00ADD8?logo=go)](https://go.dev/)
  [![Wails](https://img.shields.io/badge/Wails-v2-red)](https://wails.io/)
</div>

## Overview

CapyDeploy is a cross-platform tool for uploading and managing games on Steam Deck, Bazzite, and other handheld Linux devices. It uses a **Hub-Agent architecture** where the Hub (your PC) sends commands to the Agent (handheld device) over WebSocket.

### Key Features

- **Auto-Discovery**: Agents broadcast via mDNS. No IP configuration needed.
- **WebSocket Protocol**: Persistent bidirectional connection with real-time progress.
- **Secure Pairing**: 6-digit code on first connection. Token stored for future sessions.
- **Binary Uploads**: Games sent as 1MB chunks. Resume on disconnect.
- **Steam Integration**: Automatic shortcuts with artwork from SteamGridDB.
- **Agent Autonomy**: Hub sends simple orders, Agent handles everything internally.

## Architecture

```
┌─────────────────┐         WebSocket          ┌─────────────────┐
│                 │◄──────────────────────────►│                 │
│      Hub        │    Binary chunks + JSON    │     Agent       │
│    (Your PC)    │                            │   (Handheld)    │
│                 │         mDNS               │                 │
└─────────────────┘◄───────────────────────────└─────────────────┘
                        Discovery
```

| Component | Role |
|-----------|------|
| **Hub** | Desktop app on your PC. Discovers agents, initiates connections, sends games. |
| **Agent** | Runs on handheld (desktop mode). Receives games, creates Steam shortcuts, applies artwork, restarts Steam. |
| **Decky Plugin** | Runs inside [Decky Loader](https://github.com/SteamDeckHomebrew/decky-loader) (gaming mode). Same protocol as Agent but uses SteamClient APIs directly — no Steam restart needed. |

## Requirements

### Building
- Go 1.24+
- Bun: https://bun.sh
- Wails CLI: `go install github.com/wailsapp/wails/v2/cmd/wails@latest`

### Platform Dependencies

| Platform | Dependencies |
|----------|--------------|
| Fedora/Bazzite | `rpm-ostree install webkit2gtk4.0-devel gtk3-devel` |
| Ubuntu/Debian | `apt install libwebkit2gtk-4.0-dev libgtk-3-dev` |
| Arch | `pacman -S webkit2gtk gtk3` |
| Windows | WebView2 (pre-installed on Win10/11) |

## Building

```bash
# Clone with submodules
git clone https://github.com/lobinuxsoft/capydeploy
cd capydeploy
git submodule update --init --recursive

# Build everything at once
./build_all.sh              # Linux
build_all.bat               # Windows

# Available flags (Linux)
./build_all.sh --skip-deps  # Skip frontend dependency installation
./build_all.sh --parallel   # Build components in parallel

# Or build individually
cd apps/hub && ./build.sh                # Hub (your PC)
cd apps/agents/desktop && ./build.sh     # Agent (handheld - desktop mode)
cd apps/agents/decky && ./build.sh       # Decky Plugin (handheld - gaming mode)
```

### Decky Plugin

The Decky plugin is an alternative Agent for gaming mode. Requires [Decky Loader](https://github.com/SteamDeckHomebrew/decky-loader) on the handheld.

```bash
cd apps/agents/decky && ./build.sh
# Output: dist/decky/CapyDeploy-v0.1.0.zip
# Install via Decky Settings > Install from ZIP
```

| Feature | Agent (Wails) | Decky Plugin |
|---------|---------------|--------------|
| Shortcuts | `shortcuts.vdf` (restart Steam) | `SteamClient.Apps.AddShortcut()` (instant) |
| Artwork | File copy to `grid/` | `SteamClient.Apps.SetCustomArtworkForApp()` |
| UI | Standalone window | Quick Access Menu panel |
| Mode | Desktop | Gaming |

## AppImage (Linux)

For easy distribution on Linux, AppImage packaging is integrated into each app's build script:

```bash
# Build Hub with AppImage
cd apps/hub && ./build.sh

# Build Agent with AppImage
cd apps/agents/desktop && ./build.sh
```

Output: `dist/` directory with `CapyDeploy_Hub.AppImage` and `CapyDeploy_Agent.AppImage`

### Auto-Installation

When you run the AppImage for the first time, it will prompt to install:
- Moves to `~/.local/bin/`
- Creates desktop entry in `~/.local/share/applications/`
- Copies icon to `~/.local/share/icons/`

You can also use command line flags:

```bash
./CapyDeploy_Agent.AppImage --install    # Install manually
./CapyDeploy_Agent.AppImage --uninstall  # Remove installation
./CapyDeploy_Agent.AppImage --help       # Show options
```

## Usage

### 1. Start the Agent (on handheld)

```bash
./capydeploy-agent
```

The Agent will:
- Start WebSocket server (dynamic port)
- Broadcast via mDNS for discovery
- Show system tray icon

### 2. Start the Hub (on PC)

```bash
./capydeploy-hub
```

The Hub will:
- Discover available Agents automatically
- Show them in the Devices tab

### 3. Pair the Devices

1. Click on a discovered Agent in the Hub
2. A 6-digit pairing code appears on the Agent
3. Enter the code in the Hub
4. Done! Token saved for future connections.

### 4. Upload Games

1. Go to **Game Setups** tab
2. Create a new setup with:
   - Game name
   - Local folder path
   - Executable file
   - Artwork (from SteamGridDB)
3. Click **Upload**
4. Agent receives the game, creates shortcut, applies artwork, restarts Steam

### 5. Manage Games

- View installed games on the **Installed Games** tab
- Delete games directly (Agent handles cleanup + Steam restart)

## WebSocket API

All communication happens over WebSocket at `ws://agent:<port>/ws`

### Message Format
```json
{
  "id": "unique-message-id",
  "type": "message_type",
  "payload": { ... }
}
```

### Message Types

| Request | Response | Description |
|---------|----------|-------------|
| `hub_connected` | `pairing_required` / `pair_success` | Authentication handshake |
| `get_info` | `info_response` | Agent details |
| `get_config` | `config_response` | Get agent configuration |
| `get_steam_users` | `steam_users_response` | List Steam users |
| `list_shortcuts` | `shortcuts_response` | List shortcuts |
| `create_shortcut` | `operation_result` | Create shortcut |
| `delete_shortcut` | `operation_result` | Delete shortcut by appID |
| `delete_game` | `operation_result` | Delete game (Agent handles everything) |
| `apply_artwork` | `artwork_response` | Apply artwork |
| `restart_steam` | `steam_response` | Restart Steam client |
| `init_upload` | `upload_response` | Start upload session |
| `upload_chunk` | `upload_chunk_response` | Send binary chunk |
| `complete_upload` | `upload_response` | Finalize upload |
| `cancel_upload` | `operation_result` | Cancel active upload |

### Push Events

| Event | Description |
|-------|-------------|
| `upload_progress` | Real-time upload progress |
| `operation_event` | Operation status (delete, install) |

## Configuration

### Hub
- Windows: `%APPDATA%/capydeploy-hub/`
- Linux: `~/.config/capydeploy-hub/`

### Agent
- Windows: `%APPDATA%/capydeploy-agent/`
- Linux: `~/.config/capydeploy-agent/`

### Decky Plugin
- Settings: `~/homebrew/settings/capydeploy.json`
- Logs: `~/homebrew/logs/CapyDeploy/`

## Project Structure

```
capydeploy/
├── apps/
│   ├── hub/                    # Hub application (PC)
│   │   ├── app.go              # Wails bindings
│   │   ├── wsclient/           # WebSocket client
│   │   ├── modules/            # Platform modules
│   │   └── frontend/           # Svelte 5 UI
│   └── agents/
│       ├── desktop/            # Agent (Handheld - desktop mode)
│       │   ├── app.go          # Wails bindings
│       │   ├── server/         # HTTP + WebSocket server
│       │   ├── shortcuts/      # Steam shortcut manager
│       │   ├── artwork/        # Artwork handler
│       │   ├── steam/          # Steam controller
│       │   ├── auth/           # Pairing & token auth
│       │   └── frontend/       # Svelte 5 UI
│       └── decky/              # Decky Loader plugin (gaming mode)
│           ├── main.py         # Python backend (WS server, pairing, uploads)
│           ├── src/            # React/TypeScript frontend
│           ├── plugin.json     # Decky plugin manifest
│           └── build.sh        # Build + bundle script
├── pkg/
│   ├── protocol/               # WebSocket protocol types
│   ├── discovery/              # mDNS discovery
│   ├── steam/                  # Steam paths/users
│   ├── steamgriddb/            # SteamGridDB API client
│   ├── config/                 # Configuration management
│   ├── version/                # Version info
│   └── transfer/               # Chunked file transfer
├── internal/
│   └── agent/                  # Agent HTTP client (legacy)
└── docs/                       # Documentation website
```

## Documentation

Full documentation available at: [docs/index.html](docs/index.html)

- WebSocket API reference
- Architecture diagrams
- Installation guides
- Donation options (crypto)

## Contributing

1. Fork the repository
2. Create a feature branch from `development`
3. Make your changes
4. Submit a PR to `development`

## License

AGPL v3 - See [LICENSE](LICENSE) for details.

This means:
- ✅ Free to use, modify, and distribute
- ✅ Contributions welcome
- ⚠️ Derivatives must use the same license
- ⚠️ Source code must be provided (even for SaaS)
- ⚠️ Original authors must be credited

## Support

If you find CapyDeploy useful, consider supporting development:

- **BTC**: `bc1qkxy898wa6mz04c9hrjekx6p0yht2ukz56e9xxq`
- **USDT (TRC20)**: `TF6AXBP3LKBCcbJkLG6RqyMsrPNs2JCpdQ`
- **USDT (BEP20)**: `0xd8d2Ed67C567CB3Af437f4638d3531e560575A20`
- **Binance Pay**: `78328894`

## Credits

- Built with [Wails](https://wails.io/) + [Svelte 5](https://svelte.dev/)
- Decky plugin with [Decky Loader](https://github.com/SteamDeckHomebrew/decky-loader) + React
- Steam shortcut management via native CEF API integration
- Artwork from [SteamGridDB](https://www.steamgriddb.com/)
