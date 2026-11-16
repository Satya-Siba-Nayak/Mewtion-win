#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use iced::font::{Family, Weight};
use iced::widget::{button, column, container, row, slider, text, text_input, toggler};
use iced::window;
use iced::{Alignment, Background, Border, Color, Element, Font, Length, Size, Task, Theme};
use std::collections::HashSet;
use std::fs;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const FONT_SEGOE: Font = Font::with_name("Segoe UI Variable Text");
const FONT_SEGOE_SEMIBOLD: Font = Font {
    family: Family::Name("Segoe UI Variable Display"),
    weight: Weight::Semibold,
    ..Font::DEFAULT
};

// Refined Windows 11 Fluent / WinUI 3 Dark Palette (PowerToys / Windows 11 Settings style)
const BG_SIDEBAR: Color = Color::from_rgb(0.094, 0.094, 0.110); // #18181C Mica base
const BG_CONTENT: Color = Color::from_rgb(0.118, 0.118, 0.137); // #1E1E23 Main workspace
const BG_CARD: Color = Color::from_rgb(0.153, 0.153, 0.176); // #27272D SettingsCard surface
const BG_CONTROL: Color = Color::from_rgb(0.192, 0.192, 0.220); // #313138 Inputs & segmented pills
const BG_CONTROL_HOVER: Color = Color::from_rgb(0.235, 0.235, 0.267); // #3C3C44 Hover state
const BG_NAV_ACTIVE: Color = Color::from_rgb(0.176, 0.176, 0.208); // #2D2D35 Sidebar active item

const BORDER_CARD: Color = Color::from_rgb(0.208, 0.208, 0.239); // #35353D Subtle 1px card stroke
const BORDER_CONTROL: Color = Color::from_rgb(0.263, 0.263, 0.302); // #43434D Input/Button stroke

const TEXT_PRIMARY: Color = Color::from_rgb(0.957, 0.957, 0.969); // #F4F4F7 Crisp primary text
const TEXT_SECONDARY: Color = Color::from_rgb(0.635, 0.639, 0.682); // #A2A3AE Muted subtitle text
const TEXT_MUTED: Color = Color::from_rgb(0.475, 0.478, 0.525); // #797A86 Subtle caption

const ACCENT_BLUE: Color = Color::from_rgb(0.376, 0.804, 1.000); // #60CDFF Win11 Dark Accent
const ACCENT_BLUE_HOVER: Color = Color::from_rgb(0.475, 0.843, 1.000);
const ACCENT_DARK_TEXT: Color = Color::from_rgb(0.047, 0.075, 0.118);

const STATUS_GREEN: Color = Color::from_rgb(0.247, 0.804, 0.439);
const STATUS_AMBER: Color = Color::from_rgb(0.988, 0.725, 0.192);
const STATUS_RED: Color = Color::from_rgb(0.969, 0.349, 0.349);

const COLOR_WHITE: Color = Color::from_rgb(1.00, 1.00, 1.00);
const COLOR_BLUE: Color = Color::from_rgb(0.00, 0.48, 1.00);
const COLOR_GREEN: Color = Color::from_rgb(0.20, 0.78, 0.35);
const COLOR_RED: Color = Color::from_rgb(1.00, 0.27, 0.23);
const COLOR_YELLOW: Color = Color::from_rgb(1.00, 0.80, 0.00);
const COLOR_PURPLE: Color = Color::from_rgb(0.69, 0.32, 0.87);
const COLOR_CYAN: Color = Color::from_rgb(0.00, 0.78, 0.75);

pub fn main() -> iced::Result {
    ensure_adb_and_overlay_started();

    let mut win_settings = window::Settings {
        size: Size::new(780.0, 540.0),
        min_size: Some(Size::new(720.0, 500.0)),
        position: window::Position::Centered,
        icon: build_window_icon(),
        ..window::Settings::default()
    };

    #[cfg(target_os = "windows")]
    {
        win_settings.platform_specific.corner_preference =
            window::settings::platform::CornerPreference::Round;
    }

    iced::application(SettingsApp::default, SettingsApp::update, SettingsApp::view)
        .title("Mewtion Settings")
        .theme(|_state: &SettingsApp| Theme::Dark)
        .default_font(FONT_SEGOE)
        .antialiasing(true)
        .window(win_settings)
        .run()
}

fn build_window_icon() -> Option<window::Icon> {
    let size = 32u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let dx = (fx - 16.0).abs() - 9.0;
            let dy = (fy - 16.0).abs() - 9.0;
            let dist =
                (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt() + dx.max(dy).min(0.0) - 5.0;
            let idx = ((y * size + x) * 4) as usize;
            if dist <= 0.5 {
                let alpha = (0.5 - dist).clamp(0.0, 1.0);
                rgba[idx] = 0x1E;
                rgba[idx + 1] = 0x20;
                rgba[idx + 2] = 0x28;
                rgba[idx + 3] = (alpha * 255.0) as u8;

                let dots = [
                    (8.0f32, 6.0f32),
                    (16.0, 6.0),
                    (24.0, 6.0),
                    (8.0, 26.0),
                    (16.0, 26.0),
                    (24.0, 26.0),
                    (6.0, 16.0),
                    (26.0, 16.0),
                    (16.0, 16.0),
                ];
                for &(cx, cy) in &dots {
                    let d = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt();
                    if d <= 2.3 {
                        rgba[idx] = 0x60;
                        rgba[idx + 1] = 0xCD;
                        rgba[idx + 2] = 0xFF;
                        rgba[idx + 3] = 255;
                    }
                }
            }
        }
    }
    window::icon::from_rgba(rgba, size, size).ok()
}

#[cfg(windows)]
fn apply_win11_dwm_chrome() {
    std::thread::spawn(|| {
        use windows::core::w;
        use windows::Win32::Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_CAPTION_COLOR, DWMWA_SYSTEMBACKDROP_TYPE,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
        };
        use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

        for _ in 0..25 {
            std::thread::sleep(Duration::from_millis(60));
            unsafe {
                if let Ok(hwnd) = FindWindowW(None, w!("Mewtion Settings")) {
                    if !hwnd.0.is_null() {
                        let dark_mode: i32 = 1;
                        let _ = DwmSetWindowAttribute(
                            hwnd,
                            DWMWA_USE_IMMERSIVE_DARK_MODE,
                            &dark_mode as *const _ as *const _,
                            std::mem::size_of::<i32>() as u32,
                        );
                        // COLORREF 0x00BBGGRR for #18181C -> 0x001C1818
                        let caption_color: u32 = 0x001C_1818;
                        let _ = DwmSetWindowAttribute(
                            hwnd,
                            DWMWA_CAPTION_COLOR,
                            &caption_color as *const _ as *const _,
                            std::mem::size_of::<u32>() as u32,
                        );
                        let mica_backdrop: i32 = 2;
                        let _ = DwmSetWindowAttribute(
                            hwnd,
                            DWMWA_SYSTEMBACKDROP_TYPE,
                            &mica_backdrop as *const _ as *const _,
                            std::mem::size_of::<i32>() as u32,
                        );
                        break;
                    }
                }
            }
        }
    });
}

#[cfg(not(windows))]
fn apply_win11_dwm_chrome() {}

fn sanitize_ip_input(raw: &str) -> String {
    let trimmed = raw.trim();
    let without_scheme = trimmed
        .strip_prefix("tcp://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .or_else(|| trimmed.strip_prefix("https://"))
        .unwrap_or(trimmed)
        .trim()
        .trim_end_matches('/');

    let host = if let Some((h, _port)) = without_scheme.rsplit_once(':') {
        if !h.contains(':') {
            h.trim()
        } else {
            without_scheme
        }
    } else {
        without_scheme
    };

    if host.is_empty() {
        "127.0.0.1".to_string()
    } else {
        host.to_string()
    }
}

/// Always uses `%LOCALAPPDATA%\Mewtion` so config and status writes never fail with
/// `PermissionDenied` when installed under `C:\Program Files\Mewtion`.
fn app_data_dir() -> PathBuf {
    if let Ok(local_app_data) =
        std::env::var("LOCALAPPDATA").or_else(|_| std::env::var("APPDATA"))
    {
        let dir = PathBuf::from(local_app_data).join("Mewtion");
        if fs::create_dir_all(&dir).is_ok() {
            return dir;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            return dir.to_path_buf();
        }
    }
    PathBuf::from(".")
}

fn resolve_config_path() -> PathBuf {
    let cfg_path = app_data_dir().join("mewtion_config.txt");
    if !cfg_path.exists() {
        let seed_candidates = [
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("mewtion_config.txt"))),
            Some(PathBuf::from("mewtion_config.txt")),
        ];
        for cand in seed_candidates.into_iter().flatten() {
            if cand.exists() && cand != cfg_path {
                let _ = fs::copy(&cand, &cfg_path);
                break;
            }
        }
    }
    cfg_path
}

