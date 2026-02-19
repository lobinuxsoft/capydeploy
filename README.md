# CapyDeploy

<div align="center">
  <img src="docs/mascot.gif" alt="CapyDeploy" width="200">

  **Deploy games to your handheld devices with the chill energy of a capybara.**

  [![License](https://img.shields.io/badge/License-AGPL%20v3-blue.svg)](LICENSE)
  [![Rust](https://img.shields.io/badge/Rust-stable-DEA584?logo=rust)](https://www.rust-lang.org/)
  [![Tauri](https://img.shields.io/badge/Tauri-v2-FFC131?logo=tauri)](https://tauri.app/)
</div>

## Overview

CapyDeploy is a cross-platform tool for uploading and managing games on Steam Deck, Bazzite, and other handheld Linux devices. It uses a **Hub-Agent architecture** where the Hub (your PC) sends commands to the Agent (handheld device) over WebSocket.

### What is CapyDeploy

- A **game deployment tool** for developers testing builds on handheld hardware
- A **DRM-free game manager** for games you legally own (GOG, itch.io, Humble Bundle)
- A **retro gaming organizer** for legally obtained content
- A **platform-agnostic alternative** to Valve's SteamOS Devkit Client

### What CapyDeploy is NOT

- **NOT a piracy tool.** CapyDeploy does not download, crack, or distribute games.
- **NOT a DRM bypass.** It does not circumvent any copy protection mechanisms.
- **NOT a game distribution platform.** It transfers files between your own devices on a local network.
- **NOT a game store or launcher.** It manages deployment, not purchasing or playing games.

### Key Features

- **Auto-Discovery**: Agents broadcast via mDNS. No IP configuration needed.
- **WebSocket Protocol**: Persistent bidirectional connection with real-time progress.
- **Secure Pairing**: 6-digit code on first connection. Token stored for future sessions.
- **Binary Uploads**: Games sent as 1MB chunks. Resume on disconnect.
- **Steam Integration**: Automatic shortcuts with artwork from SteamGridDB.
- **Agent Autonomy**: Hub sends simple orders, Agent handles everything internally.
- **Hardware Telemetry**: Real-time CPU, GPU, RAM, battery, fan metrics streamed to Hub.
- **Console Log Streaming**: Live Steam console logs with level filtering (debug/info/warn/error).
- **Game Log Wrapper**: Inject a wrapper into Steam launch options to capture game stdout/stderr (Linux only).

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
| **Decky Plugin** | Runs inside [Decky Loader](https://github.com/SteamDeckHomebrew/decky-loader) (gaming mode). Same protocol as Agent but uses SteamClient APIs directly — no Steam restart needed. Includes telemetry, console log streaming, and game log wrapper. |

## Download

Pre-built binaries are available for each release:

| Component | Linux | Windows |
|-----------|-------|---------|
| **Hub** (your PC) | [AppImage](https://github.com/lobinuxsoft/capydeploy/releases/latest/download/CapyDeploy_Hub.AppImage) | [ZIP](https://github.com/lobinuxsoft/capydeploy/releases/latest/download/capydeploy-windows-amd64.zip) |
| **Agent** (handheld) | [AppImage](https://github.com/lobinuxsoft/capydeploy/releases/latest/download/CapyDeploy_Agent.AppImage) | [ZIP](https://github.com/lobinuxsoft/capydeploy/releases/latest/download/capydeploy-windows-amd64.zip) |
| **Decky Plugin** (gaming mode) | [ZIP](https://github.com/lobinuxsoft/decky-capydeploy/releases/latest/download/CapyDeploy.zip) | — |

Also available: [Linux tar.gz](https://github.com/lobinuxsoft/capydeploy/releases/latest/download/capydeploy-linux-amd64.tar.gz) (Hub + Agent) · [Checksums](https://github.com/lobinuxsoft/capydeploy/releases/latest/download/checksums-sha256.txt) · [All releases](https://github.com/lobinuxsoft/capydeploy/releases)

See the [Installation Guide](https://lobinuxsoft.github.io/capydeploy/install) for platform-specific instructions.

## Building from Source (for contributors)

### Requirements
- Rust (stable): https://rustup.rs
- Bun: https://bun.sh

### Platform Dependencies

| Platform | Dependencies |
|----------|--------------|
| Fedora/Bazzite | `rpm-ostree install webkit2gtk4.1-devel gtk3-devel pkg-config` |
| Ubuntu/Debian | `apt install libwebkit2gtk-4.1-dev libgtk-3-dev pkg-config build-essential` |
| Arch | `pacman -S webkit2gtk-4.1 gtk3 pkgconf base-devel` |
| Windows | WebView2 (pre-installed on Win10/11) |

### Build

```bash
# Clone
git clone https://github.com/lobinuxsoft/capydeploy
cd capydeploy

# Build everything at once
./build_all.sh              # Linux
build_all.bat               # Windows

# Available flags (Linux)
./build_all.sh --skip-deps  # Skip frontend dependency installation
./build_all.sh --parallel   # Build components in parallel

# Or build individually
cd apps/hub-tauri && ./build.sh           # Hub (your PC)
cd apps/agents/agent-tauri && ./build.sh  # Agent (handheld - desktop mode)
cd apps/agents/decky && ./build.sh        # Decky Plugin (handheld - gaming mode)
```

### Decky Plugin

The Decky plugin is an alternative Agent for gaming mode. Requires [Decky Loader](https://github.com/SteamDeckHomebrew/decky-loader) on the handheld.

```bash
cd apps/agents/decky && ./build.sh
# Output: dist/decky/CapyDeploy-Decky.zip
# Install via Decky Settings > Install from ZIP
```

| Feature | Agent (Tauri) | Decky Plugin |
|---------|---------------|--------------|
| Shortcuts | `shortcuts.vdf` (restart Steam) | `SteamClient.Apps.AddShortcut()` (instant) |
| Artwork | File copy to `grid/` | `SteamClient.Apps.SetCustomArtworkForApp()` |
| Telemetry | sysfs/procfs collector | sysfs/procfs collector |
| Console Log | CEF CDP console capture | SteamClient console capture |
| Game Log | Wrapper via CEF launch options | Wrapper via context menu |
| UI | Standalone window | Quick Access Menu panel |
| Mode | Desktop | Gaming |

## AppImage (Linux)

For easy distribution on Linux, AppImage packaging is integrated into each app's build script:

```bash
# Build Hub with AppImage
cd apps/hub-tauri && ./build.sh

# Build Agent with AppImage
cd apps/agents/agent-tauri && ./build.sh
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

For a detailed walkthrough with visual mockups of each application, see the [Usage Guide](https://lobinuxsoft.github.io/capydeploy/usage).

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

1. Go to **Upload Game** tab
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
| `ping` | `pong` | Keep-alive heartbeat |
| `get_info` | `info_response` | Agent details |
| `get_config` | `config_response` | Get agent configuration |
| `get_steam_users` | `steam_users_response` | List Steam users |
| `list_shortcuts` | `shortcuts_response` | List shortcuts |
| `create_shortcut` | `operation_result` | Create shortcut |
| `delete_shortcut` | `operation_result` | Delete shortcut by appID |
| `delete_game` | `operation_result` | Delete game (Agent handles everything) |
| `apply_artwork` | `artwork_response` | Apply artwork from URL |
| `send_artwork_image` | `artwork_image_response` | Upload artwork image binary |
| `restart_steam` | `steam_response` | Restart Steam client |
| `init_upload` | `upload_init_response` | Start upload session |
| `upload_chunk` | `upload_chunk_response` | Send binary chunk |
| `complete_upload` | `operation_result` | Finalize upload |
| `cancel_upload` | `operation_result` | Cancel active upload |
| `set_console_log_filter` | `operation_result` | Set log level bitmask filter |
| `set_console_log_enabled` | `operation_result` | Enable/disable console log streaming |
| `set_game_log_wrapper` | `operation_result` | Enable/disable game log wrapper (Linux only) |

### Push Events

| Event | Description |
|-------|-------------|
| `upload_progress` | Real-time upload progress |
| `operation_event` | Operation status (delete, install) |
| `telemetry_status` | Telemetry collector state (enabled, interval) |
| `telemetry_data` | Hardware metrics (CPU, GPU, RAM, battery, fan, power) |
| `console_log_status` | Console log collector state (enabled, level mask) |
| `console_log_data` | Batch of console log entries with level/source |
| `game_log_wrapper_status` | Active game log wrappers (appID → enabled map) |

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

## Versioning

CapyDeploy uses [SemVer](https://semver.org/) with a single `VERSION` file at the repository root as the source of truth. Version injection is automated:

- **Rust apps** (Hub, Agent): Version read from `Cargo.toml` workspace, synced with `VERSION`
- **Decky plugin**: `plugin.json` updated by `build.sh` from `VERSION`
- **Releases**: Managed by [release-please](https://github.com/googleapis/release-please), triggered by [Conventional Commits](https://www.conventionalcommits.org/)

## Project Structure

```
capydeploy/
├── Cargo.toml                  # Workspace root
├── VERSION                     # Single source of truth for version (release-please)
├── apps/
│   ├── hub-tauri/              # Hub application (Tauri + Svelte)
│   │   ├── src-tauri/          # Rust backend (commands, state, events)
│   │   └── frontend/           # Svelte 5 UI
│   └── agents/
│       ├── agent-tauri/        # Agent (Tauri + Svelte)
│       │   ├── src-tauri/      # Rust backend (server, pairing, uploads)
│       │   └── frontend/       # Svelte 5 UI
│       └── decky/              # Decky Loader plugin (git submodule)
│           ├── main.py         # Python backend (WS server, pairing, uploads)
│           ├── telemetry.py    # Hardware telemetry (sysfs/procfs)
│           ├── console_log.py  # Console log collector
│           ├── game_log.py     # Game log file tailer
│           ├── src/            # React/TypeScript frontend
│           └── build.sh        # Build + bundle script
├── crates/                     # Shared Rust crates
│   ├── protocol/               # WebSocket protocol types
│   ├── discovery/              # mDNS discovery
│   ├── steam/                  # Steam paths/users/controller
│   ├── steamgriddb/            # SteamGridDB API client
│   ├── transfer/               # Chunked file transfer
│   ├── telemetry/              # Hardware telemetry collector
│   ├── console-log/            # Console log collector
│   ├── game-log/               # Game log wrapper
│   ├── file-ops/               # File operations
│   ├── agent-server/           # Agent WebSocket server
│   ├── hub-connection/         # Hub WebSocket client
│   ├── hub-deploy/             # Hub deploy logic
│   ├── hub-games/              # Hub game management
│   ├── hub-telemetry/          # Hub telemetry state
│   ├── hub-console-log/        # Hub console log state
│   └── hub-settings/           # Hub configuration
├── tests/wire_compat/          # Wire compatibility tests
└── docs/                       # Documentation website (GitHub Pages)
```

## Documentation

Full documentation available at: [lobinuxsoft.github.io/capydeploy](https://lobinuxsoft.github.io/capydeploy)

- [About & Legal](https://lobinuxsoft.github.io/capydeploy/about) — What is / isn't CapyDeploy, disclaimer
- [Usage Guide](https://lobinuxsoft.github.io/capydeploy/usage) — Visual walkthrough with mockups
- [Architecture](https://lobinuxsoft.github.io/capydeploy/architecture) — System diagrams
- [API Reference](https://lobinuxsoft.github.io/capydeploy/api) — WebSocket protocol
- [Installation](https://lobinuxsoft.github.io/capydeploy/install) — Build & install guides
- [Developers](https://lobinuxsoft.github.io/capydeploy/developers) — Adding new platforms

## Contributing

1. Fork the repository
2. Create a feature branch from `development`
3. Make your changes
4. Submit a PR to `development`

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

## License

AGPL v3 - See [LICENSE](LICENSE) for details.

This means:
- Free to use, modify, and distribute
- Contributions welcome
- Derivatives must use the same license
- Source code must be provided (even for SaaS)
- Original authors must be credited

## Disclaimer

CapyDeploy was created as an early development testing tool for deploying game builds to handheld devices. While capable of installing DRM-free games, it was **not conceived or intended for piracy** or any form of copyright infringement. The author assumes no responsibility for any misuse of this software by end users. Users are solely responsible for complying with all applicable laws and licensing agreements in their jurisdiction.

See the full [Legal Disclaimer](https://lobinuxsoft.github.io/capydeploy/about) for more details.

## Support

If you find CapyDeploy useful, consider supporting development:

- **BTC**: `bc1qkxy898wa6mz04c9hrjekx6p0yht2ukz56e9xxq`
- **USDT (TRC20)**: `TF6AXBP3LKBCcbJkLG6RqyMsrPNs2JCpdQ`
- **USDT (BEP20)**: `0xd8d2Ed67C567CB3Af437f4638d3531e560575A20`
- **Binance Pay**: `78328894`

## Credits

- Built with [Tauri](https://tauri.app/) + [Svelte 5](https://svelte.dev/)
- Decky plugin with [Decky Loader](https://github.com/SteamDeckHomebrew/decky-loader) + React
- Steam shortcut management via native CEF API integration
- Artwork from [SteamGridDB](https://www.steamgriddb.com/)
