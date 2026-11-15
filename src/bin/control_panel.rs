#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use iced::font::{Family, Weight};
use iced::widget::{
    button, column, container, pick_list, row, scrollable, slider, text, text_input, toggler,
};
use iced::window;
use iced::{Alignment, Background, Border, Color, Element, Font, Length, Size, Task, Theme};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

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

// Windows 11 Fluent / WinUI 3 Dark Palette
const WIN_BG_MICA: Color = Color::from_rgb(0.125, 0.125, 0.137); // #202023 Mica base
const WIN_CARD_BG: Color = Color::from_rgb(0.169, 0.169, 0.184); // #2B2B2F WinUI 3 LayerFillColorDefault
const WIN_CARD_HEADER: Color = Color::from_rgb(0.196, 0.196, 0.215); // #323237 Subtle elevated surface
const WIN_CARD_BORDER: Color = Color::from_rgb(0.235, 0.235, 0.255); // #3C3C41 CardStrokeColorDefault
const WIN_DIVIDER: Color = Color::from_rgb(0.220, 0.220, 0.239); // #38383D DividerStrokeColorDefault
const WIN_INPUT_BG: Color = Color::from_rgb(0.141, 0.141, 0.157); // #242428 ControlFillColorDefault
const WIN_INPUT_HOVER: Color = Color::from_rgb(0.185, 0.185, 0.204);

const WIN_TEXT_PRIMARY: Color = Color::from_rgb(0.96, 0.96, 0.97); // #F5F5F7
const WIN_TEXT_SECONDARY: Color = Color::from_rgb(0.68, 0.68, 0.72); // #ADADB8
const WIN_ACCENT: Color = Color::from_rgb(0.298, 0.761, 1.000); // #4CC2FF Windows 11 Dark Mode Accent
const WIN_ACCENT_HOVER: Color = Color::from_rgb(0.380, 0.805, 1.000);
const WIN_ACCENT_DARK_TEXT: Color = Color::from_rgb(0.05, 0.08, 0.12);

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
        size: Size::new(500.0, 860.0),
        min_size: Some(Size::new(440.0, 620.0)),
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
        .window(win_settings)
        .run()
}

/// Generates a crisp 32x32 RGBA window icon matching the Mewtion brand so the titlebar
/// and taskbar always show the custom icon.
fn build_window_icon() -> Option<window::Icon> {
    let size = 32u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let dx = (fx - 16.0).abs() - 9.0;
            let dy = (fy - 16.0).abs() - 9.0;
            let dist = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt() + dx.max(dy).min(0.0) - 5.0;
            let idx = ((y * size + x) * 4) as usize;
            if dist <= 0.5 {
                let alpha = (0.5 - dist).clamp(0.0, 1.0);
                rgba[idx] = 0x1E;
                rgba[idx + 1] = 0x20;
                rgba[idx + 2] = 0x28;
                rgba[idx + 3] = (alpha * 255.0) as u8;

                // Bright cyan motion dots around the perimeter
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
                        rgba[idx] = 0x4C;
                        rgba[idx + 1] = 0xC2;
                        rgba[idx + 2] = 0xFF;
                        rgba[idx + 3] = 255;
                    }
                }
            }
        }
    }
    window::icon::from_rgba(rgba, size, size).ok()
}

/// Applies native Windows 11 DWM Dark Mode titlebar (`DWMWA_USE_IMMERSIVE_DARK_MODE`),
/// dark caption color (`#202023`), and Mica backdrop (`DWMWA_SYSTEMBACKDROP_TYPE`).
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
            std::thread::sleep(std::time::Duration::from_millis(60));
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
                        // COLORREF is 0x00BBGGRR -> #202023 is 0x00232020
                        let caption_color: u32 = 0x0023_2020;
                        let _ = DwmSetWindowAttribute(
                            hwnd,
                            DWMWA_CAPTION_COLOR,
                            &caption_color as *const _ as *const _,
                            std::mem::size_of::<u32>() as u32,
                        );
                        // DWMSBT_MAINWINDOW (Mica) = 2
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

