@echo off
setlocal enabledelayedexpansion

:: Build script for CapyDeploy Decky Plugin (Windows)

cd /d "%~dp0"

set "PLUGIN_NAME=CapyDeploy"

:: Extract version from package.json
for /f "tokens=2 delims=:, " %%a in ('findstr /c:"\"version\"" package.json') do (
    set "VERSION=%%~a"
    goto :got_version
)
:got_version

set "OUTPUT_DIR=%~dp0out"
set "BUILD_DIR=%OUTPUT_DIR%\%PLUGIN_NAME%"

echo === Building %PLUGIN_NAME% v%VERSION% ===

:: Detect package manager (bun > pnpm > npm)
set "PM="
where bun >nul 2>&1 && set "PM=bun"
if not defined PM where pnpm >nul 2>&1 && set "PM=pnpm"
if not defined PM where npm >nul 2>&1 && set "PM=npm"

if not defined PM (
    echo ERROR: No package manager found ^(bun, pnpm, or npm^)
    echo.
    echo Install one of:
    echo   winget install Oven-sh.Bun
    echo   winget install OpenJS.NodeJS
    exit /b 1
)

echo Using package manager: %PM%

:: Clean previous builds
if exist "%OUTPUT_DIR%" rmdir /s /q "%OUTPUT_DIR%"
mkdir "%BUILD_DIR%"

:: Install dependencies if needed
if not exist "node_modules" (
    echo Installing dependencies...
    %PM% install
)

:: Build frontend (bypass npm script — shx doesn't resolve under bun run)
echo Building frontend...
if exist "dist" rmdir /s /q "dist"
if "%PM%"=="bun" (
    bunx rollup -c --environment ROLLUP_ENV:production
) else (
    %PM% run build
)
if errorlevel 1 (
    echo ERROR: Frontend build failed.
    exit /b 1
)

:: Install Python dependencies into py_modules (cross-compile for Linux/Steam Deck)
echo Installing Python dependencies (targeting Linux x86_64)...
if exist "py_modules" rmdir /s /q "py_modules"
mkdir py_modules
python -m pip install --target py_modules -r requirements.txt --platform manylinux_2_17_x86_64 --python-version 311 --only-binary=:all: --no-cache-dir
if errorlevel 1 (
    echo ERROR: Failed to install Python dependencies for Linux target.
    exit /b 1
)

:: Copy files to build directory
echo Copying files...
copy /y plugin.json "%BUILD_DIR%\" >nul
copy /y package.json "%BUILD_DIR%\" >nul
for %%f in (main.py steam_utils.py mdns_service.py pairing.py upload.py artwork.py ws_server.py) do (
    copy /y "%%f" "%BUILD_DIR%\" >nul
)
copy /y requirements.txt "%BUILD_DIR%\" >nul
xcopy /s /e /i /q py_modules "%BUILD_DIR%\py_modules" >nul

:: Copy dist (frontend bundle)
if exist "dist" (
    xcopy /s /e /i /q dist "%BUILD_DIR%\dist" >nul
) else (
    echo ERROR: dist\ not found. Frontend build failed?
    exit /b 1
)

:: Copy assets
if exist "assets" (
    xcopy /s /e /i /q assets "%BUILD_DIR%\assets" >nul
)

:: Copy LICENSE from root
if exist "..\..\LICENSE" (
    copy /y "..\..\LICENSE" "%BUILD_DIR%\" >nul
)

:: Create ZIP using PowerShell (available on all modern Windows)
echo Creating ZIP...
powershell -NoProfile -Command "Compress-Archive -Path '%BUILD_DIR%' -DestinationPath '%OUTPUT_DIR%\%PLUGIN_NAME%-v%VERSION%.zip' -Force"

echo.
echo === Build complete! ===
echo Output: %OUTPUT_DIR%\%PLUGIN_NAME%-v%VERSION%.zip
echo.
echo Installation options:
echo   1. Manual: Copy %BUILD_DIR% to ~/homebrew/plugins/ on Steam Deck
echo   2. URL: Host the ZIP and use Decky Settings ^> Install from URL
echo   3. Dev: Use decky-cli to deploy during development

endlocal
