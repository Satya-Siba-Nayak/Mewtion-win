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
use std::sync::{Arc, Mutex};

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
        size: Size::new(560.0, 680.0),
        min_size: Some(Size::new(480.0, 560.0)),
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
            let dist =
                (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt() + dx.max(dy).min(0.0) - 5.0;
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

fn sanitize_ip_input(raw: &str) -> String {
    let trimmed = raw.trim();
    let without_scheme = trimmed
        .strip_prefix("tcp://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .or_else(|| trimmed.strip_prefix("https://"))
        .unwrap_or(trimmed)
        .trim();

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

    // Run ADB port forward and overlay startup check on a background thread
    // so the Control Panel window opens instantaneously with zero startup delay.
    std::thread::spawn(|| {
        let _ = new_silent_command("adb")
            .args(["forward", "tcp:8765", "tcp:8765"])
            .output();

        if !is_overlay_running() {
            if let Some(overlay_path) = find_overlay_exe() {
                let _ = new_silent_command(overlay_path.to_str().unwrap_or("Mewtion.exe")).spawn();
            }
        }
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

fn detect_windows_gateway_ip() -> Option<String> {
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

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavTab {
    Connection,
    Appearance,
    MotionAndSystem,
    Guide,
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
impl std::fmt::Display for SensorSourcePref {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SensorSourcePref::Auto => "Auto (Phone + Built-in)",
                SensorSourcePref::Phone => "Android Phone Only",
                SensorSourcePref::BuiltIn => "Built-in Sensor Only",
            }
        )
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
    active_tab: NavTab,
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
    bg_status: Arc<Mutex<(bool, String)>>,
}

impl Default for SettingsApp {
    fn default() -> Self {
        let path = resolve_config_path();
        let initial_status = read_sensor_status();
        let bg_status = Arc::new(Mutex::new((true, initial_status.clone())));
        let bg_clone = bg_status.clone();

        // Background status poller so the UI thread NEVER spawns tasklist.exe or blocks on I/O!
        std::thread::spawn(move || loop {
            let running = is_overlay_running();
            let status_txt = read_sensor_status();
            if let Ok(mut guard) = bg_clone.lock() {
                *guard = (running, status_txt);
            }
            std::thread::sleep(std::time::Duration::from_millis(1200));
        });

        let mut app = Self {
            active_tab: NavTab::Connection,
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
            overlay_running: true,
            sensor_status: initial_status,
            status_message: "All settings auto-apply in real time".to_string(),
            bg_status,
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
                    "sensor_source" => app.sensor_source = SensorSourcePref::from_str(v),
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
        app
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(NavTab),
    ToggleGuide,
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
    AutoDetectHotspot,
    RefreshStatus,
    ToggleOverlay(bool),
    ResetDefaults,
    OpenUrl(&'static str),
}

impl SettingsApp {
    fn sync_background_status(&mut self) {
        if let Ok(guard) = self.bg_status.try_lock() {
            self.overlay_running = guard.0;
            self.sensor_status = guard.1.clone();
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
        if self.write_config().is_ok() {
            self.status_message = msg.to_string();
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        // Non-blocking status sync from background thread (0ms overhead, never blocks UI!)
        self.sync_background_status();

        match message {
            Message::TabSelected(tab) => {
                self.active_tab = tab;
            }
            Message::ToggleGuide => {
                self.active_tab = if self.active_tab == NavTab::Guide {
                    NavTab::Connection
                } else {
                    NavTab::Guide
                };
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
                let _ = self.write_config();
                self.status_message = format!("Opacity set to {:.0}%", val * 100.0);
            }
            Message::MarginChanged(val) => {
                self.margin_pct = val;
                let _ = self.write_config();
                self.status_message = format!("Edge margin set to {:.0}%", val * 100.0);
            }
            Message::SensitivityChanged(val) => {
                self.sensitivity = val;
                let _ = self.write_config();
                self.status_message = format!("Motion sensitivity set to {:.1}", val);
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
                self.apply_live("Sensor source preference applied live");
            }
            Message::OrientationChanged(orient) => {
                self.orientation = orient;
                self.apply_live("Sensor orientation applied live");
            }
            Message::InvertXChanged(val) => {
                self.invert_x = val;
                self.apply_live("Lateral (X) axis inversion applied");
            }
            Message::InvertYChanged(val) => {
                self.invert_y = val;
                self.apply_live("Forward (Y) axis inversion applied");
            }
            Message::SimulateMotionChanged(val) => {
                self.simulate_motion = val;
                self.apply_live(if val {
                    "Desk Test Mode active — simulating vehicle motion"
                } else {
                    "Desk Test Mode stopped immediately"
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
                let clean = sanitize_ip_input(&self.phone_ip);
                let _ = self.write_config();
                self.status_message = format!("Connecting live to {clean}:8765...");
            }
            Message::SetUsbMode => {
                self.phone_ip = "127.0.0.1".to_string();
                self.sensor_source = SensorSourcePref::Phone;
                self.apply_live("USB Mode (127.0.0.1:8765) activated — forwarding ADB port...");
                std::thread::spawn(|| {
                    let _ = new_silent_command("adb")
                        .args(["forward", "tcp:8765", "tcp:8765"])
                        .output();
                });
            }
            Message::AutoDetectHotspot => {
                if let Some(ip) = detect_windows_gateway_ip() {
                    self.phone_ip = ip.clone();
                    self.sensor_source = SensorSourcePref::Phone;
                    self.apply_live(&format!("Hotspot gateway detected ({ip}:8765) & connected!"));
                } else {
                    self.status_message =
                        "Could not detect gateway IP — enter your phone's IP manually".to_string();
                }
            }
            Message::RefreshStatus => {
                self.sensor_status = read_sensor_status();
                self.status_message = "Sensor connection status refreshed".to_string();
            }
            Message::ToggleOverlay(should_run) => {
                self.overlay_running = should_run;
                let bg_clone = self.bg_status.clone();
                if !should_run {
                    self.status_message = "Overlay stopped".to_string();
                    std::thread::spawn(move || {
                        let _ = new_silent_command("taskkill")
                            .args(["/F", "/IM", "Mewtion.exe"])
                            .output();
                        if let Ok(mut guard) = bg_clone.lock() {
                            guard.0 = false;
                        }
                    });
                } else {
                    self.status_message = "Overlay launched".to_string();
                    std::thread::spawn(move || {
                        if let Some(overlay_path) = find_overlay_exe() {
                            let _ = new_silent_command(
                                overlay_path.to_str().unwrap_or("Mewtion.exe"),
                            )
                            .spawn();
                            if let Ok(mut guard) = bg_clone.lock() {
                                guard.0 = true;
                            }
                        }
                    });
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
                self.status_message = format!("Opened guide in browser: {url}");
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        // === 1. COMPACT HERO HEADER BAR WITH `?` SETUP GUIDE BUTTON ===
        let is_connected = self.sensor_status.starts_with("Connected")
            || self.sensor_status.starts_with("Active:");

        let status_dot_color = if !self.overlay_running {
            COLOR_RED
        } else if is_connected {
            COLOR_GREEN
        } else {
            COLOR_YELLOW
        };

        let status_dot = container(text(""))
            .width(Length::Fixed(9.0))
            .height(Length::Fixed(9.0))
            .style(move |_| container::Style {
                background: Some(status_dot_color.into()),
                border: Border {
                    radius: 4.5.into(),
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
            row![
                text("Mewtion")
                    .font(FONT_SEGOE_SEMIBOLD)
                    .size(21)
                    .color(WIN_TEXT_PRIMARY),
                container(
                    text("Windows 11")
                        .font(FONT_SEGOE_SEMIBOLD)
                        .size(10)
                        .color(WIN_ACCENT)
                )
                .padding([2, 7])
                .style(|_| container::Style {
                    background: Some(WIN_CARD_HEADER.into()),
                    border: Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                status_dot,
                text(status_str).size(12).color(WIN_TEXT_SECONDARY),
                button(text("↻").size(12).color(WIN_TEXT_PRIMARY))
                    .on_press(Message::RefreshStatus)
                    .padding([2, 7])
                    .style(subtle_button_style),
            ]
            .spacing(7)
            .align_y(Alignment::Center),
        ]
        .spacing(4)
        .width(Length::Fill);

        let is_guide_open = self.active_tab == NavTab::Guide;
        let help_btn = button(
            row![
                text("?").font(FONT_SEGOE_SEMIBOLD).size(13).color(if is_guide_open {
                    WIN_ACCENT_DARK_TEXT
                } else {
                    WIN_ACCENT
                }),
                text("Setup Guide").size(12).color(if is_guide_open {
                    WIN_ACCENT_DARK_TEXT
                } else {
                    WIN_TEXT_PRIMARY
                }),
            ]
            .spacing(5)
            .align_y(Alignment::Center),
        )
        .on_press(Message::ToggleGuide)
        .padding([6, 11])
        .style(move |theme, status| {
            if is_guide_open {
                accent_button_style(theme, status)
            } else {
                subtle_button_style(theme, status)
            }
        });

        let overlay_switch = row![
            help_btn,
            column![
                text(if self.overlay_running {
                    "Overlay On"
                } else {
                    "Overlay Off"
                })
                .size(11)
                .color(WIN_TEXT_SECONDARY),
                toggler(self.overlay_running)
                    .on_toggle(Message::ToggleOverlay)
                    .size(20.0)
                    .style(fluent_toggler_style),
            ]
            .spacing(3)
            .align_x(Alignment::End),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let header_card = fluent_card(
            row![hero_left, overlay_switch]
                .align_y(Alignment::Center)
                .into(),
        );

        // === 2. SEGMENTED NAVIGATION BAR (Eliminates Long Page Scrolling) ===
        let nav_bar = container(
            row![
                self.nav_tab_button(NavTab::Connection, "Connection & Sensor"),
                self.nav_tab_button(NavTab::Appearance, "Appearance"),
                self.nav_tab_button(NavTab::MotionAndSystem, "Motion & System"),
                self.nav_tab_button(NavTab::Guide, "? Connection Guide"),
            ]
            .spacing(6),
        )
        .width(Length::Fill)
        .padding(4)
        .style(|_| container::Style {
            background: Some(WIN_INPUT_BG.into()),
            border: Border {
                radius: 8.0.into(),
                width: 1.0,
                color: WIN_CARD_BORDER,
            },
            ..Default::default()
        });

        // === 3. ACTIVE TAB CONTENT ===
        let tab_content: Element<'_, Message> = match self.active_tab {
            NavTab::Connection => self.view_connection_tab(),
            NavTab::Appearance => self.view_appearance_tab(),
            NavTab::MotionAndSystem => self.view_motion_system_tab(),
            NavTab::Guide => self.view_guide_tab(),
        };

        // === 4. LIVE STATUS FOOTER BAR (No Manual "Apply" Needed!) ===
        let footer_row = row![
            text(&self.status_message)
                .size(12)
                .color(WIN_ACCENT)
                .width(Length::Fill),
            button(text("Reset Defaults").size(11).color(WIN_TEXT_SECONDARY))
                .on_press(Message::ResetDefaults)
                .padding([5, 10])
                .style(subtle_button_style),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let main_column = column![
            header_card,
            nav_bar,
            scrollable(container(tab_content).width(Length::Fill)).height(Length::Fill),
            footer_row,
        ]
        .spacing(10)
        .padding([16, 20]);

        container(main_column)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(WIN_BG_MICA.into()),
                text_color: Some(WIN_TEXT_PRIMARY),
                ..Default::default()
            })
            .into()
    }

    fn view_connection_tab(&self) -> Element<'_, Message> {
        let ip_input = text_input("e.g. 192.168.43.1 or 192.168.1.42", &self.phone_ip)
            .on_input(Message::IpChanged)
            .padding([8, 12])
            .size(13)
            .style(fluent_text_input_style);

        let connection_card = fluent_card(
            column![
                setting_row(
                    "Sensor Source",
                    "Auto uses your Android phone whenever connected, or falls back to built-in sensors",
                    pick_list(
                        &[
                            SensorSourcePref::Auto,
                            SensorSourcePref::Phone,
                            SensorSourcePref::BuiltIn,
                        ][..],
                        Some(self.sensor_source),
                        Message::SensorSourceChanged,
                    )
                    .width(Length::Fixed(195.0))
                    .padding([6, 10])
                    .style(fluent_pick_list_style)
                    .into(),
                ),
                divider(),
                column![
                    row![
                        column![
                            text("Android Companion IP Address")
                                .font(FONT_SEGOE_SEMIBOLD)
                                .size(13)
                                .color(WIN_TEXT_PRIMARY),
                            text("Applies automatically as you type — port 8765 is handled for you")
                                .size(11)
                                .color(WIN_TEXT_SECONDARY),
                        ]
                        .spacing(2)
                        .width(Length::Fill),
                        button(
                            text("Need Help? (?)")
                                .font(FONT_SEGOE_SEMIBOLD)
                                .size(11)
                                .color(WIN_ACCENT)
                        )
                        .on_press(Message::TabSelected(NavTab::Guide))
                        .padding([5, 10])
                        .style(subtle_button_style),
                    ]
                    .align_y(Alignment::Center),
                    row![
                        ip_input,
                        button(
                            text("Detect Hotspot")
                                .font(FONT_SEGOE_SEMIBOLD)
                                .size(12)
                                .color(WIN_ACCENT_DARK_TEXT)
                        )
                        .on_press(Message::AutoDetectHotspot)
                        .padding([8, 12])
                        .style(accent_button_style),
                        button(text("USB (ADB)").size(12).color(WIN_TEXT_PRIMARY))
                            .on_press(Message::SetUsbMode)
                            .padding([8, 12])
                            .style(subtle_button_style),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                ]
                .spacing(8),
                divider(),
                setting_row(
                    "Simulate Motion (Desk Test Mode)",
                    "Test the overlay dots right now at your desk without a phone connected",
                    toggler(self.simulate_motion)
                        .on_toggle(Message::SimulateMotionChanged)
                        .size(20.0)
                        .style(fluent_toggler_style)
                        .into(),
                ),
            ]
            .spacing(12)
            .into(),
        );

        let calibration_card = fluent_card(
            column![
                setting_row(
                    "Phone / Sensor Orientation",
                    "Rotate axes if your phone is placed sideways in the vehicle",
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
                    .width(Length::Fixed(195.0))
                    .padding([6, 10])
                    .style(fluent_pick_list_style)
                    .into(),
                ),
                divider(),
                setting_row(
                    "Invert Lateral Axis (X)",
                    "Reverse left/right dot movement during turns",
                    toggler(self.invert_x)
                        .on_toggle(Message::InvertXChanged)
                        .size(20.0)
                        .style(fluent_toggler_style)
                        .into(),
                ),
                divider(),
                setting_row(
                    "Invert Forward Axis (Y)",
                    "Reverse forward/backward dot movement during braking & acceleration",
                    toggler(self.invert_y)
                        .on_toggle(Message::InvertYChanged)
                        .size(20.0)
                        .style(fluent_toggler_style)
                        .into(),
                ),
            ]
            .spacing(11)
            .into(),
        );

        column![connection_card, calibration_card]
            .spacing(10)
            .into()
    }

    fn view_appearance_tab(&self) -> Element<'_, Message> {
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
                .spacing(6)
                .align_x(Alignment::Center),
                container(
                    column![
                        text("LIVE OVERLAY PREVIEW")
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
                .spacing(6)
                .align_x(Alignment::Center),
            ]
            .align_y(Alignment::Center)
            .padding([8, 16]),
        )
        .width(Length::Fill)
        .height(Length::Fixed(66.0))
        .style(|_| container::Style {
            background: Some(WIN_INPUT_BG.into()),
            border: Border {
                radius: 6.0.into(),
                width: 1.0,
                color: WIN_CARD_BORDER,
            },
            ..Default::default()
        });

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

        fluent_card(
            column![
                mini_screen,
                divider(),
                setting_row(
                    "Dot Size",
                    "Diameter of edge motion cues on screen",
                    sizes_row.into()
                ),
                divider(),
                setting_row(
                    "Dot Color",
                    "High-contrast tint for your desktop wallpaper",
                    colors_row.into()
                ),
                divider(),
                slider_setting_row(
                    "Opacity",
                    "Transparency of motion dots",
                    format!("{:.0}%", self.opacity * 100.0),
                    slider(0.1..=1.0, self.opacity, Message::OpacityChanged)
                        .step(0.05_f32)
                        .style(fluent_slider_style)
                        .into(),
                ),
                divider(),
                slider_setting_row(
                    "Screen Edge Margin",
                    "Distance from the left and right edges of your display",
                    format!("{:.0}%", self.margin_pct * 100.0),
                    slider(0.0..=0.15, self.margin_pct, Message::MarginChanged)
                        .step(0.01_f32)
                        .style(fluent_slider_style)
                        .into(),
                ),
            ]
            .spacing(12)
            .into(),
        )
    }

    fn view_motion_system_tab(&self) -> Element<'_, Message> {
        let motion_card = fluent_card(
            column![
                slider_setting_row(
                    "Motion Sensitivity",
                    "Lower values react more strongly to gentle turns and acceleration",
                    format!("{:.1}", self.sensitivity),
                    slider(1.0..=10.0, self.sensitivity, Message::SensitivityChanged)
                        .step(0.5_f32)
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
                    .padding([6, 10])
                    .style(fluent_pick_list_style)
                    .into(),
                ),
                divider(),
                setting_row(
                    "Display Target",
                    "Render cues on your primary monitor or across all monitors",
                    pick_list(
                        &[MonitorTarget::Primary, MonitorTarget::All][..],
                        Some(self.monitor_target),
                        Message::MonitorTargetChanged,
                    )
                    .width(Length::Fixed(185.0))
                    .padding([6, 10])
                    .style(fluent_pick_list_style)
                    .into(),
                ),
            ]
            .spacing(12)
            .into(),
        );

        let system_card = fluent_card(
            column![
                setting_row(
                    "Start Mewtion with Windows",
                    "Automatically launch the motion cues overlay when you sign in",
                    toggler(self.autostart_windows)
                        .on_toggle(Message::AutostartChanged)
                        .size(20.0)
                        .style(fluent_toggler_style)
                        .into(),
                ),
                divider(),
                setting_row(
                    "Simulate Motion (Desk Test Mode)",
                    "Preview smooth figure-8 vehicle motion while stationary",
                    toggler(self.simulate_motion)
                        .on_toggle(Message::SimulateMotionChanged)
                        .size(20.0)
                        .style(fluent_toggler_style)
                        .into(),
                ),
            ]
            .spacing(12)
            .into(),
        );

        column![motion_card, system_card].spacing(10).into()
    }

    fn view_guide_tab(&self) -> Element<'_, Message> {
        let companion_card = fluent_card(
            row![
                column![
                    text("Step 0: Install Mewtion-Android on Your Phone")
                        .font(FONT_SEGOE_SEMIBOLD)
                        .size(13)
                        .color(WIN_TEXT_PRIMARY),
                    text("Open the app on your Android phone and tap 'Start Service' so it listens on port 8765.")
                        .size(11)
                        .color(WIN_TEXT_SECONDARY),
                ]
                .spacing(2)
                .width(Length::Fill),
                button(
                    text("Download App ↗")
                        .font(FONT_SEGOE_SEMIBOLD)
                        .size(11)
                        .color(WIN_ACCENT_DARK_TEXT)
                )
                .on_press(Message::OpenUrl(
                    "https://github.com/aayuxh-vim/Mewtion-Android/releases"
                ))
                .padding([6, 12])
                .style(accent_button_style),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .into(),
        );

        let hotspot_card = fluent_card(
            column![
                row![
                    column![
                        text("Method 1: Mobile Hotspot (Easiest Wireless)")
                            .font(FONT_SEGOE_SEMIBOLD)
                            .size(13)
                            .color(WIN_ACCENT),
                        text("1. Turn on Mobile Hotspot on your Android phone and connect your Windows laptop to it.")
                            .size(11)
                            .color(WIN_TEXT_PRIMARY),
                        text("2. Open Mewtion-Android on your phone and tap 'Start Service'.")
                            .size(11)
                            .color(WIN_TEXT_PRIMARY),
                        text("3. Click 'Auto-Detect Hotspot' — Mewtion will find your phone's gateway IP and connect!")
                            .size(11)
                            .color(WIN_TEXT_SECONDARY),
                    ]
                    .spacing(3)
                    .width(Length::Fill),
                    button(
                        text("Auto-Detect Hotspot")
                            .font(FONT_SEGOE_SEMIBOLD)
                            .size(11)
                            .color(WIN_ACCENT_DARK_TEXT)
                    )
                    .on_press(Message::AutoDetectHotspot)
                    .padding([7, 12])
                    .style(accent_button_style),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            ]
            .into(),
        );

        let wifi_card = fluent_card(
            column![
                text("Method 2: Same Wi-Fi Network")
                    .font(FONT_SEGOE_SEMIBOLD)
                    .size(13)
                    .color(WIN_ACCENT),
                text("1. Connect both your Windows PC and Android phone to the same Wi-Fi router.")
                    .size(11)
                    .color(WIN_TEXT_PRIMARY),
                text("2. Look at the IP address shown inside the Mewtion-Android app (e.g. 192.168.1.42:8765).")
                    .size(11)
                    .color(WIN_TEXT_PRIMARY),
                text("3. Type that IP in the 'Connection & Sensor' tab (you can include or omit :8765 — it connects live!).")
                    .size(11)
                    .color(WIN_TEXT_PRIMARY),
                text("Tip: Set your Wi-Fi network profile in Windows Settings to 'Private' so Windows Firewall doesn't block it.")
                    .size(11)
                    .color(WIN_TEXT_SECONDARY),
            ]
            .spacing(3)
            .into(),
        );

        let usb_card = fluent_card(
            column![
                text("Method 3: USB Cable via ADB (Zero Latency + Charges Phone)")
                    .font(FONT_SEGOE_SEMIBOLD)
                    .size(13)
                    .color(WIN_ACCENT),
                text("1. On your Android phone, open Settings → About Phone and tap 'Build Number' 7 times.")
                    .size(11)
                    .color(WIN_TEXT_PRIMARY),
                text("2. Go to Settings → System → Developer Options and enable 'USB Debugging'.")
                    .size(11)
                    .color(WIN_TEXT_PRIMARY),
                text("3. Install Google SDK Platform-Tools (ADB) on Windows, plug in your phone via USB, and tap 'Allow' on the phone prompt.")
                    .size(11)
                    .color(WIN_TEXT_PRIMARY),
                text("4. Click 'Activate USB Mode (ADB)' below to forward port 8765 over USB (127.0.0.1).")
                    .size(11)
                    .color(WIN_TEXT_SECONDARY),
                row![
                    button(
                        text("USB Debugging Guide ↗")
                            .size(11)
                            .color(WIN_TEXT_PRIMARY)
                    )
                    .on_press(Message::OpenUrl(
                        "https://developer.android.com/studio/debug/dev-options"
                    ))
                    .padding([6, 10])
                    .style(subtle_button_style),
                    button(
                        text("Download ADB Tools ↗")
                            .size(11)
                            .color(WIN_TEXT_PRIMARY)
                    )
                    .on_press(Message::OpenUrl(
                        "https://developer.android.com/tools/releases/platform-tools"
                    ))
                    .padding([6, 10])
                    .style(subtle_button_style),
                    button(
                        text("Activate USB Mode (ADB)")
                            .font(FONT_SEGOE_SEMIBOLD)
                            .size(11)
                            .color(WIN_ACCENT_DARK_TEXT)
                    )
                    .on_press(Message::SetUsbMode)
                    .padding([6, 12])
                    .style(accent_button_style),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(5)
            .into(),
        );

        column![companion_card, hotspot_card, wifi_card, usb_card]
            .spacing(8)
            .into()
    }

    fn nav_tab_button(&self, tab: NavTab, label: &'static str) -> Element<'_, Message> {
        let is_selected = self.active_tab == tab;
        button(
            container(
                text(label)
                    .font(if is_selected {
                        FONT_SEGOE_SEMIBOLD
                    } else {
                        FONT_SEGOE
                    })
                    .size(12)
                    .color(if is_selected {
                        WIN_ACCENT_DARK_TEXT
                    } else {
                        WIN_TEXT_PRIMARY
                    }),
            )
            .width(Length::Fill)
            .center_x(Length::Fill),
        )
        .on_press(Message::TabSelected(tab))
        .width(Length::Fill)
        .padding([7, 8])
        .style(move |_theme, status| button::Style {
            background: Some(
                if is_selected {
                    WIN_ACCENT.into()
                } else if status == button::Status::Hovered {
                    WIN_INPUT_HOVER.into()
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

    fn segmented_size_button(&self, size: DotSize) -> Element<'_, Message> {
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

fn fluent_card(content: Element<'_, Message>) -> Element<'_, Message> {
    container(content)
        .width(Length::Fill)
        .padding(14)
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