fn resolve_config_path() -> PathBuf {
    let cwd_path = PathBuf::from("mewtion_config.txt");
    if cwd_path.exists() {
        return cwd_path;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let exe_cfg = dir.join("mewtion_config.txt");
            if exe_cfg.exists() {
                return exe_cfg;
            }
            if let Some(parent) = dir.parent() {
                let parent_cfg = parent.join("mewtion_config.txt");
                if parent_cfg.exists() {
                    return parent_cfg;
                }
            }
            return exe_cfg;
        }
    }
    cwd_path
}

fn resolve_status_path() -> PathBuf {
    let cfg = resolve_config_path();
    if let Some(parent) = cfg.parent() {
        if !parent.as_os_str().is_empty() {
            return parent.join("mewtion_status.txt");
        }
    }
    PathBuf::from("mewtion_status.txt")
}

fn read_sensor_status() -> String {
    fs::read_to_string(resolve_status_path())
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Overlay ready — waiting for sensor stream".to_string())
}

fn new_silent_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

fn is_overlay_running() -> bool {
    if let Ok(output) = new_silent_command("tasklist")
        .args(["/FI", "IMAGENAME eq Mewtion.exe", "/NH"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
        return stdout.contains("mewtion.exe");
    }
    false
}

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

fn ensure_adb_and_overlay_started() {
    apply_win11_dwm_chrome();

    let _ = new_silent_command("adb")
        .args(["forward", "tcp:8765", "tcp:8765"])
        .output();

    if !is_overlay_running() {
        if let Some(overlay_path) = find_overlay_exe() {
            let _ = new_silent_command(overlay_path.to_str().unwrap_or("Mewtion.exe")).spawn();
        }
    }
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

fn detect_windows_gateway_ip() -> Option<String> {
    if let Ok(output) = new_silent_command("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "(Get-NetRoute -DestinationPrefix '0.0.0.0/0' | Sort-Object RouteMetric | Select-Object -First 1).NextHop",
        ])
        .output()
    {
        let ip = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !ip.is_empty() && ip != "0.0.0.0" && ip.contains('.') {
            return Some(ip);
        }
    }

    if let Ok(output) = new_silent_command("ipconfig").output() {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if line.contains("Default Gateway") {
                if let Some((_left, right)) = line.split_once(':') {
                    let candidate = right.trim();
                    if !candidate.is_empty() && candidate.contains('.') && candidate != "0.0.0.0" {
                        return Some(candidate.to_string());
                    }
                }
            }
        }
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DotSize {
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
enum DotColor {
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
enum AnimMode {
    Fluid,
    Rigid,
}
impl std::fmt::Display for AnimMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AnimMode::Fluid => "Fluid (Organic)",
                AnimMode::Rigid => "Rigid (Mechanical)",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MonitorTarget {
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
impl std::fmt::Display for MonitorTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MonitorTarget::Primary => "Primary Display",
                MonitorTarget::All => "All Displays (Span)",
            }
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Orientation {
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
impl std::fmt::Display for Orientation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Orientation::Deg0 => "0° (Standard)",
                Orientation::Deg90 => "90° (Landscape Right)",
                Orientation::Deg180 => "180° (Inverted)",
                Orientation::Deg270 => "270° (Landscape Left)",
            }
        )
    }
}

struct SettingsApp {
    selected_size: DotSize,
    selected_color: DotColor,
    opacity: f32,
    margin_pct: f32,
    sensitivity: f32,
    anim_mode: AnimMode,
    monitor_target: MonitorTarget,
    orientation: Orientation,
    invert_x: bool,
    invert_y: bool,
    simulate_motion: bool,
    autostart_windows: bool,
    phone_ip: String,
    overlay_running: bool,
    sensor_status: String,
    status_message: String,
}

impl Default for SettingsApp {
    fn default() -> Self {
        let path = resolve_config_path();
        let mut app = Self {
            selected_size: DotSize::Medium,
            selected_color: DotColor::White,
            opacity: 0.85,
            margin_pct: 0.03,
            sensitivity: 3.5,
            anim_mode: AnimMode::Fluid,
            monitor_target: MonitorTarget::Primary,
            orientation: Orientation::Deg0,
            invert_x: false,
            invert_y: false,
            simulate_motion: false,
            autostart_windows: is_autostart_enabled(),
            phone_ip: "127.0.0.1".to_string(),
            overlay_running: is_overlay_running(),
            sensor_status: read_sensor_status(),
            status_message: "All changes apply live automatically".to_string(),
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
                    "orientation" => {
                        if let Ok(val) = v.parse::<i32>() {
                            app.orientation = Orientation::from_degrees(val);
                        }
                    }
                    "invert_x" => app.invert_x = matches!(v, "true" | "1" | "yes"),
                    "invert_y" => app.invert_y = matches!(v, "true" | "1" | "yes"),
                    "simulate" => app.simulate_motion = matches!(v, "true" | "1" | "yes"),
                    "ip" => app.phone_ip = v.to_string(),
                    _ => {}
                }
            }
        }
        app
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    SizeSelected(DotSize),
    ColorSelected(DotColor),
    OpacityChanged(f32),
    MarginChanged(f32),
    SensitivityChanged(f32),
    AnimModeChanged(AnimMode),
    MonitorTargetChanged(MonitorTarget),
    OrientationChanged(Orientation),
    InvertXChanged(bool),
    InvertYChanged(bool),
    SimulateMotionChanged(bool),
    AutostartChanged(bool),
    IpChanged(String),
    SetUsbMode,
    AutoDetectHotspot,
    RefreshStatus,
    ToggleOverlay(bool),
    ResetDefaults,
    SaveClicked,
}

