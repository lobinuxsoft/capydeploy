.PHONY: all build build-windows build-linux build-shortcut-manager clean deps

# Output directories
BUILD_DIR = build
ASSETS_DIR = assets

# Binary names
APP_NAME = bazzite-devkit
SHORTCUT_MANAGER = steam-shortcut-manager

all: deps build

# Install dependencies
deps:
	go mod tidy
	go mod download

# Build everything
build: build-shortcut-manager build-windows build-linux

# Build steam-shortcut-manager binaries
build-shortcut-manager:
	@echo "Building steam-shortcut-manager for Linux..."
	cd steam-shortcut-manager && GOOS=linux GOARCH=amd64 go build -o ../$(BUILD_DIR)/linux/$(SHORTCUT_MANAGER) .
	@echo "Building steam-shortcut-manager for Windows..."
	cd steam-shortcut-manager && GOOS=windows GOARCH=amd64 go build -o ../$(BUILD_DIR)/windows/$(SHORTCUT_MANAGER).exe .

# Build main app for Windows
build-windows: build-shortcut-manager
	@echo "Building $(APP_NAME) for Windows..."
	@mkdir -p $(BUILD_DIR)/windows
	GOOS=windows GOARCH=amd64 go build -o $(BUILD_DIR)/windows/$(APP_NAME).exe ./cmd/bazzite-devkit
	@echo "Copying assets..."
	@cp -r $(ASSETS_DIR)/* $(BUILD_DIR)/windows/ 2>/dev/null || true
	@echo "Windows build complete: $(BUILD_DIR)/windows/"

# Build main app for Linux
build-linux: build-shortcut-manager
	@echo "Building $(APP_NAME) for Linux..."
	@mkdir -p $(BUILD_DIR)/linux
	GOOS=linux GOARCH=amd64 go build -o $(BUILD_DIR)/linux/$(APP_NAME) ./cmd/bazzite-devkit
	@echo "Copying assets..."
	@cp -r $(ASSETS_DIR)/* $(BUILD_DIR)/linux/ 2>/dev/null || true
	@echo "Linux build complete: $(BUILD_DIR)/linux/"

# Create distributable packages
package: build
	@echo "Creating Windows package..."
	cd $(BUILD_DIR) && zip -r $(APP_NAME)-windows-amd64.zip windows/
	@echo "Creating Linux package..."
	cd $(BUILD_DIR) && tar -czvf $(APP_NAME)-linux-amd64.tar.gz linux/
	@echo "Packages created in $(BUILD_DIR)/"

# Clean build artifacts
clean:
	rm -rf $(BUILD_DIR)

# Run the app (development)
run:
	go run ./cmd/bazzite-devkit

# Run tests
test:
	go test ./...