fn resolve_status_path() -> PathBuf {
    app_data_dir().join("mewtion_status.txt")
}

fn read_sensor_status() -> String {
    fs::read_to_string(resolve_status_path())
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Waiting for sensor stream...".to_string())
}

fn new_silent_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Instantaneous Win32 window check (< 1 microsecond, zero process spawns!)
#[cfg(windows)]
fn is_overlay_running() -> bool {
    use windows::core::w;
    use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
    unsafe {
        if let Ok(hwnd) = FindWindowW(w!("MewtionOverlayWindowClass"), None) {
            return !hwnd.0.is_null();
        }
    }
    false
}

#[cfg(not(windows))]
fn is_overlay_running() -> bool {
    false
}

#[cfg(windows)]
fn stop_overlay_process() {
    use windows::core::w;
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW, WM_CLOSE};
    unsafe {
        if let Ok(hwnd) = FindWindowW(w!("MewtionOverlayWindowClass"), None) {
            if !hwnd.0.is_null() {
                let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
                return;
            }
        }
    }
    let _ = new_silent_command("taskkill")
        .args(["/F", "/IM", "Mewtion.exe"])
        .output();
}

#[cfg(not(windows))]
fn stop_overlay_process() {}

fn find_overlay_exe() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join("Mewtion.exe");
            if sibling.exists() {
                return Some(sibling);
            }
        }
    }
    let cwd_exe = PathBuf::from("Mewtion.exe");
    if cwd_exe.exists() {
        return Some(cwd_exe);
    }
    let release_exe = PathBuf::from("target/release/Mewtion.exe");
    if release_exe.exists() {
        return Some(release_exe);
    }
    None
}

fn ensure_overlay_running() -> bool {
    if is_overlay_running() {
        return true;
    }
    if let Some(overlay_path) = find_overlay_exe() {
        let mut cmd = new_silent_command(overlay_path.to_str().unwrap_or("Mewtion.exe"));
        if let Some(parent) = overlay_path.parent() {
            cmd.current_dir(parent);
        }
        return cmd.spawn().is_ok();
    }
    false
}

fn find_adb_executable() -> String {
    let mut candidates = vec![PathBuf::from("adb.exe"), PathBuf::from("adb")];
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        candidates.push(
            PathBuf::from(&local)
                .join("Android")
                .join("Sdk")
                .join("platform-tools")
                .join("adb.exe"),
        );
        candidates.push(
            PathBuf::from(&local)
                .join("Microsoft")
                .join("WinGet")
                .join("Links")
                .join("adb.exe"),
        );
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        candidates.push(
            PathBuf::from(&profile)
                .join("Downloads")
                .join("platform-tools")
                .join("adb.exe"),
        );
        candidates.push(
            PathBuf::from(&profile)
                .join("Downloads")
                .join("platform-tools-latest-windows")
                .join("platform-tools")
                .join("adb.exe"),
        );
        candidates.push(
            PathBuf::from(&profile)
                .join("Desktop")
                .join("platform-tools")
                .join("adb.exe"),
        );
    }
    candidates.push(PathBuf::from(r"C:\platform-tools\adb.exe"));

    for cand in candidates {
        if cand.is_absolute() && cand.exists() {
            return cand.to_string_lossy().to_string();
        }
    }
    "adb".to_string()
}

fn run_adb_forward() -> Result<String, String> {
    let adb = find_adb_executable();
    match new_silent_command(&adb)
        .args(["forward", "tcp:8765", "tcp:8765"])
        .output()
    {
        Ok(out) => {
            if out.status.success() {
                Ok("USB ADB port 8765 forwarded — connecting to 127.0.0.1:8765".to_string())
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
                if stderr.is_empty() {
                    Err("ADB returned an error — check that USB Debugging is allowed on your phone".to_string())
                } else {
                    Err(format!("ADB: {stderr}"))
                }
            }
        }
        Err(_) => Err(
            "ADB not found on PC — open Setup Guide → USB Cable to download ADB Tools".to_string(),
        ),
    }
}

fn ensure_adb_and_overlay_started() {
    apply_win11_dwm_chrome();
    // Immediately start Mewtion.exe before doing anything else so the overlay is always active!
    ensure_overlay_running();

    std::thread::spawn(|| {
        let _ = run_adb_forward();
    });
}

fn open_external_url(url: &str) {
    let url_owned = url.to_string();
    std::thread::spawn(move || {
        #[cfg(windows)]
        {
            let _ = new_silent_command("cmd")
                .args(["/C", "start", "", &url_owned])
                .spawn();
        }
        #[cfg(not(windows))]
        {
            let _ = Command::new("xdg-open").arg(&url_owned).spawn();
        }
    });
}

fn is_autostart_enabled() -> bool {
    if let Ok(output) = new_silent_command("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
            "/v",
            "Mewtion",
        ])
        .output()
    {
        return output.status.success();
    }
    false
}