impl SettingsApp {
    fn write_config(&self) -> Result<(), std::io::Error> {
        let config_data = format!(
            "size={}\ncolor={}\nopacity={}\nmargin={}\nsensitivity={}\nanimation={}\nip={}\nmonitor={}\norientation={}\ninvert_x={}\ninvert_y={}\nsimulate={}\n",
            self.selected_size.pixels(),
            self.selected_color.hex_string(),
            self.opacity,
            self.margin_pct,
            self.sensitivity,
            match self.anim_mode {
                AnimMode::Fluid => "Fluid",
                AnimMode::Rigid => "Rigid",
            },
            self.phone_ip,
            self.monitor_target.config_str(),
            self.orientation.degrees(),
            self.invert_x,
            self.invert_y,
            self.simulate_motion,
        );
        fs::write(resolve_config_path(), config_data)
    }

    fn apply_live(&mut self, msg: &str) {
        if self.write_config().is_ok() {
            self.status_message = msg.to_string();
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        self.overlay_running = is_overlay_running();
        self.sensor_status = read_sensor_status();

        match message {
            Message::SizeSelected(size) => {
                self.selected_size = size;
                self.apply_live("Dot size updated live");
            }
            Message::ColorSelected(color) => {
                self.selected_color = color;
                self.apply_live("Dot color updated live");
            }
            Message::OpacityChanged(val) => {
                self.opacity = val;
                let _ = self.write_config();
            }
            Message::MarginChanged(val) => {
                self.margin_pct = val;
                let _ = self.write_config();
            }
            Message::SensitivityChanged(val) => {
                self.sensitivity = val;
                let _ = self.write_config();
            }
            Message::AnimModeChanged(mode) => {
                self.anim_mode = mode;
                self.apply_live("Animation mode updated live");
            }
            Message::MonitorTargetChanged(target) => {
                self.monitor_target = target;
                self.apply_live("Display target updated live");
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
                self.simulate_motion = val;
                self.apply_live(if val {
                    "Desk Test Mode active — simulating vehicle motion"
                } else {
                    "Desk Test Mode off — using live sensor input"
                });
            }
            Message::AutostartChanged(val) => {
                if set_autostart_enabled(val) {
                    self.autostart_windows = val;
                    self.status_message = if val {
                        "Registered Mewtion in Windows Startup".to_string()
                    } else {
                        "Removed Mewtion from Windows Startup".to_string()
                    };
                } else {
                    self.status_message = "Could not update Windows Startup registry".to_string();
                }
            }
            Message::IpChanged(ip) => self.phone_ip = ip,
            Message::SetUsbMode => {
                self.phone_ip = "127.0.0.1".to_string();
                let _ = new_silent_command("adb")
                    .args(["forward", "tcp:8765", "tcp:8765"])
                    .output();
                self.apply_live("USB Mode enabled (127.0.0.1:8765)");
            }
            Message::AutoDetectHotspot => match detect_windows_gateway_ip() {
                Some(ip) => {
                    self.phone_ip = ip;
                    self.apply_live("Hotspot gateway IP detected & applied");
                }
                None => {
                    self.status_message = "Could not detect default gateway IP".to_string();
                }
            },
            Message::RefreshStatus => {
                self.overlay_running = is_overlay_running();
                self.sensor_status = read_sensor_status();
                self.status_message = "Sensor status refreshed".to_string();
            }
            Message::ToggleOverlay(should_run) => {
                if !should_run && is_overlay_running() {
                    let _ = new_silent_command("taskkill")
                        .args(["/F", "/IM", "Mewtion.exe"])
                        .output();
                    self.overlay_running = false;
                    self.status_message = "Overlay paused".to_string();
                } else if should_run && !is_overlay_running() {
                    if let Some(overlay_path) = find_overlay_exe() {
                        match new_silent_command(overlay_path.to_str().unwrap_or("Mewtion.exe"))
                            .spawn()
                        {
                            Ok(_) => {
                                self.overlay_running = true;
                                self.status_message = "Overlay active".to_string();
                            }
                            Err(_) => {
                                self.status_message = "Failed to launch Mewtion.exe".to_string();
                            }
                        }
                    } else {
                        self.status_message = "Mewtion.exe not found in application directory".to_string();
                    }
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
                self.orientation = Orientation::Deg0;
                self.invert_x = false;
                self.invert_y = false;
                self.simulate_motion = false;
                self.apply_live("Restored default settings");
            }
            Message::SaveClicked => match self.write_config() {
                Ok(_) => self.status_message = "All settings saved & applied live".to_string(),
                Err(_) => self.status_message = "Error writing mewtion_config.txt".to_string(),
            },
        }
        Task::none()
    }

    fn view(&self) -> Element<Message> {
        // === 1. HERO HEADER CARD (Windows 11 Settings Top Banner) ===
        let status_dot_color = if !self.overlay_running {
            COLOR_RED
        } else if self.sensor_status.starts_with("Active:")
            || self.sensor_status.starts_with("Connected")
        {
            COLOR_GREEN
        } else {
            COLOR_YELLOW
        };

        let status_dot = container(text(""))
            .width(Length::Fixed(10.0))
            .height(Length::Fixed(10.0))
            .style(move |_| container::Style {
                background: Some(status_dot_color.into()),
                border: Border {
                    radius: 5.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        let status_str = if self.overlay_running {
            self.sensor_status.as_str()
        } else {
            "Overlay stopped"
        };

        let hero_left = column![
            text("Mewtion")
                .font(FONT_SEGOE_SEMIBOLD)
                .size(24)
                .color(WIN_TEXT_PRIMARY),
            text("Vehicle Motion Cues for Windows")
                .size(12)
                .color(WIN_TEXT_SECONDARY),
            row![
                status_dot,
                text(status_str).size(12).color(WIN_TEXT_SECONDARY),
                button(text("Refresh").size(11).color(WIN_TEXT_PRIMARY))
                    .on_press(Message::RefreshStatus)
                    .padding([3, 8])
                    .style(subtle_button_style),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(4)
        .width(Length::Fill);

        let overlay_switch = column![
            text(if self.overlay_running {
                "Overlay On"
            } else {
                "Overlay Off"
            })
            .size(12)
            .color(WIN_TEXT_PRIMARY),
            toggler(self.overlay_running)
                .on_toggle(Message::ToggleOverlay)
                .size(22.0)
                .style(fluent_toggler_style),
        ]
        .spacing(6)
        .align_x(Alignment::End);

        // Mini screen simulation preview inside the hero card
        let dot_px = self.selected_size.pixels();
        let dot_color = Color {
            a: self.opacity,
            ..self.selected_color.to_color()
        };
        let make_edge_dot = |sz: f32| {
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

        let mini_screen = container(
            row![
                column![
                    make_edge_dot(dot_px * 0.65),
                    make_edge_dot(dot_px),
                    make_edge_dot(dot_px * 0.65)
                ]
                .spacing(8)
                .align_x(Alignment::Center),
                container(
                    column![
                        text("LIVE PREVIEW")
                            .font(FONT_SEGOE_SEMIBOLD)
                            .size(11)
                            .color(WIN_TEXT_SECONDARY),
                        text(format!(
                            "{} • {:.0}% Opacity • {}% Margin",
                            self.selected_size.label(),
                            self.opacity * 100.0,
                            (self.margin_pct * 100.0).round() as i32
                        ))
                        .size(12)
                        .color(WIN_TEXT_PRIMARY),
                    ]
                    .spacing(2)
                    .align_x(Alignment::Center),
                )
                .width(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
                column![
                    make_edge_dot(dot_px * 0.65),
                    make_edge_dot(dot_px),
                    make_edge_dot(dot_px * 0.65)
                ]
                .spacing(8)
                .align_x(Alignment::Center),
            ]
            .align_y(Alignment::Center)
            .padding([10, 18]),
        )
        .width(Length::Fill)
        .height(Length::Fixed(76.0))
        .style(|_| container::Style {
            background: Some(WIN_INPUT_BG.into()),
            border: Border {
                radius: 6.0.into(),
                width: 1.0,
                color: WIN_CARD_BORDER,
            },
            ..Default::default()
        });

        let hero_card = fluent_card(
            column![
                row![hero_left, overlay_switch].align_y(Alignment::Center),
                mini_screen,
            ]
            .spacing(14)
            .into(),
        );

        // === 2. APPEARANCE SECTION ===
        let sizes_row = row![
            self.segmented_size_button(DotSize::Small),
            self.segmented_size_button(DotSize::Medium),
            self.segmented_size_button(DotSize::Large),
        ]
        .spacing(6);

        let colors_row = row![
            self.color_swatch(DotColor::White),
            self.color_swatch(DotColor::Blue),
            self.color_swatch(DotColor::Cyan),
            self.color_swatch(DotColor::Green),
            self.color_swatch(DotColor::Yellow),
            self.color_swatch(DotColor::Purple),
            self.color_swatch(DotColor::Red),
        ]
        .spacing(10);

        let appearance_card = fluent_card(
            column![
                setting_row(
                    "Dot Size",
                    "Diameter of edge motion cues on screen",
                    sizes_row.into()
                ),
                divider(),
                setting_row(
                    "Dot Color",
                    "Choose a high-contrast tint that suits your wallpaper",
                    colors_row.into()
                ),
                divider(),
                slider_setting_row(
                    "Opacity",
                    "Transparency of motion dots",
                    format!("{:.0}%", self.opacity * 100.0),
                    slider(0.1..=1.0, self.opacity, Message::OpacityChanged)
                        .step(0.05)
                        .style(fluent_slider_style)
                        .into(),
                ),
                divider(),
                slider_setting_row(
                    "Screen Edge Margin",
                    "Distance from the left and right edges of your display",
                    format!("{:.0}%", self.margin_pct * 100.0),
                    slider(0.0..=0.15, self.margin_pct, Message::MarginChanged)
                        .step(0.01)
                        .style(fluent_slider_style)
                        .into(),
                ),
            ]
            .spacing(12)
            .into(),
        );

        // === 3. MOTION & DISPLAY SECTION ===
        let motion_card = fluent_card(
            column![
                slider_setting_row(
                    "Motion Sensitivity",
                    "Lower values react more strongly to gentle acceleration",
                    format!("{:.1}", self.sensitivity),
                    slider(1.0..=10.0, self.sensitivity, Message::SensitivityChanged)
                        .step(0.5)
                        .style(fluent_slider_style)
                        .into(),
                ),
                divider(),
                setting_row(
                    "Animation Style",
                    "Fluid varies dot sizes organically; Rigid keeps uniform dots",
                    pick_list(
                        &[AnimMode::Fluid, AnimMode::Rigid][..],
                        Some(self.anim_mode),
                        Message::AnimModeChanged,
                    )
                    .width(Length::Fixed(185.0))
                    .padding([7, 12])
                    .style(fluent_pick_list_style)
                    .into(),
                ),
                divider(),
                setting_row(
                    "Display Target",
                    "Render cues on your primary monitor or across all displays",
                    pick_list(
                        &[MonitorTarget::Primary, MonitorTarget::All][..],
                        Some(self.monitor_target),
                        Message::MonitorTargetChanged,
                    )
                    .width(Length::Fixed(185.0))
                    .padding([7, 12])
                    .style(fluent_pick_list_style)
                    .into(),
                ),
            ]
            .spacing(12)
            .into(),
        );

        // === 4. SENSOR SOURCE & ORIENTATION SECTION ===
        let ip_input = text_input("e.g. 192.168.43.1", &self.phone_ip)
            .on_input(Message::IpChanged)
            .padding([8, 12])
            .size(13)
            .style(fluent_text_input_style);

        let sensor_card = fluent_card(
            column![
                column![
                    row![
                        column![
                            text("Android Companion IP / Mode")
                                .font(FONT_SEGOE_SEMIBOLD)
                                .size(14)
                                .color(WIN_TEXT_PRIMARY),
                            text("Used automatically when no built-in Windows accelerometer is present")
                                .size(12)
                                .color(WIN_TEXT_SECONDARY),
                        ]
                        .spacing(2)
                        .width(Length::Fill),
                        button(text("Apply IP").size(12).color(WIN_ACCENT_DARK_TEXT))
                            .on_press(Message::SaveClicked)
                            .padding([6, 12])
                            .style(accent_button_style),
                    ]
                    .align_y(Alignment::Center),
                    row![
                        ip_input,
                        button(text("USB (ADB)").size(12).color(WIN_TEXT_PRIMARY))
                            .on_press(Message::SetUsbMode)
                            .padding([8, 12])
                            .style(subtle_button_style),
                        button(text("Detect Hotspot").size(12).color(WIN_TEXT_PRIMARY))
                            .on_press(Message::AutoDetectHotspot)
                            .padding([8, 12])
                            .style(subtle_button_style),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                ]
                .spacing(10),
                divider(),
                setting_row(
                    "Sensor Orientation",
                    "Rotate axes if your phone or tablet is mounted sideways",
                    pick_list(
                        &[
                            Orientation::Deg0,
                            Orientation::Deg90,
                            Orientation::Deg180,
                            Orientation::Deg270,
                        ][..],
                        Some(self.orientation),
                        Message::OrientationChanged,
                    )
                    .width(Length::Fixed(185.0))
                    .padding([7, 12])
                    .style(fluent_pick_list_style)
                    .into(),
                ),
                divider(),
                setting_row(
                    "Invert Lateral Axis (X)",
                    "Reverse left/right drift direction during turns",
                    toggler(self.invert_x)
                        .on_toggle(Message::InvertXChanged)
                        .size(20.0)
                        .style(fluent_toggler_style)
                        .into(),
                ),
                divider(),
                setting_row(
                    "Invert Forward Axis (Y)",
                    "Reverse forward/backward drift during braking & acceleration",
                    toggler(self.invert_y)
                        .on_toggle(Message::InvertYChanged)
                        .size(20.0)
                        .style(fluent_toggler_style)
                        .into(),
                ),
            ]
            .spacing(12)
            .into(),
        );

        // === 5. SYSTEM & DIAGNOSTICS SECTION ===
        let system_card = fluent_card(
            column![
                setting_row(
                    "Simulate Motion (Desk Test Mode)",
                    "Generate smooth figure-8 motion to preview dots while stationary",
                    toggler(self.simulate_motion)
                        .on_toggle(Message::SimulateMotionChanged)
                        .size(20.0)
                        .style(fluent_toggler_style)
                        .into(),
                ),
                divider(),
                setting_row(
                    "Start Mewtion with Windows",
                    "Automatically launch the motion cues overlay when you sign in",
                    toggler(self.autostart_windows)
                        .on_toggle(Message::AutostartChanged)
                        .size(20.0)
                        .style(fluent_toggler_style)
                        .into(),
                ),
            ]
            .spacing(12)
            .into(),
        );

        // === 6. FOOTER ACTION BAR ===
        let footer_row = row![
            text(&self.status_message)
                .size(12)
                .color(WIN_ACCENT)
                .width(Length::Fill),
            button(text("Reset Defaults").size(12).color(WIN_TEXT_PRIMARY))
                .on_press(Message::ResetDefaults)
                .padding([8, 14])
                .style(subtle_button_style),
            button(
                text("Save & Apply")
                    .font(FONT_SEGOE_SEMIBOLD)
                    .size(12)
                    .color(WIN_ACCENT_DARK_TEXT)
            )
            .on_press(Message::SaveClicked)
            .padding([8, 18])
            .style(accent_button_style),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let main_column = column![
            hero_card,
            section_title("Appearance"),
            appearance_card,
            section_title("Motion & Display"),
            motion_card,
            section_title("Sensor & Calibration"),
            sensor_card,
            section_title("System & Diagnostics"),
            system_card,
            footer_row,
        ]
        .spacing(12)
        .padding([20, 24])
        .max_width(540);

        container(scrollable(
            container(main_column)
                .width(Length::Fill)
                .center_x(Length::Fill),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(WIN_BG_MICA.into()),
            text_color: Some(WIN_TEXT_PRIMARY),
            ..Default::default()
        })
        .into()
    }

    fn segmented_size_button(&self, size: DotSize) -> Element<Message> {
        let is_selected = self.selected_size == size;
        let label = match size {
            DotSize::Small => "Small",
            DotSize::Medium => "Medium",
            DotSize::Large => "Large",
        };
        button(
            text(label)
                .size(12)
                .color(if is_selected {
                    WIN_ACCENT_DARK_TEXT
                } else {
                    WIN_TEXT_PRIMARY
                }),
        )
        .on_press(Message::SizeSelected(size))
        .padding([6, 12])
        .style(move |_theme, status| button::Style {
            background: Some(
                if is_selected {
                    WIN_ACCENT.into()
                } else if status == button::Status::Hovered {
                    WIN_INPUT_HOVER.into()
                } else {
                    WIN_INPUT_BG.into()
                },
            ),
            border: Border {
                radius: 6.0.into(),
                width: 1.0,
                color: if is_selected {
                    WIN_ACCENT
                } else {
                    WIN_CARD_BORDER
                },
            },
            ..Default::default()
        })
        .into()
    }

    fn color_swatch(&self, color: DotColor) -> Element<Message> {
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
                        WIN_ACCENT
                    } else {
                        WIN_CARD_BORDER
                    },
                },
                ..Default::default()
            })
            .into()
    }
}

// === Fluent / WinUI 3 Layout & Style Helpers ===

fn section_title(label: &'static str) -> Element<'static, Message> {
    text(label)
        .font(FONT_SEGOE_SEMIBOLD)
        .size(14)
        .color(WIN_TEXT_PRIMARY)
        .into()
}

fn fluent_card(content: Element<Message>) -> Element<Message> {
    container(content)
        .width(Length::Fill)
        .padding(16)
        .style(|_| container::Style {
            background: Some(WIN_CARD_BG.into()),
            border: Border {
                radius: 8.0.into(),
                width: 1.0,
                color: WIN_CARD_BORDER,
            },
            ..Default::default()
        })
        .into()
}

fn divider() -> Element<'static, Message> {
    container(text(""))
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .style(|_| container::Style {
            background: Some(WIN_DIVIDER.into()),
            ..Default::default()
        })
        .into()
}

fn setting_row<'a>(
    title: &'static str,
    subtitle: &'static str,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    row![
        column![
            text(title)
                .font(FONT_SEGOE_SEMIBOLD)
                .size(13)
                .color(WIN_TEXT_PRIMARY),
            text(subtitle).size(11).color(WIN_TEXT_SECONDARY),
        ]
        .spacing(2)
        .width(Length::Fill),
        control,
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

fn slider_setting_row<'a>(
    title: &'static str,
    subtitle: &'static str,
    badge: String,
    slider_el: Element<'a, Message>,
) -> Element<'a, Message> {
    column![
        row![
            column![
                text(title)
                    .font(FONT_SEGOE_SEMIBOLD)
                    .size(13)
                    .color(WIN_TEXT_PRIMARY),
                text(subtitle).size(11).color(WIN_TEXT_SECONDARY),
            ]
            .spacing(2)
            .width(Length::Fill),
            container(
                text(badge)
                    .font(FONT_SEGOE_SEMIBOLD)
                    .size(12)
                    .color(WIN_ACCENT)
            )
            .padding([3, 8])
            .style(|_| container::Style {
                background: Some(WIN_CARD_HEADER.into()),
                border: Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        ]
        .align_y(Alignment::Center),
        slider_el,
    ]
    .spacing(8)
    .into()
}

fn subtle_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(
            match status {
                button::Status::Hovered => WIN_INPUT_HOVER,
                button::Status::Pressed => WIN_CARD_HEADER,
                _ => WIN_INPUT_BG,
            }
            .into(),
        ),
        text_color: WIN_TEXT_PRIMARY,
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: WIN_CARD_BORDER,
        },
        ..Default::default()
    }
}

fn accent_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(
            match status {
                button::Status::Hovered => WIN_ACCENT_HOVER,
                _ => WIN_ACCENT,
            }
            .into(),
        ),
        text_color: WIN_ACCENT_DARK_TEXT,
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: WIN_ACCENT,
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
                WIN_ACCENT_HOVER
            } else {
                WIN_ACCENT
            }
        } else if is_hovered {
            WIN_INPUT_HOVER
        } else {
            WIN_INPUT_BG
        }),
        background_border_width: 1.0,
        background_border_color: if is_toggled {
            WIN_ACCENT
        } else {
            WIN_TEXT_SECONDARY
        },
        foreground: Background::Color(if is_toggled {
            WIN_ACCENT_DARK_TEXT
        } else {
            WIN_TEXT_PRIMARY
        }),
        foreground_border_width: 0.0,
        foreground_border_color: Color::TRANSPARENT,
        text_color: Some(WIN_TEXT_PRIMARY),
        border_radius: None,
        padding_ratio: 0.22,
    }
}

