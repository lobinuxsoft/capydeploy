@echo off
setlocal

rem Find WinLibs/MinGW
for %%d in (
    "%LOCALAPPDATA%\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\mingw64\bin"
    "C:\WinLibs\mingw64\bin"
    "C:\mingw64\bin"
    "C:\msys64\mingw64\bin"
    "C:\Program Files\WinLibs\mingw64\bin"
    "%LOCALAPPDATA%\WinLibs\mingw64\bin"
    "%LOCALAPPDATA%\Programs\mingw64\bin"
) do (
    if exist "%%~d\gcc.exe" (
        set "MINGW_PATH=%%~d"
        goto :found_mingw
    )
)

echo MinGW/GCC not found. Please install WinLibs via:
echo   winget install -e --id=BrechtSanders.WinLibs.POSIX.UCRT
exit /b 1

:found_mingw
echo Found MinGW at: %MINGW_PATH%
set "PATH=%MINGW_PATH%;C:\Program Files\Go\bin;%PATH%"
set CGO_ENABLED=1

cd /d "%~dp0"

rem Create build directory if it doesn't exist
if not exist "build\windows" mkdir build\windows

rem Build Windows version
echo Building Windows binary...
set GOOS=windows
set GOARCH=amd64

go build -o build\windows\bazzite-devkit.exe ./cmd/bazzite-devkit
if %ERRORLEVEL% NEQ 0 (
    echo Build failed: bazzite-devkit (windows)
    exit /b 1
)

echo.
echo Build successful!
echo   build\windows\bazzite-devkit.exe
echo.
echo Note: To build for Linux, run build.sh on a Linux machine.