fn set_autostart_enabled(enabled: bool) -> bool {
    if enabled {
        let Some(overlay_path) = find_overlay_exe() else {
            return false;
        };
        let canonical = fs::canonicalize(&overlay_path).unwrap_or(overlay_path);
        let value = format!("\"{}\"", canonical.display());
        new_silent_command("reg")
            .args([
                "add",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "Mewtion",
                "/t",
                "REG_SZ",
                "/d",
                &value,
                "/f",
            ])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        new_silent_command("reg")
            .args([
                "delete",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "Mewtion",
                "/f",
            ])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

fn extract_ipv4_tokens(text: &str) -> Vec<Ipv4Addr> {
    let mut out = Vec::new();
    for token in text.split(|c: char| !c.is_ascii_digit() && c != '.') {
        if token.chars().filter(|&c| c == '.').count() == 3 {
            if let Ok(ip) = token.parse::<Ipv4Addr>() {
                if !ip.is_unspecified() && !ip.is_broadcast() && ip.octets()[0] != 255 {
                    out.push(ip);
                }
            }
        }
    }
    out
}

fn probe_port_8765(ip: Ipv4Addr, timeout_ms: u64) -> bool {
    let addr = SocketAddr::V4(SocketAddrV4::new(ip, 8765));
    TcpStream::connect_timeout(&addr, Duration::from_millis(timeout_ms)).is_ok()
}

/// Smart discovery that checks:
/// 1. ADB USB (`127.0.0.1:8765`)
/// 2. All IPv4 Default Gateways (handles multi-line IPv6+IPv4 `ipconfig` output on Windows 11)
/// 3. All `arp -a` neighbor IPs on port 8765
/// 4. Parallel 32-thread scan of active private `/24` subnets for any phone running `Mewtion-Android` on port 8765!
fn discover_phone_on_network() -> (Option<String>, String) {
    let _ = run_adb_forward();
    if probe_port_8765(Ipv4Addr::new(127, 0, 0, 1), 180) {
        return (
            Some("127.0.0.1".to_string()),
            "Found phone over USB (127.0.0.1:8765) & connected!".to_string(),
        );
    }

    let mut priority_ips: Vec<Ipv4Addr> = Vec::new();
    let mut local_subnets: Vec<[u8; 3]> = Vec::new();
    let mut fallback_gateway: Option<Ipv4Addr> = None;

    if let Ok(output) = new_silent_command("ipconfig").output() {
        let text = String::from_utf8_lossy(&output.stdout);
        let mut in_gateway_section = false;
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.contains("Default Gateway") {
                in_gateway_section = true;
                for ip in extract_ipv4_tokens(trimmed) {
                    if !ip.is_loopback() {
                        priority_ips.push(ip);
                        if fallback_gateway.is_none() {
                            fallback_gateway = Some(ip);
                        }
                    }
                }
            } else if in_gateway_section {
                if trimmed.contains(':') && !trimmed.starts_with("fe80") {
                    in_gateway_section = false;
                } else {
                    for ip in extract_ipv4_tokens(trimmed) {
                        if !ip.is_loopback() {
                            priority_ips.push(ip);
                            if fallback_gateway.is_none() {
                                fallback_gateway = Some(ip);
                            }
                        }
                    }
                }
            }

            if trimmed.contains("IPv4 Address") || trimmed.contains("IP Address") {
                for ip in extract_ipv4_tokens(trimmed) {
                    let o = ip.octets();
                    if o[0] == 192 || o[0] == 10 || o[0] == 172 {
                        let subnet = [o[0], o[1], o[2]];
                        if !local_subnets.contains(&subnet) {
                            local_subnets.push(subnet);
                        }
                        // Also add .1 of this subnet as a likely hotspot gateway
                        let gw = Ipv4Addr::new(o[0], o[1], o[2], 1);
                        if !priority_ips.contains(&gw) {
                            priority_ips.push(gw);
                        }
                    }
                }
            }
        }
    }

    if let Ok(output) = new_silent_command("arp").arg("-a").output() {
        let text = String::from_utf8_lossy(&output.stdout);
        for ip in extract_ipv4_tokens(&text) {
            let o = ip.octets();
            if (o[0] == 192 || o[0] == 10 || o[0] == 172) && o[3] != 255 && o[3] != 0 {
                if !priority_ips.contains(&ip) {
                    priority_ips.push(ip);
                }
            }
        }
    }

    // 1. Probe priority candidates (gateways + ARP neighbors) in parallel
    let found_priority = Arc::new(Mutex::new(None::<Ipv4Addr>));
    let mut handles = Vec::new();
    for ip in priority_ips {
        let found_ref = found_priority.clone();
        handles.push(std::thread::spawn(move || {
            if probe_port_8765(ip, 250) {
                let mut guard = found_ref.lock().unwrap();
                if guard.is_none() {
                    *guard = Some(ip);
                }
            }
        }));
    }
    for h in handles {
        let _ = h.join();
    }
    if let Some(ip) = *found_priority.lock().unwrap() {
        return (
            Some(ip.to_string()),
            format!("Found phone at {ip}:8765 & connected!"),
        );
    }

    // 2. Scan active /24 subnet(s) in parallel (32 worker threads, 200ms timeout)
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    for s in local_subnets.iter().take(2) {
        for host in 1u8..=254u8 {
            let ip = Ipv4Addr::new(s[0], s[1], s[2], host);
            if seen.insert(ip) {
                candidates.push(ip);
            }
        }
    }

    if !candidates.is_empty() {
        let queue = Arc::new(Mutex::new(candidates));
        let found_scan = Arc::new(Mutex::new(None::<Ipv4Addr>));
        let mut workers = Vec::new();
        for _ in 0..32 {
            let q = queue.clone();
            let f = found_scan.clone();
            workers.push(std::thread::spawn(move || loop {
                if f.lock().unwrap().is_some() {
                    break;
                }
                let next_ip = {
                    let mut guard = q.lock().unwrap();
                    guard.pop()
                };
                let Some(ip) = next_ip else {
                    break;
                };
                if probe_port_8765(ip, 200) {
                    let mut guard = f.lock().unwrap();
                    if guard.is_none() {
                        *guard = Some(ip);
                    }
                    break;
                }
            }));
        }
        for w in workers {
            let _ = w.join();
        }
        if let Some(ip) = *found_scan.lock().unwrap() {
            return (
                Some(ip.to_string()),
                format!("Found phone on network at {ip}:8765 & connected!"),
            );
        }
    }

    if let Some(gw) = fallback_gateway {
        (
            Some(gw.to_string()),
            format!(
                "Set gateway {gw} — port 8765 did not respond yet (make sure Mewtion-Android is open)"
            ),
        )
    } else {
        (
            None,
            "No device with port 8765 open found — check that Mewtion-Android is running".to_string(),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavTab {
    Connection,
    Appearance,
    MotionAndDisplay,
    Calibration,
    Guide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuideMethod {
    Hotspot,
    Wifi,
    Usb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorSourcePref {
    Auto,
    Phone,
    BuiltIn,
}
impl SensorSourcePref {
    fn config_str(&self) -> &'static str {
        match self {
            SensorSourcePref::Auto => "Auto",
            SensorSourcePref::Phone => "Phone",
            SensorSourcePref::BuiltIn => "BuiltIn",
        }
    }
    fn from_str(s: &str) -> Self {
        if s.eq_ignore_ascii_case("Phone") {
            SensorSourcePref::Phone
        } else if s.eq_ignore_ascii_case("BuiltIn") {
            SensorSourcePref::BuiltIn
        } else {
            SensorSourcePref::Auto
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DotSize {
    Small,
    Medium,
    Large,
}
impl DotSize {
    fn pixels(&self) -> f32 {
        match self {
            DotSize::Small => 8.0,
            DotSize::Medium => 16.0,
            DotSize::Large => 24.0,
        }
    }
    fn label(&self) -> &'static str {
        match self {
            DotSize::Small => "Small (8px)",
            DotSize::Medium => "Medium (16px)",
            DotSize::Large => "Large (24px)",
        }
    }
    fn from_pixels(px: f32) -> Self {
        if px <= 11.0 {
            DotSize::Small
        } else if px >= 20.0 {
            DotSize::Large
        } else {
            DotSize::Medium
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DotColor {
    White,
    Blue,
    Green,
    Red,
    Yellow,
    Purple,
    Cyan,
}
impl DotColor {
    fn to_color(&self) -> Color {
        match self {
            DotColor::White => COLOR_WHITE,
            DotColor::Blue => COLOR_BLUE,
            DotColor::Green => COLOR_GREEN,
            DotColor::Red => COLOR_RED,
            DotColor::Yellow => COLOR_YELLOW,
            DotColor::Purple => COLOR_PURPLE,
            DotColor::Cyan => COLOR_CYAN,
        }
    }
    fn hex_string(&self) -> &'static str {
        match self {
            DotColor::White => "#ffffff",
            DotColor::Blue => "#007aff",
            DotColor::Green => "#34c759",
            DotColor::Red => "#ff3b30",
            DotColor::Yellow => "#ffcc00",
            DotColor::Purple => "#af52de",
            DotColor::Cyan => "#00c7be",
        }
    }
    fn from_hex(hex: &str) -> Self {
        match hex.trim().to_ascii_lowercase().as_str() {
            "#007aff" => DotColor::Blue,
            "#34c759" => DotColor::Green,
            "#ff3b30" => DotColor::Red,
            "#ffcc00" => DotColor::Yellow,
            "#af52de" => DotColor::Purple,
            "#00c7be" => DotColor::Cyan,
            _ => DotColor::White,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimMode {
    Fluid,
    Rigid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorTarget {
    Primary,
    All,
}
impl MonitorTarget {
    fn config_str(&self) -> &'static str {
        match self {
            MonitorTarget::Primary => "Primary",
            MonitorTarget::All => "All",
        }
    }
    fn from_str(s: &str) -> Self {
        if s.eq_ignore_ascii_case("All") {
            MonitorTarget::All
        } else {
            MonitorTarget::Primary
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Deg0,
    Deg90,
    Deg180,
    Deg270,
}
impl Orientation {
    fn degrees(&self) -> i32 {
        match self {
            Orientation::Deg0 => 0,
            Orientation::Deg90 => 90,
            Orientation::Deg180 => 180,
            Orientation::Deg270 => 270,
        }
    }
    fn from_degrees(deg: i32) -> Self {
        match deg.rem_euclid(360) {
            90 => Orientation::Deg90,
            180 => Orientation::Deg180,
            270 => Orientation::Deg270,
            _ => Orientation::Deg0,
        }
    }
}

struct SharedBgState {
    overlay_running: bool,
    sensor_status: String,
    discovered_ip: Option<String>,
    pending_toast: Option<String>,
}

struct SettingsApp {
    active_tab: NavTab,
    guide_method: GuideMethod,
    selected_size: DotSize,
    selected_color: DotColor,
    opacity: f32,
    margin_pct: f32,
    sensitivity: f32,
    anim_mode: AnimMode,
    monitor_target: MonitorTarget,
    sensor_source: SensorSourcePref,
    orientation: Orientation,
    invert_x: bool,
    invert_y: bool,
    simulate_motion: bool,
    autostart_windows: bool,
    phone_ip: String,
    overlay_running: bool,
    sensor_status: String,
    status_message: String,
    bg_state: Arc<Mutex<SharedBgState>>,
}

impl Default for SettingsApp {
    fn default() -> Self {
        let path = resolve_config_path();
        let initial_running = is_overlay_running();
        let initial_status = read_sensor_status();
        let bg_state = Arc::new(Mutex::new(SharedBgState {
            overlay_running: initial_running,
            sensor_status: initial_status.clone(),
            discovered_ip: None,
            pending_toast: None,
        }));
        let bg_clone = bg_state.clone();

        std::thread::spawn(move || loop {
            let running = is_overlay_running();
            let status_txt = read_sensor_status();
            if let Ok(mut guard) = bg_clone.lock() {
                guard.overlay_running = running;
                guard.sensor_status = status_txt;
            }
            std::thread::sleep(Duration::from_millis(600));
        });

        let mut app = Self {
            active_tab: NavTab::Connection,
            guide_method: GuideMethod::Hotspot,
            selected_size: DotSize::Medium,
            selected_color: DotColor::White,
            opacity: 0.85,
            margin_pct: 0.03,
            sensitivity: 3.5,
            anim_mode: AnimMode::Fluid,
            monitor_target: MonitorTarget::Primary,
            sensor_source: SensorSourcePref::Auto,
            orientation: Orientation::Deg0,
            invert_x: false,
            invert_y: false,
            simulate_motion: false,
            autostart_windows: is_autostart_enabled(),
            phone_ip: "127.0.0.1".to_string(),
            overlay_running: initial_running,
            sensor_status: initial_status,
            status_message: "Changes apply automatically in real time".to_string(),
            bg_state,
        };

        if let Ok(contents) = fs::read_to_string(path) {
            for line in contents.lines() {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() != 2 {
                    continue;
                }
                let (k, v) = (parts[0].trim(), parts[1].trim());
                match k {
                    "size" => {
                        if let Ok(val) = v.parse::<f32>() {
                            app.selected_size = DotSize::from_pixels(val);
                        }
                    }
                    "color" => app.selected_color = DotColor::from_hex(v),
                    "opacity" => {
                        if let Ok(val) = v.parse::<f32>() {
                            app.opacity = val;
                        }
                    }
                    "margin" => {
                        if let Ok(val) = v.parse::<f32>() {
                            app.margin_pct = val;
                        }
                    }
                    "sensitivity" => {
                        if let Ok(val) = v.parse::<f32>() {
                            app.sensitivity = val;
                        }
                    }
                    "animation" => {
                        app.anim_mode = if v.eq_ignore_ascii_case("Rigid") {
                            AnimMode::Rigid
                        } else {
                            AnimMode::Fluid
                        };
                    }
                    "monitor" => app.monitor_target = MonitorTarget::from_str(v),
                    "source" | "sensor_source" => {
                        app.sensor_source = SensorSourcePref::from_str(v)
                    }
                    "orientation" => {
                        if let Ok(val) = v.parse::<i32>() {
                            app.orientation = Orientation::from_degrees(val);
                        }
                    }
                    "invert_x" => app.invert_x = matches!(v, "true" | "1" | "yes"),
                    "invert_y" => app.invert_y = matches!(v, "true" | "1" | "yes"),
                    "simulate" => app.simulate_motion = matches!(v, "true" | "1" | "yes"),
                    "ip" => app.phone_ip = sanitize_ip_input(v),
                    _ => {}
                }
            }
        }
        // Write initial config to %LOCALAPPDATA%\Mewtion so Mewtion.exe and control_panel.exe are synced
        let _ = app.write_config();
        app
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(NavTab),
    GuideMethodSelected(GuideMethod),
    SizeSelected(DotSize),
    ColorSelected(DotColor),
    OpacityChanged(f32),
    MarginChanged(f32),
    SensitivityChanged(f32),
    AnimModeChanged(AnimMode),
    MonitorTargetChanged(MonitorTarget),
    SensorSourceChanged(SensorSourcePref),
    OrientationChanged(Orientation),
    InvertXChanged(bool),
    InvertYChanged(bool),
    SimulateMotionChanged(bool),
    AutostartChanged(bool),
    IpChanged(String),
    SetUsbMode,
    AutoDetectPhone,
    TestConnection,
    RefreshStatus,
    ToggleOverlay(bool),
    ResetDefaults,
    OpenUrl(&'static str),
}

impl SettingsApp {
    fn sync_background_status(&mut self) {
        let mut apply_discovered: Option<String> = None;
        if let Ok(mut guard) = self.bg_state.try_lock() {
            self.overlay_running = guard.overlay_running;
            self.sensor_status = guard.sensor_status.clone();
            if let Some(ip) = guard.discovered_ip.take() {
                apply_discovered = Some(ip);
            }
            if let Some(toast) = guard.pending_toast.take() {
                self.status_message = toast;
            }
        }
        if let Some(ip) = apply_discovered {
            self.phone_ip = ip;
            if self.sensor_source == SensorSourcePref::BuiltIn {
                self.sensor_source = SensorSourcePref::Auto;
            }
            let _ = self.write_config();
        }
    }

    fn write_config(&self) -> Result<(), std::io::Error> {
        let clean_ip = sanitize_ip_input(&self.phone_ip);
        let config_data = format!(
            "size={}\ncolor={}\nopacity={}\nmargin={}\nsensitivity={}\nanimation={}\nip={}\nmonitor={}\norientation={}\ninvert_x={}\ninvert_y={}\nsimulate={}\nsensor_source={}\n",
            self.selected_size.pixels(),
            self.selected_color.hex_string(),
            self.opacity,
            self.margin_pct,
            self.sensitivity,
            match self.anim_mode {
                AnimMode::Fluid => "Fluid",
                AnimMode::Rigid => "Rigid",
            },
            clean_ip,
            self.monitor_target.config_str(),
            self.orientation.degrees(),
            self.invert_x,
            self.invert_y,
            self.simulate_motion,
            self.sensor_source.config_str(),
        );
        fs::write(resolve_config_path(), config_data)
    }

    fn apply_live(&mut self, msg: &str) {
        self.status_message = msg.to_string();
        let _ = self.write_config();
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        self.sync_background_status();

        match message {
            Message::TabSelected(tab) => {
                self.active_tab = tab;
            }
            Message::GuideMethodSelected(m) => {
                self.guide_method = m;
            }
            Message::SizeSelected(size) => {
                self.selected_size = size;
                self.apply_live("Dot size applied live");
            }
            Message::ColorSelected(color) => {
                self.selected_color = color;
                self.apply_live("Dot color applied live");
            }
            Message::OpacityChanged(val) => {
                self.opacity = val;
                self.apply_live(&format!("Opacity set to {:.0}%", val * 100.0));
            }
            Message::MarginChanged(val) => {
                self.margin_pct = val;
                self.apply_live(&format!("Edge margin set to {:.0}%", val * 100.0));
            }
            Message::SensitivityChanged(val) => {
                self.sensitivity = val;
                self.apply_live(&format!("Sensitivity set to {:.1}", val));
            }
            Message::AnimModeChanged(mode) => {
                self.anim_mode = mode;
                self.apply_live("Animation style applied live");
            }
            Message::MonitorTargetChanged(target) => {
                self.monitor_target = target;
                self.apply_live("Display target applied live");
            }
            Message::SensorSourceChanged(src) => {
                self.sensor_source = src;
                self.apply_live("Sensor source updated live");
            }
            Message::OrientationChanged(orient) => {
                self.orientation = orient;
                self.apply_live("Sensor orientation updated live");
            }
            Message::InvertXChanged(val) => {
                self.invert_x = val;
                self.apply_live("Lateral (X) axis inversion updated");
            }
            Message::InvertYChanged(val) => {
                self.invert_y = val;
                self.apply_live("Forward (Y) axis inversion updated");
            }
            Message::SimulateMotionChanged(val) => {
                if val && !self.overlay_running {
                    ensure_overlay_running();
                    self.overlay_running = true;
                }
                self.simulate_motion = val;
                self.apply_live(if val {
                    "Desk Test active — simulating vehicle motion"
                } else {
                    "Desk Test stopped immediately"
                });
            }
            Message::AutostartChanged(val) => {
                if set_autostart_enabled(val) {
                    self.autostart_windows = val;
                    self.status_message = if val {
                        "Enabled Mewtion on Windows Startup".to_string()
                    } else {
                        "Disabled Mewtion on Windows Startup".to_string()
                    };
                } else {
                    self.status_message = "Could not update Windows Startup registry".to_string();
                }
            }
            Message::IpChanged(ip) => {
                self.phone_ip = ip;
                if self.sensor_source == SensorSourcePref::BuiltIn {
                    self.sensor_source = SensorSourcePref::Auto;
                }
                if !self.overlay_running {
                    ensure_overlay_running();
                    self.overlay_running = true;
                }
                let clean = sanitize_ip_input(&self.phone_ip);
                self.apply_live(&format!("Connecting live to {clean}:8765..."));
            }
            Message::SetUsbMode => {
                ensure_overlay_running();
                self.overlay_running = true;
                self.phone_ip = "127.0.0.1".to_string();
                self.sensor_source = SensorSourcePref::Auto;
                self.apply_live("Running ADB port forward for USB Mode (127.0.0.1:8765)...");
                let bg_clone = self.bg_state.clone();
                std::thread::spawn(move || {
                    let msg = match run_adb_forward() {
                        Ok(ok_msg) => ok_msg,
                        Err(err_msg) => err_msg,
                    };
                    if let Ok(mut guard) = bg_clone.lock() {
                        guard.pending_toast = Some(msg);
                    }
                });
            }
            Message::AutoDetectPhone => {
                ensure_overlay_running();
                self.overlay_running = true;
                if self.sensor_source == SensorSourcePref::BuiltIn {
                    self.sensor_source = SensorSourcePref::Auto;
                }
                self.status_message =
                    "Scanning Hotspot, Wi-Fi & USB for phone on port 8765...".to_string();
                let bg_clone = self.bg_state.clone();
                std::thread::spawn(move || {
                    let (found_ip, msg) = discover_phone_on_network();
                    if let Ok(mut guard) = bg_clone.lock() {
                        if let Some(ip) = found_ip {
                            guard.discovered_ip = Some(ip);
                        }
                        guard.pending_toast = Some(msg);
                    }
                });
            }
            Message::TestConnection => {
                ensure_overlay_running();
                self.overlay_running = true;
                let target = sanitize_ip_input(&self.phone_ip);
                self.status_message = format!("Testing TCP connection to {target}:8765...");
                let bg_clone = self.bg_state.clone();
                std::thread::spawn(move || {
                    let addr_str = format!("{target}:8765");
                    let result_msg = if let Ok(ip) = target.parse::<Ipv4Addr>() {
                        let sock = SocketAddr::V4(SocketAddrV4::new(ip, 8765));
                        match TcpStream::connect_timeout(&sock, Duration::from_millis(1200)) {
                            Ok(_) => format!(
                                "Success! {addr_str} is open & streaming sensor data"
                            ),
                            Err(e) => format!(
                                "Cannot reach {addr_str} ({}) — ensure Mewtion-Android is open",
                                e.kind()
                            ),
                        }
                    } else {
                        format!("Invalid IPv4 address: {target}")
                    };
                    if let Ok(mut guard) = bg_clone.lock() {
                        guard.pending_toast = Some(result_msg);
                    }
                });
            }
            Message::RefreshStatus => {
                self.overlay_running = is_overlay_running();
                self.sensor_status = read_sensor_status();
            }
            Message::ToggleOverlay(should_run) => {
                self.overlay_running = should_run;
                if !should_run {
                    stop_overlay_process();
                    self.status_message = "Overlay paused".to_string();
                } else {
                    ensure_overlay_running();
                    self.status_message = "Overlay active".to_string();
                }
            }
            Message::ResetDefaults => {
                self.selected_size = DotSize::Medium;
                self.selected_color = DotColor::White;
                self.opacity = 0.85;
                self.margin_pct = 0.03;
                self.sensitivity = 3.5;
                self.anim_mode = AnimMode::Fluid;
                self.monitor_target = MonitorTarget::Primary;
                self.sensor_source = SensorSourcePref::Auto;
                self.orientation = Orientation::Deg0;
                self.invert_x = false;
                self.invert_y = false;
                self.simulate_motion = false;
                self.apply_live("Restored default settings");
            }
            Message::OpenUrl(url) => {
                open_external_url(url);
                self.status_message = "Opened guide in default browser".to_string();
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        // === LEFT NAVIGATION SIDEBAR (WinUI 3 NavigationView Rail) ===
        let brand_icon = container(
            text("⋮⋮")
                .font(FONT_SEGOE_SEMIBOLD)
                .size(16)
                .color(ACCENT_BLUE),
        )
        .width(Length::Fixed(34.0))
        .height(Length::Fixed(34.0))
        .center_x(Length::Fixed(34.0))
        .center_y(Length::Fixed(34.0))
        .style(|_| container::Style {
            background: Some(BG_CARD.into()),
            border: Border {
                radius: 8.0.into(),
                width: 1.0,
                color: BORDER_CARD,
            },
            ..Default::default()
        });

        let brand_header = row![
            brand_icon,
            column![
                text("Mewtion")
                    .font(FONT_SEGOE_SEMIBOLD)
                    .size(17)
                    .color(TEXT_PRIMARY),
                text("Motion Cues")
                    .size(11)
                    .color(TEXT_SECONDARY),
            ]
            .spacing(1),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .padding([4, 6]);

        let nav_items = column![
            self.sidebar_item(NavTab::Connection, "Connection"),
            self.sidebar_item(NavTab::Appearance, "Appearance"),
            self.sidebar_item(NavTab::MotionAndDisplay, "Motion & Display"),
            self.sidebar_item(NavTab::Calibration, "Calibration"),
        ]
        .spacing(4);

        let is_connected = self.sensor_status.starts_with("Connected")
            || self.sensor_status.starts_with("Active:");
        let dot_color = if !self.overlay_running {
            STATUS_RED
        } else if is_connected {
            STATUS_GREEN
        } else {
            STATUS_AMBER
        };
        let short_state = if !self.overlay_running {
            "Overlay Off"
        } else if is_connected {
            "Connected"
        } else {
            "Waiting for Phone"
        };

        let status_dot = container(text(""))
            .width(Length::Fixed(8.0))
            .height(Length::Fixed(8.0))
            .style(move |_| container::Style {
                background: Some(dot_color.into()),
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        let status_pill = container(
            row![
                status_dot,
                text(short_state)
                    .size(11)
                    .color(TEXT_SECONDARY)
                    .width(Length::Fill),
                button(text("↻").size(11).color(TEXT_SECONDARY))
                    .on_press(Message::RefreshStatus)
                    .padding([2, 6])
                    .style(ghost_button_style),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([8, 10])
        .style(|_| container::Style {
            background: Some(BG_CONTENT.into()),
            border: Border {
                radius: 6.0.into(),
                width: 1.0,
                color: BORDER_CARD,
            },
            ..Default::default()
        });

        let sidebar = container(
            column![
                brand_header,
                container(text("")).height(Length::Fixed(8.0)),
                nav_items,
                container(text("")).height(Length::Fill),
                self.sidebar_item(NavTab::Guide, "?   Setup Guide"),
                status_pill,
            ]
            .spacing(8)
            .padding([18, 14]),
        )
        .width(Length::Fixed(205.0))
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(BG_SIDEBAR.into()),
            ..Default::default()
        });

        // === RIGHT MAIN CONTENT WORKSPACE ===
        let (page_title, page_subtitle) = match self.active_tab {
            NavTab::Connection => (
                "Connection",
                "Connect your Android phone over Hotspot, Wi-Fi, or USB",
            ),
            NavTab::Appearance => (
                "Appearance",
                "Customize the size, color, opacity, and screen placement of motion dots",
            ),
            NavTab::MotionAndDisplay => (
                "Motion & Display",
                "Tune acceleration sensitivity, physics style, and multi-monitor behavior",
            ),
            NavTab::Calibration => (
                "Calibration",
                "Align sensor axes with how your phone or device is positioned",
            ),
            NavTab::Guide => (
                "Connection Setup Guide",
                "Choose how you want to connect your Android phone to Windows",
            ),
        };

        let top_controls = row![
            button(
                row![
                    text(if self.simulate_motion { "■" } else { "▶" })
                        .size(10)
                        .color(if self.simulate_motion {
                            ACCENT_DARK_TEXT
                        } else {
                            ACCENT_BLUE
                        }),
                    text(if self.simulate_motion {
                        "Stop Desk Test"
                    } else {
                        "Desk Test"
                    })
                    .font(FONT_SEGOE_SEMIBOLD)
                    .size(12)
                    .color(if self.simulate_motion {
                        ACCENT_DARK_TEXT
                    } else {
                        TEXT_PRIMARY
                    }),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .on_press(Message::SimulateMotionChanged(!self.simulate_motion))
            .padding([6, 12])
            .style(if self.simulate_motion {
                accent_button_style
            } else {
                secondary_button_style
            }),
            container(
                row![
                    text(if self.overlay_running {
                        "Overlay On"
                    } else {
                        "Overlay Off"
                    })
                    .size(12)
                    .color(TEXT_PRIMARY),
                    toggler(self.overlay_running)
                        .on_toggle(Message::ToggleOverlay)
                        .size(19.0)
                        .style(fluent_toggler_style),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([5, 10])
            .style(|_| container::Style {
                background: Some(BG_CARD.into()),
                border: Border {
                    radius: 6.0.into(),
                    width: 1.0,
                    color: BORDER_CARD,
                },
                ..Default::default()
            }),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let page_header = row![
            column![
                text(page_title)
                    .font(FONT_SEGOE_SEMIBOLD)
                    .size(22)
                    .color(TEXT_PRIMARY),
                text(page_subtitle).size(12).color(TEXT_SECONDARY),
            ]
            .spacing(2)
            .width(Length::Fill),
            top_controls,
        ]
        .align_y(Alignment::Center);

        let page_body: Element<'_, Message> = match self.active_tab {
            NavTab::Connection => self.view_connection_page(),
            NavTab::Appearance => self.view_appearance_page(),
            NavTab::MotionAndDisplay => self.view_motion_page(),
            NavTab::Calibration => self.view_calibration_page(),
            NavTab::Guide => self.view_guide_page(),
        };

        let footer_bar = row![
            text(&self.status_message)
                .size(11)
                .color(TEXT_MUTED)
                .width(Length::Fill),
            button(text("Reset Defaults").size(11).color(TEXT_SECONDARY))
                .on_press(Message::ResetDefaults)
                .padding([4, 10])
                .style(ghost_button_style),
        ]
        .align_y(Alignment::Center);

        let content_pane = container(
            column![
                page_header,
                container(page_body).width(Length::Fill).height(Length::Fill),
                footer_bar,
            ]
            .spacing(14)
            .padding([20, 24]),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(BG_CONTENT.into()),
            border: Border {
                radius: 10.0.into(),
                width: 1.0,
                color: BORDER_CARD,
            },
            ..Default::default()
        });

        container(
            row![
                sidebar,
                container(content_pane)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding([8, 8]),
            ],
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(BG_SIDEBAR.into()),
            text_color: Some(TEXT_PRIMARY),
            ..Default::default()
        })
        .into()
    }

    fn view_connection_page(&self) -> Element<'_, Message> {
        let ip_input = text_input("192.168.43.1 or 127.0.0.1", &self.phone_ip)
            .on_input(Message::IpChanged)
            .padding([7, 12])
            .size(13)
            .width(Length::Fixed(190.0))
            .style(fluent_text_input_style);

        let ip_card = settings_card(
            column![
                row![
                    column![
                        text("Android Phone Address")
                            .font(FONT_SEGOE_SEMIBOLD)
                            .size(13)
                            .color(TEXT_PRIMARY),
                        text("Click 'Auto-Find Phone' to scan Wi-Fi, Hotspot & USB automatically")
                            .size(11)
                            .color(TEXT_SECONDARY),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                    button(
                        text("Auto-Find Phone")
                            .font(FONT_SEGOE_SEMIBOLD)
                            .size(12)
                            .color(ACCENT_DARK_TEXT)
                    )
                    .on_press(Message::AutoDetectPhone)
                    .padding([7, 14])
                    .style(accent_button_style),
                ]
                .align_y(Alignment::Center),
                row![
                    ip_input,
                    button(text("USB Mode (ADB)").size(12).color(TEXT_PRIMARY))
                        .on_press(Message::SetUsbMode)
                        .padding([7, 12])
                        .style(secondary_button_style),
                    button(text("Test Port 8765").size(12).color(TEXT_PRIMARY))
                        .on_press(Message::TestConnection)
                        .padding([7, 12])
                        .style(secondary_button_style),
                    button(text("Setup Guide →").size(12).color(ACCENT_BLUE))
                        .on_press(Message::TabSelected(NavTab::Guide))
                        .padding([7, 10])
                        .style(ghost_button_style),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(12)
            .into(),
        );

        let source_pills = segmented_container(
            row![
                self.source_pill(SensorSourcePref::Auto, "Auto"),
                self.source_pill(SensorSourcePref::Phone, "Phone Only"),
                self.source_pill(SensorSourcePref::BuiltIn, "PC Sensor"),
            ]
            .spacing(2)
            .into(),
        );

        let source_card = settings_row_card(
            "Sensor Input Source",
            "Auto uses your Android phone whenever connected, or falls back to PC sensors",
            source_pills,
        );

        let status_str = if self.overlay_running {
            self.sensor_status.as_str()
        } else {
            "Overlay is currently paused — toggle 'Overlay On' above to resume"
        };

        let diag_card = settings_row_card(
            "Live Bridge Status",
            status_str,
            button(text("Test Connection").size(12).color(TEXT_PRIMARY))
                .on_press(Message::TestConnection)
                .padding([6, 12])
                .style(secondary_button_style)
                .into(),
        );

        column![ip_card, source_card, diag_card].spacing(10).into()
    }

    fn view_appearance_page(&self) -> Element<'_, Message> {
        let dot_px = self.selected_size.pixels();
        let dot_color = Color {
            a: self.opacity,
            ..self.selected_color.to_color()
        };
        let make_dot = |sz: f32| {
            container(text(""))
                .width(Length::Fixed(sz))
                .height(Length::Fixed(sz))
                .style(move |_| container::Style {
                    background: Some(dot_color.into()),
                    border: Border {
                        radius: (sz / 2.0).into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
        };

        let size_pills = segmented_container(
            row![
                self.size_pill(DotSize::Small, "Small"),
                self.size_pill(DotSize::Medium, "Medium"),
                self.size_pill(DotSize::Large, "Large"),
            ]
            .spacing(2)
            .into(),
        );

        let size_card = settings_card(
            row![
                row![
                    make_dot(dot_px * 0.65),
                    make_dot(dot_px),
                    make_dot(dot_px * 0.65),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
                column![
                    text("Dot Size")
                        .font(FONT_SEGOE_SEMIBOLD)
                        .size(13)
                        .color(TEXT_PRIMARY),
                    text(format!(
                        "{} • {:.0}% Opacity • {}% Margin",
                        self.selected_size.label(),
                        self.opacity * 100.0,
                        (self.margin_pct * 100.0).round() as i32
                    ))
                    .size(11)
                    .color(TEXT_SECONDARY),
                ]
                .spacing(2)
                .width(Length::Fill),
                size_pills,
            ]
            .spacing(14)
            .align_y(Alignment::Center)
            .into(),
        );

        let swatches = row![
            self.color_swatch(DotColor::White),
            self.color_swatch(DotColor::Blue),
            self.color_swatch(DotColor::Cyan),
            self.color_swatch(DotColor::Green),
            self.color_swatch(DotColor::Yellow),
            self.color_swatch(DotColor::Purple),
            self.color_swatch(DotColor::Red),
        ]
        .spacing(8);

        let color_card = settings_row_card(
            "Dot Color",
            "Choose a high-contrast tint that stands out against your desktop",
            swatches.into(),
        );

        let opacity_card = slider_card(
            "Dot Opacity",
            "Transparency of the motion cues overlay",
            format!("{:.0}%", self.opacity * 100.0),
            slider(0.1..=1.0, self.opacity, Message::OpacityChanged)
                .step(0.05_f32)
                .width(Length::Fixed(180.0))
                .style(fluent_slider_style)
                .into(),
        );

        let margin_card = slider_card(
            "Screen Edge Margin",
            "Horizontal inset from the left and right edges of your screen",
            format!("{:.0}%", self.margin_pct * 100.0),
            slider(0.0..=0.15, self.margin_pct, Message::MarginChanged)
                .step(0.01_f32)
                .width(Length::Fixed(180.0))
                .style(fluent_slider_style)
                .into(),
        );

        column![size_card, color_card, opacity_card, margin_card]
            .spacing(10)
            .into()
    }

    fn view_motion_page(&self) -> Element<'_, Message> {
        let sens_card = slider_card(
            "Motion Sensitivity",
            "Lower values react more strongly to gentle turns and braking",
            format!("{:.1}", self.sensitivity),
            slider(1.0..=10.0, self.sensitivity, Message::SensitivityChanged)
                .step(0.5_f32)
                .width(Length::Fixed(180.0))
                .style(fluent_slider_style)
                .into(),
        );

        let anim_pills = segmented_container(
            row![
                self.anim_pill(AnimMode::Fluid, "Fluid"),
                self.anim_pill(AnimMode::Rigid, "Rigid"),
            ]
            .spacing(2)
            .into(),
        );

        let anim_card = settings_row_card(
            "Animation Style",
            "Fluid varies particle sizes organically; Rigid keeps uniform dots",
            anim_pills,
        );

        let mon_pills = segmented_container(
            row![
                self.monitor_pill(MonitorTarget::Primary, "Primary"),
                self.monitor_pill(MonitorTarget::All, "All Displays"),
            ]
            .spacing(2)
            .into(),
        );

        let mon_card = settings_row_card(
            "Display Target",
            "Render motion cues on your primary monitor or across all displays",
            mon_pills,
        );

        let startup_card = settings_row_card(
            "Launch on Windows Startup",
            "Automatically start the Mewtion overlay in the system tray when you sign in",
            toggler(self.autostart_windows)
                .on_toggle(Message::AutostartChanged)
                .size(20.0)
                .style(fluent_toggler_style)
                .into(),
        );

        column![sens_card, anim_card, mon_card, startup_card]
            .spacing(10)
            .into()
    }

    fn view_calibration_page(&self) -> Element<'_, Message> {
        let orient_pills = segmented_container(
            row![
                self.orient_pill(Orientation::Deg0, "0°"),
                self.orient_pill(Orientation::Deg90, "90°"),
                self.orient_pill(Orientation::Deg180, "180°"),
                self.orient_pill(Orientation::Deg270, "270°"),
            ]
            .spacing(2)
            .into(),
        );

        let orient_card = settings_row_card(
            "Phone / Sensor Orientation",
            "Rotate sensor axes if your phone rests sideways (90° / 270°) or inverted (180°)",
            orient_pills,
        );

        let inv_x_card = settings_row_card(
            "Invert Lateral Axis (Left / Right)",
            "Reverse horizontal dot drift during left and right turns",
            toggler(self.invert_x)
                .on_toggle(Message::InvertXChanged)
                .size(20.0)
                .style(fluent_toggler_style)
                .into(),
        );

        let inv_y_card = settings_row_card(
            "Invert Forward Axis (Acceleration / Braking)",
            "Reverse vertical dot drift during acceleration and braking",
            toggler(self.invert_y)
                .on_toggle(Message::InvertYChanged)
                .size(20.0)
                .style(fluent_toggler_style)
                .into(),
        );

        column![orient_card, inv_x_card, inv_y_card]
            .spacing(10)
            .into()
    }

    fn view_guide_page(&self) -> Element<'_, Message> {
        let method_tabs = segmented_container(
            row![
                self.guide_method_pill(GuideMethod::Hotspot, "1. Mobile Hotspot (Recommended)"),
                self.guide_method_pill(GuideMethod::Wifi, "2. Same Wi-Fi Router"),
                self.guide_method_pill(GuideMethod::Usb, "3. USB Cable (ADB)"),
            ]
            .spacing(2)
            .into(),
        );

        let method_card: Element<'_, Message> = match self.guide_method {
            GuideMethod::Hotspot => settings_card(
                column![
                    guide_step_row(
                        "1",
                        "Install & Open Mewtion-Android on Your Phone",
                        "Install the Android companion app and open it so 'Mewtion Bridge Active' appears on screen.",
                    ),
                    guide_step_row(
                        "2",
                        "Connect Your Windows Laptop to Your Phone's Hotspot",
                        "Turn on Wi-Fi Hotspot on your phone and connect Windows to it. Set the Windows network profile to 'Private' if prompted.",
                    ),
                    guide_step_row(
                        "3",
                        "Click 'Auto-Find Phone' Below",
                        "Note: On Pixel / Android 14+ phones, the app screen may show your mobile data IP (10.x.x.x) instead of the hotspot IP. 'Auto-Find Phone' scans the hotspot subnet and finds your phone automatically!",
                    ),
                    row![
                        button(
                            text("Download Android App ↗")
                                .size(12)
                                .color(TEXT_PRIMARY)
                        )
                        .on_press(Message::OpenUrl(
                            "https://github.com/aayuxh-vim/Mewtion-Android/releases"
                        ))
                        .padding([7, 14])
                        .style(secondary_button_style),
                        button(
                            text("Auto-Find Phone on Hotspot")
                                .font(FONT_SEGOE_SEMIBOLD)
                                .size(12)
                                .color(ACCENT_DARK_TEXT)
                        )
                        .on_press(Message::AutoDetectPhone)
                        .padding([7, 16])
                        .style(accent_button_style),
                    ]
                    .spacing(10),
                ]
                .spacing(14)
                .into(),
            ),
            GuideMethod::Wifi => settings_card(
                column![
                    guide_step_row(
                        "1",
                        "Connect Phone & PC to the Same Wi-Fi Router",
                        "Ensure your Wi-Fi network in Windows Settings → Network & Internet is set to 'Private network' so Windows Firewall allows local connections.",
                    ),
                    guide_step_row(
                        "2",
                        "Open Mewtion-Android on Your Phone",
                        "Turn off Mobile Data briefly before opening the app (so Android displays your 192.168.x.x Wi-Fi IP instead of your cellular IP), or simply use 'Auto-Find Phone'.",
                    ),
                    guide_step_row(
                        "3",
                        "Click 'Auto-Find Phone' or Enter the IP in Connection",
                        "Mewtion scans your local Wi-Fi subnet for port 8765 and connects automatically.",
                    ),
                    row![
                        button(
                            text("Auto-Find Phone on Wi-Fi")
                                .font(FONT_SEGOE_SEMIBOLD)
                                .size(12)
                                .color(ACCENT_DARK_TEXT)
                        )
                        .on_press(Message::AutoDetectPhone)
                        .padding([7, 16])
                        .style(accent_button_style),
                        button(
                            text("Go to Connection Tab →")
                                .size(12)
                                .color(TEXT_PRIMARY)
                        )
                        .on_press(Message::TabSelected(NavTab::Connection))
                        .padding([7, 14])
                        .style(secondary_button_style),
                    ]
                    .spacing(10),
                ]
                .spacing(14)
                .into(),
            ),
            GuideMethod::Usb => settings_card(
                column![
                    guide_step_row(
                        "1",
                        "Enable USB Debugging on Your Android Phone",
                        "Open Settings → About Phone and tap 'Build Number' 7 times. Then open Settings → System → Developer Options and turn on 'USB Debugging'.",
                    ),
                    guide_step_row(
                        "2",
                        "Install Google SDK Platform-Tools (ADB) on Windows",
                        "Download and extract platform-tools to your Downloads folder (or run 'winget install Google.PlatformTools' in Terminal).",
                    ),
                    guide_step_row(
                        "3",
                        "Plug in USB Cable & Click 'Activate USB Mode'",
                        "Tap 'Allow USB Debugging' on your phone prompt, open Mewtion-Android, and click 'Activate USB Mode' below.",
                    ),
                    row![
                        button(
                            text("USB Debugging Guide ↗")
                                .size(12)
                                .color(TEXT_PRIMARY)
                        )
                        .on_press(Message::OpenUrl(
                            "https://developer.android.com/studio/debug/dev-options"
                        ))
                        .padding([7, 12])
                        .style(secondary_button_style),
                        button(
                            text("Download ADB Tools ↗")
                                .size(12)
                                .color(TEXT_PRIMARY)
                        )
                        .on_press(Message::OpenUrl(
                            "https://developer.android.com/tools/releases/platform-tools"
                        ))
                        .padding([7, 12])
                        .style(secondary_button_style),
                        button(
                            text("Activate USB Mode (127.0.0.1)")
                                .font(FONT_SEGOE_SEMIBOLD)
                                .size(12)
                                .color(ACCENT_DARK_TEXT)
                        )
                        .on_press(Message::SetUsbMode)
                        .padding([7, 14])
                        .style(accent_button_style),
                    ]
                    .spacing(8),
                ]
                .spacing(14)
                .into(),
            ),
        };

        column![method_tabs, method_card].spacing(12).into()
    }

    fn sidebar_item(&self, tab: NavTab, label: &'static str) -> Element<'_, Message> {
        let is_active = self.active_tab == tab;
        let indicator = container(text(""))
            .width(Length::Fixed(3.0))
            .height(Length::Fixed(16.0))
            .style(move |_| container::Style {
                background: Some(
                    if is_active {
                        ACCENT_BLUE
                    } else {
                        Color::TRANSPARENT
                    }
                    .into(),
                ),
                border: Border {
                    radius: 1.5.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        button(
            row![
                indicator,
                text(label)
                    .font(if is_active {
                        FONT_SEGOE_SEMIBOLD
                    } else {
                        FONT_SEGOE
                    })
                    .size(13)
                    .color(if is_active {
                        TEXT_PRIMARY
                    } else {
                        TEXT_SECONDARY
                    }),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .on_press(Message::TabSelected(tab))
        .width(Length::Fill)
        .padding([9, 10])
        .style(move |_theme, status| button::Style {
            background: Some(
                if is_active {
                    BG_NAV_ACTIVE.into()
                } else if status == button::Status::Hovered {
                    BG_CARD.into()
                } else {
                    Color::TRANSPARENT.into()
                },
            ),
            border: Border {
                radius: 6.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            ..Default::default()
        })
        .into()
    }

    fn source_pill(&self, val: SensorSourcePref, label: &'static str) -> Element<'_, Message> {
        let active = self.sensor_source == val;
        pill_button(label, active, Message::SensorSourceChanged(val))
    }

    fn size_pill(&self, val: DotSize, label: &'static str) -> Element<'_, Message> {
        let active = self.selected_size == val;
        pill_button(label, active, Message::SizeSelected(val))
    }

    fn anim_pill(&self, val: AnimMode, label: &'static str) -> Element<'_, Message> {
        let active = self.anim_mode == val;
        pill_button(label, active, Message::AnimModeChanged(val))
    }

    fn monitor_pill(&self, val: MonitorTarget, label: &'static str) -> Element<'_, Message> {
        let active = self.monitor_target == val;
        pill_button(label, active, Message::MonitorTargetChanged(val))
    }

    fn orient_pill(&self, val: Orientation, label: &'static str) -> Element<'_, Message> {
        let active = self.orientation == val;
        pill_button(label, active, Message::OrientationChanged(val))
    }

    fn guide_method_pill(&self, val: GuideMethod, label: &'static str) -> Element<'_, Message> {
        let active = self.guide_method == val;
        pill_button(label, active, Message::GuideMethodSelected(val))
    }

    fn color_swatch(&self, color: DotColor) -> Element<'_, Message> {
        let is_selected = self.selected_color == color;
        button(text(""))
            .on_press(Message::ColorSelected(color))
            .width(Length::Fixed(24.0))
            .height(Length::Fixed(24.0))
            .style(move |_theme, status| button::Style {
                background: Some(color.to_color().into()),
                border: Border {
                    radius: 12.0.into(),
                    width: if is_selected {
                        2.5
                    } else if status == button::Status::Hovered {
                        1.5
                    } else {
                        1.0
                    },
                    color: if is_selected {
                        ACCENT_BLUE
                    } else {
                        BORDER_CONTROL
                    },
                },
                ..Default::default()
            })
            .into()
    }
}

// === WinUI 3 Component Helpers ===

fn settings_card(content: Element<'_, Message>) -> Element<'_, Message> {
    container(content)
        .width(Length::Fill)
        .padding([14, 18])
        .style(|_| container::Style {
            background: Some(BG_CARD.into()),
            border: Border {
                radius: 8.0.into(),
                width: 1.0,
                color: BORDER_CARD,
            },
            ..Default::default()
        })
        .into()
}

fn settings_row_card<'a>(
    title: &'static str,
    subtitle: &'a str,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    settings_card(
        row![
            column![
                text(title)
                    .font(FONT_SEGOE_SEMIBOLD)
                    .size(13)
                    .color(TEXT_PRIMARY),
                text(subtitle).size(11).color(TEXT_SECONDARY),
            ]
            .spacing(2)
            .width(Length::Fill),
            control,
        ]
        .spacing(16)
        .align_y(Alignment::Center)
        .into(),
    )
}

fn slider_card<'a>(
    title: &'static str,
    subtitle: &'static str,
    badge: String,
    slider_el: Element<'a, Message>,
) -> Element<'a, Message> {
    settings_card(
        row![
            column![
                text(title)
                    .font(FONT_SEGOE_SEMIBOLD)
                    .size(13)
                    .color(TEXT_PRIMARY),
                text(subtitle).size(11).color(TEXT_SECONDARY),
            ]
            .spacing(2)
            .width(Length::Fill),
            row![
                container(
                    text(badge)
                        .font(FONT_SEGOE_SEMIBOLD)
                        .size(12)
                        .color(ACCENT_BLUE)
                )
                .padding([3, 8])
                .style(|_| container::Style {
                    background: Some(BG_CONTENT.into()),
                    border: Border {
                        radius: 4.0.into(),
                        width: 1.0,
                        color: BORDER_CARD,
                    },
                    ..Default::default()
                }),
                slider_el,
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(16)
        .align_y(Alignment::Center)
        .into(),
    )
}

fn guide_step_row<'a>(
    num: &'static str,
    title: &'static str,
    desc: &'static str,
) -> Element<'a, Message> {
    row![
        container(
            text(num)
                .font(FONT_SEGOE_SEMIBOLD)
                .size(12)
                .color(ACCENT_BLUE)
        )
        .width(Length::Fixed(24.0))
        .height(Length::Fixed(24.0))
        .center_x(Length::Fixed(24.0))
        .center_y(Length::Fixed(24.0))
        .style(|_| container::Style {
            background: Some(BG_CONTENT.into()),
            border: Border {
                radius: 12.0.into(),
                width: 1.0,
                color: BORDER_CONTROL,
            },
            ..Default::default()
        }),
        column![
            text(title)
                .font(FONT_SEGOE_SEMIBOLD)
                .size(13)
                .color(TEXT_PRIMARY),
            text(desc).size(11).color(TEXT_SECONDARY),
        ]
        .spacing(2)
        .width(Length::Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Start)
    .into()
}

fn segmented_container(content: Element<'_, Message>) -> Element<'_, Message> {
    container(content)
        .padding(3)
        .style(|_| container::Style {
            background: Some(BG_CONTENT.into()),
            border: Border {
                radius: 7.0.into(),
                width: 1.0,
                color: BORDER_CARD,
            },
            ..Default::default()
        })
        .into()
}

fn pill_button<'a>(label: &'static str, active: bool, msg: Message) -> Element<'a, Message> {
    button(
        text(label)
            .font(if active {
                FONT_SEGOE_SEMIBOLD
            } else {
                FONT_SEGOE
            })
            .size(12)
            .color(if active {
                ACCENT_DARK_TEXT
            } else {
                TEXT_SECONDARY
            }),
    )
    .on_press(msg)
    .padding([5, 11])
    .style(move |_theme, status| button::Style {
        background: Some(
            if active {
                ACCENT_BLUE.into()
            } else if status == button::Status::Hovered {
                BG_CONTROL_HOVER.into()
            } else {
                Color::TRANSPARENT.into()
            },
        ),
        border: Border {
            radius: 5.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        ..Default::default()
    })
    .into()
}

fn secondary_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(
            match status {
                button::Status::Hovered => BG_CONTROL_HOVER,
                button::Status::Pressed => BG_CARD,
                _ => BG_CONTROL,
            }
            .into(),
        ),
        text_color: TEXT_PRIMARY,
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: BORDER_CONTROL,
        },
        ..Default::default()
    }
}

fn ghost_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(
            match status {
                button::Status::Hovered => BG_CONTROL,
                _ => Color::TRANSPARENT,
            }
            .into(),
        ),
        text_color: TEXT_SECONDARY,
        border: Border {
            radius: 5.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        ..Default::default()
    }
}

fn accent_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(
            match status {
                button::Status::Hovered => ACCENT_BLUE_HOVER,
                _ => ACCENT_BLUE,
            }
            .into(),
        ),
        text_color: ACCENT_DARK_TEXT,
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: ACCENT_BLUE,
        },
        ..Default::default()
    }
}

fn fluent_toggler_style(_theme: &Theme, status: toggler::Status) -> toggler::Style {
    let is_toggled = match status {
        toggler::Status::Active { is_toggled }
        | toggler::Status::Hovered { is_toggled }
        | toggler::Status::Disabled { is_toggled } => is_toggled,
    };
    let is_hovered = matches!(status, toggler::Status::Hovered { .. });

    toggler::Style {
        background: Background::Color(if is_toggled {
            if is_hovered {
                ACCENT_BLUE_HOVER
            } else {
                ACCENT_BLUE
            }
        } else if is_hovered {
            BG_CONTROL_HOVER
        } else {
            BG_CONTENT
        }),
        background_border_width: 1.0,
        background_border_color: if is_toggled {
            ACCENT_BLUE
        } else {
            BORDER_CONTROL
        },
        foreground: Background::Color(if is_toggled {
            ACCENT_DARK_TEXT
        } else {
            TEXT_PRIMARY
        }),
        foreground_border_width: 0.0,
        foreground_border_color: Color::TRANSPARENT,
        text_color: Some(TEXT_PRIMARY),
        border_radius: None,
        padding_ratio: 0.22,
    }
}

fn fluent_slider_style(_theme: &Theme, status: slider::Status) -> slider::Style {
    let handle_color = match status {
        slider::Status::Hovered | slider::Status::Dragged => ACCENT_BLUE_HOVER,
        slider::Status::Active => ACCENT_BLUE,
    };
    slider::Style {
        rail: slider::Rail {
            backgrounds: (
                Background::Color(ACCENT_BLUE),
                Background::Color(BORDER_CONTROL),
            ),
            width: 4.0,
            border: Border {
                radius: 2.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 8.0 },
            background: Background::Color(handle_color),
            border_width: 2.5,
            border_color: BG_CARD,
        },
    }
}

fn fluent_text_input_style(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let is_focused = matches!(status, text_input::Status::Focused { .. });
    let is_hovered = matches!(status, text_input::Status::Hovered);
    text_input::Style {
        background: Background::Color(if is_hovered || is_focused {
            BG_CONTROL_HOVER
        } else {
            BG_CONTENT
        }),
        border: Border {
            radius: 6.0.into(),
            width: if is_focused { 1.5 } else { 1.0 },
            color: if is_focused {
                ACCENT_BLUE
            } else {
                BORDER_CONTROL
            },
        },
        icon: TEXT_SECONDARY,
        placeholder: TEXT_MUTED,
        value: TEXT_PRIMARY,
        selection: ACCENT_BLUE,
    }
}