fn fluent_slider_style(_theme: &Theme, status: slider::Status) -> slider::Style {
    let handle_color = match status {
        slider::Status::Hovered | slider::Status::Dragged => WIN_ACCENT_HOVER,
        slider::Status::Active => WIN_ACCENT,
    };
    slider::Style {
        rail: slider::Rail {
            backgrounds: (
                Background::Color(WIN_ACCENT),
                Background::Color(WIN_CARD_BORDER),
            ),
            width: 4.0,
            border: Border {
                radius: 2.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 8.5 },
            background: Background::Color(handle_color),
            border_width: 3.0,
            border_color: WIN_CARD_BG,
        },
    }
}

fn fluent_pick_list_style(_theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let is_active = matches!(
        status,
        pick_list::Status::Hovered | pick_list::Status::Opened { .. }
    );
    pick_list::Style {
        text_color: WIN_TEXT_PRIMARY,
        placeholder_color: WIN_TEXT_SECONDARY,
        handle_color: WIN_TEXT_SECONDARY,
        background: Background::Color(if is_active {
            WIN_INPUT_HOVER
        } else {
            WIN_INPUT_BG
        }),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: if is_active {
                WIN_ACCENT
            } else {
                WIN_CARD_BORDER
            },
        },
    }
}

fn fluent_text_input_style(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let is_focused = matches!(status, text_input::Status::Focused { .. });
    let is_hovered = matches!(status, text_input::Status::Hovered);
    text_input::Style {
        background: Background::Color(if is_hovered || is_focused {
            WIN_INPUT_HOVER
        } else {
            WIN_INPUT_BG
        }),
        border: Border {
            radius: 6.0.into(),
            width: if is_focused { 1.5 } else { 1.0 },
            color: if is_focused {
                WIN_ACCENT
            } else {
                WIN_CARD_BORDER
            },
        },
        icon: WIN_TEXT_SECONDARY,
        placeholder: WIN_TEXT_SECONDARY,
        value: WIN_TEXT_PRIMARY,
        selection: WIN_ACCENT,
    }
}
