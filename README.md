# Bazzite Devkit

A cross-platform GUI tool for uploading and managing games on Bazzite/Linux devices. Upload games from your Windows or Linux PC to your handheld device and automatically create Steam shortcuts with custom artwork.

## Features

- **Network Scanner**: Automatically discover SSH-enabled devices on your local network
- **Device Management**: Save and manage multiple device configurations with SSH credentials
- **Game Upload**: Upload game folders to remote devices via SFTP
- **Steam Shortcuts**: Automatically create Steam shortcuts for uploaded games
- **SteamGridDB Integration**: Select custom artwork (capsules, heroes, logos, icons) with support for animated WebP/GIF
- **Installed Games Management**: View, manage, and delete games installed on remote devices
- **Persistent Configuration**: Save device and game setup configurations for reuse
- **Native Performance**: Built with Wails + Svelte 5 for a lightweight, fast UI

## Requirements

### For Building
- Go 1.23 or later
- Bun: https://bun.sh
  - Windows: `powershell -c "irm bun.sh/install.ps1 | iex"`
  - Linux/Mac: `curl -fsSL https://bun.sh/install | bash`
- Wails CLI: `go install github.com/wailsapp/wails/v2/cmd/wails@latest`
- Windows: WebView2 (included in Windows 10+)
- Linux: `webkit2gtk-4.0`

### For Running
- Windows 10/11 or Linux
- Target device must have:
  - SSH server enabled
  - Steam installed

## Building

### Development Mode

```bash
wails dev
```

This starts the app in development mode with hot reload.

### Production Build

Use the build scripts:

```bash
# Windows
build.bat

# Linux/Mac
chmod +x build.sh
./build.sh
```

Or manually:

```bash
cd frontend && bun install && cd ..
wails build
```

**Note:** Cross-compilation is not supported. Build on the target platform.

## Usage

### Step 1: Launch the Application

Run `bazzite-devkit` from the build folder.

### Step 2: Add a Device

1. Go to the **Devices** tab
2. Click **Scan Network** to find devices with SSH, or click **Add Device** to add manually
3. Enter the device details:
   - **Name**: A friendly name for the device
   - **Host/IP**: The IP address of your device
   - **Port**: SSH port (default: 22)
   - **User**: SSH username
   - **Authentication**: Choose password or SSH key
4. Click **Save**

### Step 3: Connect to the Device

1. In the Devices list, click the **Connect** button next to your device
2. Wait for the connection to establish
3. The status indicator will turn green when connected

### Step 4: Create a Game Setup

1. Go to the **Upload Game** tab
2. Click **New Game Setup**
3. Fill in the details:
   - **Game Name**: Name for the game (will be used for the Steam shortcut)
   - **Local Folder**: Browse to select the game folder on your PC
   - **Executable**: The main executable file (e.g., `game.x86_64` or `game.sh`)
   - **Launch Options**: Optional command-line arguments
   - **Tags**: Optional Steam tags (comma-separated)
   - **Remote Path**: Where to install on the device (default: `~/devkit-games`)
   - **Artwork**: Click "Select Artwork" to choose custom images from SteamGridDB
4. Click **Save Setup**

### Step 5: Upload the Game

1. In the game setups list, click the **Upload** button next to your game
2. Wait for the upload to complete
3. The tool will:
   - Create the remote directory
   - Upload all game files
   - Set executable permissions
   - Create a Steam shortcut with artwork

### Step 6: Play the Game

1. On your device, Steam will auto-restart to load the new shortcut
2. The game should appear in your library under "Non-Steam Games"
3. Launch and enjoy!

### Managing Installed Games

1. Go to the **Installed Games** tab
2. Click **Refresh** to see games installed on the connected device
3. Select a game and click **Delete Game** to remove it (this also removes the Steam shortcut)

### SteamGridDB Artwork

1. Go to **Settings** tab
2. Enter your SteamGridDB API key (get one from [steamgriddb.com](https://www.steamgriddb.com/profile/preferences/api))
3. Click **Save Settings**
4. When creating a game setup, click "Select Artwork" to browse and select:
   - **Capsule**: 600x900 portrait grid
   - **Wide Capsule**: 920x430 landscape grid
   - **Hero**: 1920x620 banner image
   - **Logo**: Game logo with transparency
   - **Icon**: Square icon

## Configuration

Configuration is stored in:
- Windows: `%APPDATA%/bazzite-devkit/config.json`
- Linux: `~/.config/bazzite-devkit/config.json`

Image cache is stored in:
- Windows: `%APPDATA%/bazzite-devkit/cache/images/`
- Linux: `~/.config/bazzite-devkit/cache/images/`

## Project Structure

```
bazzite-devkit/
├── main.go                    # Wails entry point
├── app.go                     # App struct with Go bindings
├── wails.json                 # Wails configuration
├── internal/
│   ├── config/                # Configuration management
│   ├── device/                # SSH/SFTP client
│   ├── shortcuts/             # Steam shortcuts management
│   └── steamgriddb/           # SteamGridDB API client
├── frontend/
│   ├── src/
│   │   ├── lib/
│   │   │   ├── components/    # Svelte components
│   │   │   ├── stores/        # Svelte stores
│   │   │   └── types.ts       # TypeScript types
│   │   └── routes/            # SvelteKit routes
│   ├── package.json
│   └── vite.config.ts
├── steam-shortcut-manager/    # Steam shortcut management library
└── go.mod
```

## Troubleshooting

### Cannot connect to device
- Ensure SSH is enabled on the target device
- Verify the IP address and credentials
- Check that port 22 is not blocked by a firewall

### Game doesn't appear in Steam
- Steam will auto-restart after upload to load the shortcut
- Check that the shortcuts.vdf file was created in `~/.steam/steam/userdata/<user_id>/config/`

### Game won't launch
- Verify the executable path is correct
- Ensure the executable has proper permissions (the tool sets these automatically)
- Check that all required dependencies are installed on the target device

### Artwork not showing
- Verify your SteamGridDB API key is correct in Settings
- Check that artwork was selected before uploading
- Try clearing the image cache in Settings

## License

Apache License 2.0 - See [LICENSE](LICENSE) for details.

## Credits

- Built with [Wails](https://wails.io/) - Build desktop apps with Go and Web Technologies
- Frontend with [Svelte 5](https://svelte.dev/) + [Tailwind CSS](https://tailwindcss.com/)
- Based on [steam-shortcut-manager](https://github.com/shadowblip/steam-shortcut-manager) by ShadowBlip
- Artwork from [SteamGridDB](https://www.steamgriddb.com/)
