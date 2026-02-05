@echo off
setlocal enabledelayedexpansion

echo ============================================
echo   CapyDeploy Agent - Build Script
echo ============================================
echo.

:: Parse arguments
set "MODE=production"
set "SKIP_DEPS=0"

:parse_args
if "%~1"=="" goto :done_args
if /i "%~1"=="dev" set "MODE=dev"
if /i "%~1"=="--dev" set "MODE=dev"
if /i "%~1"=="-d" set "MODE=dev"
if /i "%~1"=="--skip-deps" set "SKIP_DEPS=1"
if /i "%~1"=="--help" goto :show_help
if /i "%~1"=="-h" goto :show_help
shift
goto :parse_args
:done_args

:: ============================================
:: Check required tools
:: ============================================

echo [1/5] Checking required tools...
echo.

:: Check Go
where go >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Go not found.
    echo Please install Go 1.23+ from: https://go.dev/dl/
    exit /b 1
)
for /f "tokens=3" %%v in ('go version') do set GO_VERSION=%%v
echo   Go: %GO_VERSION%

:: Check Bun
where bun >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Bun not found.
    echo.
    echo Installing Bun...
    powershell -c "irm bun.sh/install.ps1 | iex"
    if %ERRORLEVEL% neq 0 (
        echo [ERROR] Failed to install Bun.
        echo Please install manually from: https://bun.sh
        exit /b 1
    )
    echo Bun installed. Please restart your terminal and run this script again.
    exit /b 0
)
for /f "tokens=1,2" %%a in ('bun --version 2^>nul') do set BUN_VERSION=%%a
echo   Bun: %BUN_VERSION%

:: Check Wails
where wails >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo.
    echo [WARN] Wails CLI not found. Installing...
    go install github.com/wailsapp/wails/v2/cmd/wails@latest
    if %ERRORLEVEL% neq 0 (
        echo [ERROR] Failed to install Wails CLI.
        exit /b 1
    )
    echo Wails CLI installed.
)
for /f "tokens=2" %%v in ('wails version 2^>nul ^| findstr /i "version"') do set WAILS_VERSION=%%v
echo   Wails: %WAILS_VERSION%

echo.
echo   All tools OK!
echo.

:: ============================================
:: Install frontend dependencies
:: ============================================

if %SKIP_DEPS%==0 (
    echo [2/5] Installing frontend dependencies...
    cd frontend
    call bun install
    if %ERRORLEVEL% neq 0 (
        echo [ERROR] Failed to install frontend dependencies.
        cd ..
        exit /b 1
    )
    cd ..
    echo   Dependencies installed.
    echo.
) else (
    echo [2/5] Skipping frontend dependencies ^(--skip-deps^)
    echo.
)

:: ============================================
:: Generate icons (production only)
:: ============================================

if "%MODE%"=="production" (
    echo [3/5] Generating application icons...
    echo.

    if exist "..\..\scripts\generate-icons.py" (
        where python >nul 2>&1
        if %ERRORLEVEL% equ 0 (
            python ..\..\scripts\generate-icons.py
        ) else (
            echo   [WARN] Python not found, skipping icon generation.
            echo   Install Python 3 + Pillow to generate icons.
        )
    ) else (
        echo   [WARN] Icon script not found: ..\..\scripts\generate-icons.py
    )
    echo.
) else (
    echo [3/5] Skipping icon generation ^(dev mode^)
    echo.
)

:: ============================================
:: Build
:: ============================================

if "%MODE%"=="dev" (
    echo [4/5] Starting development server...
    echo.
    echo   Press Ctrl+C to stop.
    echo.
    wails dev
) else (
    echo [4/5] Building production binary...
    echo.

    wails build -clean -windowsconsole
    if %ERRORLEVEL% neq 0 (
        echo.
        echo ============================================
        echo   BUILD FAILED
        echo ============================================
        exit /b 1
    )

    echo.
    echo ============================================
    echo   BUILD SUCCESSFUL
    echo ============================================
    echo.

    :: Show result
    echo [5/5] Build output:
    echo.

    if exist "build\bin\capydeploy-agent.exe" (
        for %%A in ("build\bin\capydeploy-agent.exe") do (
            set SIZE=%%~zA
            set /a SIZE_KB=!SIZE!/1024
            set /a SIZE_MB=!SIZE!/1048576
            echo   File: build\bin\capydeploy-agent.exe
            echo   Size: !SIZE_MB! MB ^(!SIZE_KB! KB^)
        )
    ) else (
        echo   Output directory: build\bin\
        dir /b "build\bin\" 2>nul
    )

    echo.
    echo Done! Run with: .\build\bin\capydeploy-agent.exe
)

exit /b 0

:show_help
echo.
echo Usage: build.bat [options]
echo.
echo Options:
echo   dev, --dev, -d    Start in development mode with hot reload
echo   --skip-deps       Skip frontend dependency installation
echo   --help, -h        Show this help message
echo.
echo Examples:
echo   build.bat              Build production binary
echo   build.bat dev          Start development server
echo   build.bat --skip-deps  Build without reinstalling deps
echo.
exit /b 0
