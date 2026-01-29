# Bazzite Devkit

A cross-platform GUI tool for uploading and managing games on Bazzite/Linux devices. Upload games from your Windows or Linux PC to your handheld device and automatically create Steam shortcuts.

## Features

- **Network Scanner**: Automatically discover SSH-enabled devices on your local network
- **Device Management**: Save and manage multiple device configurations with SSH credentials
- **Game Upload**: Upload game folders to remote devices via SFTP
- **Steam Shortcuts**: Automatically create Steam shortcuts for uploaded games
- **Installed Games Management**: View, manage, and delete games installed on remote devices
- **Persistent Configuration**: Save device and game setup configurations for reuse
- **Single Binary**: All functionality is built into a single executable (steam-shortcut-manager is integrated as a library)

## Requirements

### For Building
- Go 1.21 or later
- GCC/MinGW (for Windows, required by Fyne)
  - Install via: `winget install -e --id=BrechtSanders.WinLibs.POSIX.UCRT`

### For Running
- Windows 10/11 or Linux
- Target device must have:
  - SSH server enabled
  - Steam installed

## Building

### Windows

Run the build script:
```batch
build.bat
```

This will create:
- `build/windows/bazzite-devkit.exe`

### Linux

Run the build script on a Linux machine:
```bash
chmod +x build.sh
./build.sh
```

This will create:
- `build/linux/bazzite-devkit`

**Note:** Fyne requires CGO, so you cannot cross-compile from Windows to Linux. You must build on the target platform.

## Usage

### Step 1: Launch the Application

Run `bazzite-devkit.exe` from the `build/windows` folder.

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

1. In the Devices list, click the **Connect** button (login icon) next to your device
2. Wait for the connection to establish
3. The status will change to "● Connected"

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
4. Click **Save Setup**

### Step 5: Upload the Game

1. In the game setups list, click the **Upload** button (upload icon) next to your game
2. Wait for the upload to complete
3. The tool will:
   - Create the remote directory
   - Upload all game files
   - Set executable permissions
   - Create a Steam shortcut

### Step 6: Play the Game

1. On your device, restart Steam or go to Library
2. The game should appear in your library under "Non-Steam Games"
3. Launch and enjoy!

### Managing Installed Games

1. Go to the **Installed Games** tab
2. Click **Refresh** to see games installed on the connected device
3. Select a game and click **Delete Game** to remove it (this also removes the Steam shortcut)

## Configuration

Configuration is stored in:
- Windows: `%APPDATA%/bazzite-devkit/config.json`
- Linux: `~/.config/bazzite-devkit/config.json`

## Project Structure

```
bazzite-devkit/
├── cmd/bazzite-devkit/     # Main application entry point
├── internal/
│   ├── ui/                 # Fyne GUI components
│   ├── device/             # SSH/SFTP client
│   ├── config/             # Configuration management
│   └── shortcuts/          # Steam shortcuts management (uses steam-shortcut-manager library)
├── steam-shortcut-manager/ # Steam shortcut management library (integrated)
├── build/                  # Compiled binaries
│   ├── windows/
│   └── linux/
├── build.bat               # Windows build script
└── go.mod
```

## Troubleshooting

### Cannot connect to device
- Ensure SSH is enabled on the target device
- Verify the IP address and credentials
- Check that port 22 is not blocked by a firewall

### Game doesn't appear in Steam
- Restart Steam on the target device
- Check that the shortcuts.vdf file was created in `~/.steam/steam/userdata/<user_id>/config/`

### Game won't launch
- Verify the executable path is correct
- Ensure the executable has proper permissions (the tool sets these automatically)
- Check that all required dependencies are installed on the target device

## License

Apache License 2.0 - See [LICENSE](LICENSE) for details.

## Credits

- Built with [Fyne](https://fyne.io/) - Cross-platform GUI toolkit for Go
- Based on [steam-shortcut-manager](https://github.com/shadowblip/steam-shortcut-manager) by ShadowBlip - Steam shortcuts management library
