@echo off
setlocal enabledelayedexpansion

echo ============================================
echo   CapyDeploy - Build All (Windows)
echo ============================================
echo.

:: Project root
set "ROOT_DIR=%~dp0"
set "ROOT_DIR=%ROOT_DIR:~0,-1%"
set "DIST_DIR=%ROOT_DIR%\dist"

:: Initialize submodules (decky plugin)
git submodule update --init --recursive

:: Parse arguments
set SKIP_DEPS=0

:parse_args
if "%~1"=="" goto :done_args
if "%~1"=="--skip-deps" (
    set SKIP_DEPS=1
    shift
    goto :parse_args
)
if "%~1"=="--help" goto :show_help
if "%~1"=="-h" goto :show_help
echo [ERROR] Unknown option: %~1
exit /b 1

:show_help
echo Usage: build_all.bat [options]
echo.
echo Options:
echo   --skip-deps       Skip frontend dependency installation
echo   --help, -h        Show this help message
echo.
echo Builds:
echo   - Hub (Wails + Go)
echo   - Desktop Agent (Wails + Go)
echo   - Decky Plugin (TypeScript frontend only)
echo.
echo Output: dist\
exit /b 0

:done_args

:: Track results
set HUB_RESULT=failed
set DESKTOP_RESULT=failed
set DECKY_RESULT=failed

:: ============================================
:: Check required tools
:: ============================================

echo [1/5] Checking required tools...
echo.

where go >nul 2>nul
if %errorlevel% neq 0 (
    echo   [ERROR] Go not found. Install from https://go.dev/dl/
    exit /b 1
)
for /f "tokens=3" %%v in ('go version') do set "GO_VER=%%v"
echo   Go: %GO_VER%

where bun >nul 2>nul
if %errorlevel% neq 0 (
    echo   [ERROR] Bun not found. Install from https://bun.sh
    exit /b 1
)
for /f %%v in ('bun --version 2^>nul') do set "BUN_VER=%%v"
echo   Bun: %BUN_VER%

where wails >nul 2>nul
if %errorlevel% neq 0 (
    echo   [WARN] Wails CLI not found. Installing...
    go install github.com/wailsapp/wails/v2/cmd/wails@latest
    where wails >nul 2>nul
    if !errorlevel! neq 0 (
        echo   [ERROR] Wails install failed. Add %%GOPATH%%\bin to PATH.
        exit /b 1
    )
)
echo   Wails: OK

echo.
echo   All tools OK!
echo.

:: ============================================
:: Version info
:: ============================================

echo [2/5] Collecting version info...
echo.

set "BASE_VERSION=0.1.0"

for /f %%h in ('git rev-parse --short HEAD 2^>nul') do set "COMMIT=%%h"
if not defined COMMIT set "COMMIT=unknown"

for /f "tokens=*" %%d in ('powershell -NoProfile -Command "Get-Date -Format 'yyyy-MM-ddTHH:mm:ssZ' -AsUTC"') do set "BUILD_DATE=%%d"

set "VERSION=%BASE_VERSION%-dev+%COMMIT%"
for /f %%t in ('git describe --tags --exact-match 2^>nul') do (
    set "EXACT_TAG=%%t"
    :: Strip 'v' prefix for release version
    set "VERSION=!EXACT_TAG:~1!"
)

echo   Version:    %VERSION%
echo   Commit:     %COMMIT%
echo   Build Date: %BUILD_DATE%
echo.

set "LDFLAGS=-X github.com/lobinuxsoft/capydeploy/pkg/version.Version=%VERSION% -X github.com/lobinuxsoft/capydeploy/pkg/version.Commit=%COMMIT% -X github.com/lobinuxsoft/capydeploy/pkg/version.BuildDate=%BUILD_DATE%"

:: ============================================
:: Build Hub
:: ============================================

echo [3/5] Building Hub...
echo.

if %SKIP_DEPS%==0 (
    echo   Installing frontend dependencies...
    pushd "%ROOT_DIR%\apps\hub\frontend"
    call bun install
    if !errorlevel! neq 0 (
        echo   [ERROR] Failed to install Hub frontend deps
        popd
        goto :build_desktop
    )
    popd
)

