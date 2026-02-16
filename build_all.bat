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
echo   - Hub (Tauri + Rust)
echo   - Desktop Agent (Tauri + Rust)
echo   - Decky Plugin (TypeScript frontend only)
echo.
echo Output: dist\
exit /b 0

:done_args

:: Track results
set HUB_RESULT=failed
set AGENT_RESULT=failed
set DECKY_RESULT=failed

:: ============================================
:: [1/5] Check required tools
:: ============================================

echo [1/5] Checking required tools...
echo.

where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo   [ERROR] cargo not found. Install Rust from https://rustup.rs/
    exit /b 1
)
for /f "tokens=2" %%v in ('cargo --version') do set "CARGO_VER=%%v"
echo   cargo: %CARGO_VER%

where bun >nul 2>nul
if %errorlevel% neq 0 (
    echo   [ERROR] bun not found. Install from https://bun.sh
    exit /b 1
)
for /f %%v in ('bun --version 2^>nul') do set "BUN_VER=%%v"
echo   bun: %BUN_VER%

echo.
echo   All tools OK!
echo.

:: ============================================
:: [2/5] Init submodules
:: ============================================

echo [2/5] Initializing submodules...
git submodule update --init --recursive
echo   Done.
echo.

:: ============================================
:: [3/5] Build frontends
:: ============================================

echo [3/5] Building frontends...
echo.

:: Hub frontend
echo   [Hub] Installing dependencies ^& building...
pushd "%ROOT_DIR%\apps\hub-tauri\frontend"
if %SKIP_DEPS%==0 (
    call bun install
    if !errorlevel! neq 0 (
        echo   [ERROR] Failed to install Hub frontend deps
        popd
        goto :build_rust
    )
)
call bun run build
if !errorlevel! neq 0 (
    echo   [ERROR] Hub frontend build failed
    popd
    goto :build_rust
)
popd
echo   [Hub] Frontend ready

:: Agent frontend
echo   [Agent] Installing dependencies ^& building...
pushd "%ROOT_DIR%\apps\agents\agent-tauri\frontend"
if %SKIP_DEPS%==0 (
    call bun install
    if !errorlevel! neq 0 (
        echo   [ERROR] Failed to install Agent frontend deps
        popd
        goto :build_rust
    )
)
call bun run build
if !errorlevel! neq 0 (
    echo   [ERROR] Agent frontend build failed
    popd
    goto :build_rust
)
popd
echo   [Agent] Frontend ready
echo.

:: ============================================
:: [4/5] Build Windows binaries (cargo)
:: ============================================

:build_rust
echo [4/5] Building Windows binaries (cargo release)...
echo.

pushd "%ROOT_DIR%"
cargo build --release -p capydeploy-hub-tauri -p capydeploy-agent-tauri
if !errorlevel! neq 0 (
    echo   [ERROR] Cargo build failed
    popd
    goto :build_decky
)
popd

if not exist "%DIST_DIR%\windows" mkdir "%DIST_DIR%\windows"
copy /y "%ROOT_DIR%\target\release\capydeploy-hub-tauri.exe" "%DIST_DIR%\windows\" >nul
copy /y "%ROOT_DIR%\target\release\capydeploy-agent-tauri.exe" "%DIST_DIR%\windows\" >nul
set HUB_RESULT=success
set AGENT_RESULT=success
echo   Windows binaries ready
echo.

:: ============================================
:: [5/5] Build Decky plugin (frontend only)
:: ============================================

:build_decky
echo [5/5] Building Decky plugin (frontend only)...
echo.

where npm >nul 2>nul
if %errorlevel% neq 0 (
    echo   [WARN] npm not found, skipping Decky frontend build
    goto :summary
)

pushd "%ROOT_DIR%\apps\agents\decky"
if %SKIP_DEPS%==0 (
    call npm install
    if !errorlevel! neq 0 (
        echo   [ERROR] Failed to install Decky deps
        popd
        goto :summary
    )
)
call npm run build
if !errorlevel! neq 0 (
    echo   [ERROR] Decky frontend build failed
    popd
    goto :summary
)
popd

echo   Decky: OK (full package requires Linux)
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
    if exist "%DIST_DIR%\windows\capydeploy-hub-tauri.exe" (
        for %%F in ("%DIST_DIR%\windows\capydeploy-hub-tauri.exe") do set /a "SIZE_MB=%%~zF / 1048576"
        echo   [OK] Hub: !SIZE_MB! MB
    )
) else (
    echo   [FAIL] Hub
)

if "%AGENT_RESULT%"=="success" (
    if exist "%DIST_DIR%\windows\capydeploy-agent-tauri.exe" (
        for %%F in ("%DIST_DIR%\windows\capydeploy-agent-tauri.exe") do set /a "SIZE_MB=%%~zF / 1048576"
        echo   [OK] Agent: !SIZE_MB! MB
    )
) else (
    echo   [FAIL] Agent
)

echo.
echo Decky:
if "%DECKY_RESULT%"=="success" (
    echo   [OK] Frontend built (full ZIP requires Linux)
) else (
    echo   [FAIL] Plugin
)

echo.
echo Done!
endlocal
