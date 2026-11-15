# Mewtion for Windows (`Mewtion-win`)

A native **Windows 10 / 11** port of **[Mewtion](https://github.com/aayuxh-vim/Mewtion)** — an open-source desktop implementation inspired by Apple's **Vehicle Motion Cues**.

Mewtion helps reduce motion sickness when working on your laptop or Windows handheld/tablet in a moving vehicle. It creates a transparent, click-through, always-on-top overlay of dots along the edges of your screen. Using real-time accelerometer and gyroscope data, the dots drift in the opposite direction of the vehicle's movement, helping resolve the sensory conflict between your stationary screen and the physical motion your inner ear feels.

## Demo

https://github.com/user-attachments/assets/1a454867-f16b-4b72-ab93-a5af378831a5

---

## Linux Port vs. Windows Port: What's Carried Over & What's New

This repository (`Mewtion-win`) ports the original Linux/Wayland project to run natively on Windows with **zero external DLL dependencies** (no GTK4 or Wayland runtime needed).

| Component | Carried Over from Linux (`Mewtion`) | Re-Engineered for Windows (`Mewtion-win`) |
| :--- | :--- | :--- |
| **Particle Physics & Sensor Fusion** | **100% preserved** (`src/main.rs`) — identical 60 FPS particle simulation, `Fluid` vs. `Rigid` animation modes, stationary anchor dots, software deadband (noise gate), and lateral acceleration + gyroscope yaw (`gz`) fusion. | Driven by a native Win32 frame timer (`WM_TIMER` at ~60 FPS) with automatic display resolution tracking (`WM_DISPLAYCHANGE`). |
| **Overlay Window & Click-Through** | Inspired by Wayland Layer Shell's top-layer + empty Cairo input region behavior. | Replaced `gtk4` + `gtk4-layer-shell` with a native **Win32 Layered Window** (`WS_EX_LAYERED \| WS_EX_TRANSPARENT \| WS_EX_TOPMOST \| WS_EX_TOOLWINDOW \| WS_EX_NOACTIVATE`) and Per-Monitor V2 High-DPI awareness. Mouse clicks pass straight through at the OS level, and the overlay stays hidden from the Taskbar and `Alt+Tab`. |
| **2D Dot Renderer (`src/dotview.rs`)** | Preserves exact dot coordinates, sizes, RGB colors, and sine-wave alpha fading. | Replaced GTK4/GSK snapshots with a **32-bit premultiplied BGRA DIB section** (`CreateDIBSection` + `UpdateLayeredWindow`) and a custom signed-distance anti-aliased circle rasterizer with dirty-rectangle clearing (<1% CPU usage, zero GPU context overhead). |
| **Built-in Device Sensors** | Preserves the 0.25 Hz first-order low-pass gravity estimator and automatic fallback architecture from `iio.rs`. | Replaced Linux `/sys/bus/iio/devices/` sysfs polling with **`src/win_sensor.rs`** using the Windows Runtime (**WinRT `Windows.Devices.Sensors`**) `Accelerometer` and `Gyrometer` APIs for Windows 2-in-1s, tablets, and handhelds. |
| **Android Companion Tunnel (`src/tcp.rs`)** | **100% preserved** — fully compatible with **[Mewtion-Android](https://github.com/aayuxh-vim/Mewtion-Android)** over USB (`adb forward tcp:8765 tcp:8765`), Wi-Fi, or Mobile Hotspot. | Unchanged wire protocol (`ax,ay,az,gx,gy,gz` over TCP port `8765`), so the existing Android app works out of the box. |
| **Control Panel (`src/bin/control_panel.rs`)** | **Preserved** `iced` GUI layout, live dot preview, color/size/opacity/margin/sensitivity controls, and live `mewtion_config.txt` hot-reloading. | Replaced Linux `sh`/`ip route` hotspot detection with silent Windows gateway detection (`Get-NetRoute` + `ipconfig` fallback), added automatic `Mewtion.exe` startup, and added a **Start / Stop Overlay** button directly in the header. |

*(Note: All original Linux-specific files — `src/iio.rs`, GTK4 `main.rs`/`dotview.rs`, `flake.nix`, and Linux packaging scripts — are stashed in the [`archive/`](archive/) directory for reference.)*

---

## Architecture

The system consists of three parts working together for ultra-low latency:

1. **Sensor (`src/win_sensor.rs` / Android Fallback)**
   Mewtion first checks for a built-in Windows accelerometer and gyrometer via the WinRT `Windows.Devices.Sensors` API (supported on Surface devices, 2-in-1 convertibles, and Windows handhelds with an integrated HID Sensor Hub). It converts $g$-force readings to $\text{m/s}^2$ and subtracts gravity using a 0.25 Hz low-pass filter. If no built-in accelerometer is found, it automatically falls back to the **[Mewtion-Android](https://github.com/aayuxh-vim/Mewtion-Android)** companion app.

2. **Tunnel (`src/tcp.rs` — Network / ADB)**
   When using the Android companion app, the connection works over local Wi-Fi, a mobile hotspot, or a physical USB cable using `adb forward tcp:8765 tcp:8765`. Sensor readings stream over a lightweight TCP socket on port `8765`.

3. **Overlay & Control Panel (`Mewtion.exe` & `control_panel.exe`)**
   - `Mewtion.exe` renders the click-through, always-on-top Win32 layered overlay at 60 FPS.
   - `control_panel.exe` provides a dark-themed `iced` GUI that automatically launches `Mewtion.exe`, lets you start/stop the overlay anytime, auto-detects your phone hotspot IP, and writes live updates to `mewtion_config.txt`.

---

## Installation & Quick Start (Windows 10 / 11)

### 1. Pre-built Portable Package (Recommended)

1. Download `Mewtion-*-windows-x86_64.zip` from the **Releases** page (or GitHub Actions artifacts) and extract it to any folder.
2. Double-click **`control_panel.exe`** (or **`start.bat`**).
   - Opening `control_panel.exe` automatically starts `Mewtion.exe` in the background and sets up ADB port forwarding if `adb` is installed.
   - You can toggle the overlay on or off anytime using the **Start / Stop Overlay** button in the top-right corner of the Control Panel, or close `start.bat` to exit both.

---

### 2. Building from Source on Windows

#### Prerequisites

* **Windows 10 or Windows 11** (`x86_64`)
* **Rust & Cargo** (install via [rustup.rs](https://rustup.rs) with the default `x86_64-pc-windows-msvc` toolchain and Visual Studio C++ Build Tools)
* **Sensor Input:**
  * Built-in Windows laptop/tablet accelerometer (detected automatically via `Windows.Devices.Sensors`), **OR**
  * Android phone running **[Mewtion-Android](https://github.com/aayuxh-vim/Mewtion-Android)** over Wi-Fi/Hotspot or USB (with [Android Platform Tools (`adb`)](https://developer.android.com/tools/releases/platform-tools) in your `PATH` for USB mode).

#### Build & Run

**Automated (1-Click Dev Script):**
```cmd
run.bat
```
`run.bat` checks for connected ADB devices, forwards port `8765` if plugged in, compiles release binaries (`Mewtion.exe` and `control_panel.exe`), and launches both.

**Manual Cargo Commands:**
```powershell
cargo build --release
Start-Process .\target\release\Mewtion.exe
.\target\release\control_panel.exe
```

---

## Troubleshooting

### The overlay does not react to my laptop's movement
Most standard clamshell laptops and desktop PCs do not include a hardware accelerometer/IMU in the chassis. Check if Windows detects a motion sensor on your device in **Settings $\rightarrow$ Privacy & security $\rightarrow$ Motion** (or **Sensors** in Device Manager). If your laptop has no built-in sensor, use the **[Mewtion-Android](https://github.com/aayuxh-vim/Mewtion-Android)** app on your phone via USB or Wi-Fi/Hotspot.

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
