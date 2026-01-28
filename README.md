# Bazzite Devkit

GUI tool for uploading games to Bazzite/Linux devices and creating Steam shortcuts.

## Features

- Connect to remote devices via SSH
- Upload game folders to the device
- Automatically create Steam shortcuts using [steam-shortcut-manager](https://github.com/lobinuxsoft/steam-shortcut-manager)
- Cross-platform (Windows & Linux)

## Requirements

- Go 1.21 or later
- On the target device: SSH server running

## Building

```bash
# Install dependencies
make deps

# Build for all platforms
make build

# Or build for specific platform
make build-windows
make build-linux

# Create distributable packages
make package
```

## Usage

1. Launch `bazzite-devkit`
2. Add your device (IP, SSH user, password/key)
3. Connect to the device
4. Select a game folder to upload
5. Set the executable name and options
6. Click "Upload & Create Shortcut"

The game will be uploaded to the device and a Steam shortcut will be created automatically.

## Project Structure

```
bazzite-devkit/
├── cmd/bazzite-devkit/    # Main application entry point
├── internal/
│   ├── ui/                # Fyne GUI components
│   └── device/            # SSH/SFTP client
├── steam-shortcut-manager/ # Submodule for managing Steam shortcuts
├── assets/                # Icons and images
├── Makefile              # Build scripts
└── go.mod
```

## License

MIT
