@echo off
setlocal
cd /d "%~dp0"

echo === Starting Mewtion Ecosystem (Windows Dev Build) ===

:: 1. Check for USB / ADB connection
where adb >nul 2>nul
if %ERRORLEVEL% EQU 0 (
    adb devices | findstr /R /C:"[	 ]device$" >nul
    if %ERRORLEVEL% EQU 0 (
        echo USB: Android device detected! Forwarding tcp:8765...
        adb forward tcp:8765 tcp:8765
    ) else (
        echo No USB Android device detected. Using built-in sensors or Wi-Fi mode.
    )
) else (
    echo ADB not in PATH. Using built-in sensors or Wi-Fi mode.
)

:: 2. Build Release Binaries
echo Compiling release binaries...
cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo Build failed!
    pause
    exit /b %ERRORLEVEL%
)

:: 3. Launch Overlay & Control Panel
echo Launching Mewtion Overlay...
start "" "%~dp0target\release\Mewtion.exe"

echo Launching Mewtion Control Panel...
"%~dp0target\release\control_panel.exe"

:: Cleanly stop overlay when Control Panel closes
taskkill /F /IM Mewtion.exe >nul 2>nul
endlocal