:: Generate icons
where python >nul 2>nul
if %errorlevel%==0 (
    echo   Generating icons...
    pushd "%ROOT_DIR%\apps\hub"
    python "..\..\scripts\generate-icons.py" 2>nul
    popd
) else (
    echo   [WARN] Python not found, skipping icon generation
)

:: Build Hub with Wails
echo   Building Hub (Wails)...
pushd "%ROOT_DIR%\apps\hub"
wails build -clean -ldflags "%LDFLAGS%"
if !errorlevel! neq 0 (
    echo   [ERROR] Hub build failed
    popd
    goto :build_desktop
)
popd

if not exist "%DIST_DIR%\windows" mkdir "%DIST_DIR%\windows"
copy /y "%ROOT_DIR%\apps\hub\build\bin\capydeploy-hub.exe" "%DIST_DIR%\windows\" >nul
echo   Hub: OK
set HUB_RESULT=success
echo.

:: ============================================
:: Build Desktop Agent
:: ============================================

:build_desktop
echo [4/5] Building Desktop Agent...
echo.

if %SKIP_DEPS%==0 (
    echo   Installing frontend dependencies...
    pushd "%ROOT_DIR%\apps\agents\desktop\frontend"
    call bun install
    if !errorlevel! neq 0 (
        echo   [ERROR] Failed to install Agent frontend deps
        popd
        goto :build_decky
    )
    popd
)

:: Generate icons
where python >nul 2>nul
if %errorlevel%==0 (
    echo   Generating icons...
    pushd "%ROOT_DIR%\apps\agents\desktop"
    python "..\..\scripts\generate-icons.py" 2>nul
    popd
) else (
    echo   [WARN] Python not found, skipping icon generation
)

:: Build Agent with Wails
echo   Building Desktop Agent (Wails)...
pushd "%ROOT_DIR%\apps\agents\desktop"
wails build -clean -ldflags "%LDFLAGS%"
if !errorlevel! neq 0 (
    echo   [ERROR] Desktop Agent build failed
    popd
    goto :build_decky
)
popd

if not exist "%DIST_DIR%\windows" mkdir "%DIST_DIR%\windows"
copy /y "%ROOT_DIR%\apps\agents\desktop\build\bin\capydeploy-agent.exe" "%DIST_DIR%\windows\" >nul
echo   Desktop Agent: OK
set DESKTOP_RESULT=success
echo.

:: ============================================
:: Build Decky Plugin (frontend only)
:: ============================================

:build_decky
echo [5/5] Building Decky Plugin (frontend)...
echo.

if %SKIP_DEPS%==0 (
    echo   Installing dependencies...
    pushd "%ROOT_DIR%\apps\agents\decky"
    call bun install
    if !errorlevel! neq 0 (
        echo   [ERROR] Failed to install Decky deps
        popd
        goto :summary
    )
    popd
)

echo   Building frontend...
pushd "%ROOT_DIR%\apps\agents\decky"
call bun run build
if !errorlevel! neq 0 (
    echo   [ERROR] Decky frontend build failed
    popd
    goto :summary
)
popd

echo   Decky Plugin: OK (full package requires Linux/Deck)
set DECKY_RESULT=success
echo.

:: ============================================
:: Summary
:: ============================================

:summary
echo.
echo ============================================
echo   Build Summary
echo ============================================
echo.
echo Output: %DIST_DIR%
echo.

echo Windows:
if "%HUB_RESULT%"=="success" (
    if exist "%DIST_DIR%\windows\capydeploy-hub.exe" (
        for %%F in ("%DIST_DIR%\windows\capydeploy-hub.exe") do set /a "SIZE_MB=%%~zF / 1048576"
        echo   [OK] Hub: !SIZE_MB! MB
    )
) else (
    echo   [FAIL] Hub
)

if "%DESKTOP_RESULT%"=="success" (
    if exist "%DIST_DIR%\windows\capydeploy-agent.exe" (
        for %%F in ("%DIST_DIR%\windows\capydeploy-agent.exe") do set /a "SIZE_MB=%%~zF / 1048576"
        echo   [OK] Desktop Agent: !SIZE_MB! MB
    )
) else (
    echo   [FAIL] Desktop Agent
)

echo.
echo Decky:
if "%DECKY_RESULT%"=="success" (
    echo   [OK] Frontend built (full ZIP requires Linux/Deck)
) else (
    echo   [FAIL] Plugin
)

echo.
echo Done!
endlocal
