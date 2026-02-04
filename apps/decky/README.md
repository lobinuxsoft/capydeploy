# CapyDeploy Decky Plugin

Decky Loader plugin for receiving games from CapyDeploy Hub in gaming mode.

## Features

- **Toggle On/Off** - Enable/disable the WebSocket connection
- **Auto-connect** - Reconnects automatically when enabled
- **Real-time notifications** - Toast notifications for game installs
- **Progress tracking** - See transfer progress in the panel
- **No Steam restart** - Creates shortcuts using native Steam APIs

## Requirements

- Steam Deck or other handheld running SteamOS/Bazzite
- [Decky Loader](https://decky.xyz/) installed
- CapyDeploy Hub running on your PC

## Installation

### From Decky Store (Recommended)
1. Open Decky Loader (... button)
2. Go to the Plugin Store
3. Search for "CapyDeploy"
4. Click Install

### Manual Installation
1. Download the latest release `.zip`
2. Extract to `~/homebrew/plugins/`
3. Restart Decky Loader

## Usage

1. Enable CapyDeploy in the Decky panel
2. Wait for the Hub to connect
3. If prompted, enter the pairing code shown in your Hub
4. Start sending games from the Hub!

## Development

```bash
# Install dependencies
pnpm install

# Build
pnpm build

# Watch mode
pnpm watch
```

## Architecture

```
Hub (PC) ──WebSocket──► Decky Plugin (Handheld)
                              │
                              ▼
                     SteamClient.Apps.*
                     (No restart needed)
```

## License

AGPL-3.0 - See [LICENSE](../../LICENSE)
