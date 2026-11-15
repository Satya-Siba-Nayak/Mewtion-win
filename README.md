# Mewtion for Windows (`Mewtion-win`)

A native **Windows 10 / 11** (`x86_64` & **Windows on ARM / `arm64`**) port of **[Mewtion](https://github.com/aayuxh-vim/Mewtion)** — an open-source desktop implementation inspired by Apple's **Vehicle Motion Cues**.

Mewtion helps reduce motion sickness when working on your laptop, Surface, or Windows handheld/tablet in a moving vehicle. It creates a transparent, click-through, always-on-top overlay of dots along the edges of your screen. Using real-time accelerometer and gyroscope data, the dots drift in the opposite direction of the vehicle's movement, helping resolve the sensory conflict between your stationary screen and the physical motion your inner ear feels.

## Demo

https://github.com/user-attachments/assets/1a454867-f16b-4b72-ab93-a5af378831a5

---

## Linux Port vs. Windows Port: What's Carried Over & What's New

This repository (`Mewtion-win`) ports the original Linux/Wayland project to run natively on Windows (`x86_64` and `ARM64`) with **zero external DLL dependencies** (no GTK4 or Wayland runtime needed).

| Component | Carried Over from Linux (`Mewtion`) | Re-Engineered & Added for Windows (`Mewtion-win`) |
| :--- | :--- | :--- |
| **Particle Physics & Sensor Fusion** | **100% preserved** (`src/main.rs`) — identical 60 FPS particle simulation, `Fluid` vs. `Rigid` animation modes, stationary anchor dots, software deadband (noise gate), and lateral acceleration + gyroscope yaw (`gz`) fusion. | Driven by a native Win32 frame timer (`WM_TIMER` at ~60 FPS) with dynamic resolution & multi-monitor tracking (`WM_DISPLAYCHANGE`). |
| **Overlay Window & Click-Through** | Inspired by Wayland Layer Shell's top-layer + empty Cairo input region behavior. | Replaced `gtk4` + `gtk4-layer-shell` with a native **Win32 Layered Window** (`WS_EX_LAYERED \| WS_EX_TRANSPARENT \| WS_EX_TOPMOST \| WS_EX_TOOLWINDOW \| WS_EX_NOACTIVATE`) and Per-Monitor V2 High-DPI awareness. Mouse clicks pass straight through at the OS level, and a Win32 Named Mutex prevents duplicate instances. |
| **System Tray Icon & Quick Menu** | *New in Windows port* | `Mewtion.exe` adds a native Windows **System Tray Icon** (`Shell_NotifyIconW`) with a right-click menu to **Open Mewtion Settings**, **Pause Motion Cues**, **Simulate Motion (Desk Test)**, or **Exit Mewtion** (double-click opens Settings). |
| **2D Dot Renderer (`src/dotview.rs`)** | Preserves exact dot coordinates, sizes, RGB colors, and sine-wave alpha fading. | Replaced GTK4/GSK snapshots with a **32-bit premultiplied BGRA DIB section** (`CreateDIBSection` + `UpdateLayeredWindow`) and a custom signed-distance anti-aliased circle rasterizer with dirty-rectangle clearing (<1% CPU usage, zero GPU context overhead). |
| **Built-in Device Sensors** | Preserves the 0.25 Hz first-order low-pass gravity estimator and automatic fallback architecture from `iio.rs`. | Replaced Linux `/sys/bus/iio/devices/` sysfs polling with **`src/win_sensor.rs`** using the Windows Runtime (**WinRT `Windows.Devices.Sensors`**) `Accelerometer` (hardware linear + low-pass gravity fallback) and `Gyrometer` APIs — ideal for Surface Pro/Go, Snapdragon X Elite laptops, 2-in-1 convertibles, and Windows handhelds. |
| **Android Companion Tunnel (`src/tcp.rs`)** | **100% preserved** — fully compatible with **[Mewtion-Android](https://github.com/aayuxh-vim/Mewtion-Android)** over USB (`adb forward tcp:8765 tcp:8765`), Wi-Fi, or Mobile Hotspot. | Same wire protocol (`ax,ay,az,gx,gy,gz` over TCP port `8765`) with non-blocking connection timeouts, instant IP switching, and live connection status reporting to the Control Panel. |
| **Control Panel (`src/bin/control_panel.rs`)** | **Preserved** `iced` GUI layout, live dot preview, color/size/opacity/margin/sensitivity controls, and live `mewtion_config.txt` hot-reloading. | Added **Live Sensor & Connection Status banner**, **Start / Stop Overlay** button, **Multi-Monitor target selector** (`Primary` vs. `All Monitors`), **Sensor Orientation (`0°`/`90°`/`180°`/`270°`) & X/Y Axis Inversion**, **Simulate Motion (Desk Test Mode)**, **Run on Windows Startup** toggle, and silent Windows Hotspot gateway detection. |
| **Native Architectures & Packaging** | Linux had `.AppImage`, `.deb`, `.tar.gz`, and `flake.nix`. | Native **Windows x86_64** (`x86_64-pc-windows-msvc`) and **Windows on ARM64** (`aarch64-pc-windows-msvc`) builds, packaged as both a **Setup Installer (`Mewtion-Setup-*.exe`)** and a **Portable Archive (`Mewtion-*.zip`)** with embedded `.ico` and Win32 version resources. |

---

## Architecture

The system consists of three parts working together for ultra-low latency:

1. **Sensor (`src/win_sensor.rs` / Android Fallback)**
   Mewtion first checks for a built-in Windows accelerometer and gyrometer via the WinRT `Windows.Devices.Sensors` API (supported on Surface devices, Windows on ARM laptops/tablets, 2-in-1 convertibles, and Windows handhelds with an integrated HID Sensor Hub). It prefers hardware linear acceleration (`AccelerometerReadingType::Linear`) when available, or converts standard $g$-force readings to $\text{m/s}^2$ and subtracts gravity using a 0.25 Hz low-pass filter. If no built-in accelerometer is found, it automatically falls back to the **[Mewtion-Android](https://github.com/aayuxh-vim/Mewtion-Android)** companion app.

2. **Tunnel (`src/tcp.rs` — Network / ADB)**
   When using the Android companion app, the connection works over local Wi-Fi, a mobile hotspot, or a physical USB cable using `adb forward tcp:8765 tcp:8765`. Sensor readings stream over a lightweight TCP socket on port `8765`.

3. **Overlay & Control Panel (`Mewtion.exe` & `control_panel.exe`)**
   - `Mewtion.exe` renders the click-through, always-on-top Win32 layered overlay at 60 FPS and places an icon in the Windows System Tray.
   - `control_panel.exe` provides a dark-themed `iced` GUI that automatically launches `Mewtion.exe`, displays live sensor/connection status, lets you test motion cues at your desk, calibrates sensor orientation, and writes live updates to `mewtion_config.txt`.

---

## Installation & Quick Start (Windows 10 / 11 — x64 & ARM64)

### Option A: Windows Setup Installer (`.exe` — Recommended)

1. Download from **[Releases](https://github.com/Satya-Siba-Nayak/Mewtion-win/releases)**:
   - **Intel / AMD (64-bit):** `Mewtion-Setup-*-windows-x86_64.exe`
   - **Snapdragon / Windows on ARM:** `Mewtion-Setup-*-windows-arm64.exe`
2. Run the installer to add **Mewtion Settings** and **Mewtion Overlay** to your Start Menu (and optionally enable **Start Mewtion automatically when Windows starts**).

### Option B: Portable Archive (`.zip` — No Installation Needed)

1. Download `Mewtion-*-windows-x86_64.zip` (or `Mewtion-*-windows-arm64.zip`) and extract it to any folder.
2. Double-click **`control_panel.exe`** (or **`start.bat`**).
   - Opening `control_panel.exe` automatically starts `Mewtion.exe` in the background and sets up ADB port forwarding if `adb` is installed.
   - Right-click the **Mewtion icon in the Windows System Tray** anytime to pause motion cues, toggle **Simulate Motion (Desk Test)**, open Settings, or exit.

---

## Building from Source on Windows

### Prerequisites

* **Windows 10 or Windows 11** (`x86_64` or `ARM64`)
* **Rust & Cargo** (install via [rustup.rs](https://rustup.rs) with Visual Studio C++ Build Tools)
  * For `x86_64`: `rustup target add x86_64-pc-windows-msvc`
  * For `ARM64` (native or cross-compiled): `rustup target add aarch64-pc-windows-msvc`

### Build & Run

**Automated (1-Click Dev Script):**
```cmd
run.bat
```

**Manual Cargo Commands (`x86_64`):**
```powershell
cargo build --release --target x86_64-pc-windows-msvc
```

**Cross-Compile for Windows on ARM (`arm64`):**
```powershell
cargo build --release --target aarch64-pc-windows-msvc
```

---

## Features & Settings Guide

- **Simulate Motion (Desk Test Mode):** Check **Simulate Motion** in the Control Panel (or from the System Tray menu) to generate smooth synthetic figure-8 vehicle acceleration so you can preview and tune dot size, color, opacity, and sensitivity while stationary at your desk.
- **Display Target (`Primary` vs. `All Monitors`):** Choose whether motion cues appear along the edges of your primary screen or span your entire multi-monitor virtual desktop.
- **Sensor Orientation & Calibration:** Rotate sensor axes by `0°`, `90°`, `180°`, or `270°` and optionally invert Lateral (X) or Forward (Y) axes when your phone or tablet is mounted sideways or upside-down.
- **Run on Windows Startup:** Toggle directly inside the Control Panel to register/unregister `Mewtion.exe` under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.

---

## Troubleshooting

### The overlay does not react to my laptop's movement
Most standard clamshell laptops and desktop PCs do not include a hardware accelerometer/IMU in the chassis (whereas 2-in-1 convertibles, Surface devices, and Windows on ARM tablets usually do). Check the **Sensor & Connection Status** banner at the top of the Control Panel:
- If it says `Active: Built-in WinRT accelerometer...`, your built-in sensor is active.
- If it says `Waiting for companion app at ...`, use the **[Mewtion-Android](https://github.com/aayuxh-vim/Mewtion-Android)** app on your phone via USB or Wi-Fi/Hotspot, or enable **Simulate Motion (Desk Test Mode)** to test the overlay right away.

### Cannot connect to phone via Wi-Fi or Hotspot
1. Ensure your phone's screen is on and the **Mewtion-Android** app is actively running.
2. Connect your Windows PC to your phone's Mobile Hotspot, click **Auto-Detect Hotspot** in the Mewtion Control Panel, and click **Save & Apply**.
3. If using a shared Wi-Fi router, enter the IP address shown inside the Mewtion-Android app and ensure your Wi-Fi network is set to **Private** in Windows Network Settings so Windows Firewall does not block local peer-to-peer connections.

### Cannot connect via USB (`127.0.0.1`)
1. Enable **USB Debugging** on your Android phone and connect it with a USB data cable.
2. Verify `adb devices` shows your device as `device` (authorized).
3. Click **USB Mode** in the Control Panel (which automatically runs `adb forward tcp:8765 tcp:8765`), or run manually in PowerShell/Command Prompt:
   ```cmd
   adb forward tcp:8765 tcp:8765
   ```

### Fullscreen Games
The Windows overlay uses `WS_EX_TOPMOST` + `UpdateLayeredWindow`, which renders over the Windows desktop and **Borderless Windowed** games. For games running in legacy Exclusive Fullscreen mode, switch the game's display mode to **Borderless Windowed** so Windows DWM can composite the motion cues over the game.

---

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.
