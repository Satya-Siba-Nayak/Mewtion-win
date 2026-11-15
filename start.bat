@echo off
setlocal
cd /d "%~dp0"

echo === Starting Mewtion for Windows ===

:: 1. Check for USB / ADB connection if adb is available
where adb >nul 2>nul
if %ERRORLEVEL% EQU 0 (
    adb devices | findstr /R /C:"[	 ]device$" >nul
    if %ERRORLEVEL% EQU 0 (
        echo USB: Android device detected! Forwarding tcp:8765...
        adb forward tcp:8765 tcp:8765
    )
)

:: 2. Launch Mewtion Overlay & Control Panel
if exist "%~dp0Mewtion.exe" (
    start "" "%~dp0Mewtion.exe"
    "%~dp0control_panel.exe"
) else if exist "%~dp0bin\Mewtion.exe" (
    start "" "%~dp0bin\Mewtion.exe"
    "%~dp0bin\control_panel.exe"
) else (
    echo Error: Mewtion.exe not found!
    pause
    exit /b 1
)

:: Cleanly stop the overlay when the Control Panel window is closed
taskkill /F /IM Mewtion.exe >nul 2>nul
endlocal
