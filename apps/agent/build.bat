@echo off
setlocal enabledelayedexpansion

:: Build script for CapyDeploy Agent (Windows)

set "OUTPUT=build"
set "VERSION=dev"
set "PLATFORM="

:parse_args
if "%~1"=="" goto :main
if /i "%~1"=="-o" (set "OUTPUT=%~2" & shift & shift & goto :parse_args)
if /i "%~1"=="--output" (set "OUTPUT=%~2" & shift & shift & goto :parse_args)
if /i "%~1"=="-v" (set "VERSION=%~2" & shift & shift & goto :parse_args)
if /i "%~1"=="--version" (set "VERSION=%~2" & shift & shift & goto :parse_args)
if /i "%~1"=="-p" (set "PLATFORM=%~2" & shift & shift & goto :parse_args)
if /i "%~1"=="--platform" (set "PLATFORM=%~2" & shift & shift & goto :parse_args)
if /i "%~1"=="-h" goto :usage
if /i "%~1"=="--help" goto :usage
echo [ERROR] Unknown option: %~1
goto :usage

:usage
echo Usage: build.bat [OPTIONS]
echo.
echo Options:
echo   -o, --output DIR     Output directory (default: build)
echo   -v, --version VER    Version string (default: dev)
echo   -p, --platform OS    Target platform: linux, windows (default: windows)
echo   -h, --help           Show this help message
echo.
echo Examples:
echo   build.bat                           # Build for Windows
echo   build.bat -p linux                  # Build for Linux
echo   build.bat -p windows -v 1.0.0       # Build with version
exit /b 1

:main
:: Create output directory
if not exist "%OUTPUT%" mkdir "%OUTPUT%"

set "LDFLAGS=-X main.Version=%VERSION% -s -w"

if "%PLATFORM%"=="" set "PLATFORM=windows"

if /i "%PLATFORM%"=="linux" (
    call :build linux amd64 capydeploy-agent
) else if /i "%PLATFORM%"=="windows" (
    call :build windows amd64 capydeploy-agent.exe
) else (
    echo [ERROR] Unknown platform: %PLATFORM%
    echo Supported platforms: linux, windows
    exit /b 1
)

echo [INFO] Build complete!
exit /b 0

:build
set "GOOS=%~1"
set "GOARCH=%~2"
set "OUTNAME=%~3"

echo [INFO] Building for %GOOS%/%GOARCH%...

go build -ldflags "%LDFLAGS%" -o "%OUTPUT%\%OUTNAME%" .

if errorlevel 1 (
    echo [ERROR] Build failed
    exit /b 1
)

echo [INFO] Built: %OUTPUT%\%OUTNAME%
exit /b 0
