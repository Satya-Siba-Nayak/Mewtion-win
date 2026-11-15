#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, slider, text, text_input,
};
use iced::{Alignment, Color, Element, Length, Task};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn main() -> iced::Result {
    ensure_adb_and_overlay_started();

    iced::application(SettingsApp::default, SettingsApp::update, SettingsApp::view)
        .title("Mewtion Settings")
        .window_size((420.0, 860.0))
        .centered()
        .run()
}

const UI_BG_MAIN: Color = Color::from_rgb(0.12, 0.12, 0.13);
const UI_BG_CARD: Color = Color::from_rgb(0.18, 0.18, 0.20);
const UI_FG_TEXT: Color = Color::from_rgb(0.95, 0.95, 0.96);
const UI_FG_MUTED: Color = Color::from_rgb(0.60, 0.60, 0.64);

const COLOR_WHITE: Color = Color::from_rgb(1.00, 1.00, 1.00);
const COLOR_BLUE: Color = Color::from_rgb(0.00, 0.48, 1.00);
const COLOR_GREEN: Color = Color::from_rgb(0.20, 0.78, 0.35);
const COLOR_RED: Color = Color::from_rgb(1.00, 0.23, 0.19);
const COLOR_YELLOW: Color = Color::from_rgb(1.00, 0.80, 0.00);
const COLOR_PURPLE: Color = Color::from_rgb(0.69, 0.32, 0.87);
const COLOR_CYAN: Color = Color::from_rgb(0.00, 0.78, 0.75);

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
        .unwrap_or_else(|| "Overlay initializing...".to_string())
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
            DotSize::Small => "Small",
            DotSize::Medium => "Medium",
            DotSize::Large => "Large",
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
                MonitorTarget::Primary => "Primary Monitor Only",
                MonitorTarget::All => "All Monitors (Virtual Desktop)",
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
                Orientation::Deg0 => "0° (Standard / Flat)",
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
            status_message: String::new(),
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
    ToggleOverlay,
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

    fn update(&mut self, message: Message) -> Task<Message> {
        self.overlay_running = is_overlay_running();
        self.sensor_status = read_sensor_status();

        match message {
            Message::SizeSelected(size) => self.selected_size = size,
            Message::ColorSelected(color) => self.selected_color = color,
            Message::OpacityChanged(val) => self.opacity = val,
            Message::MarginChanged(val) => self.margin_pct = val,
            Message::SensitivityChanged(val) => self.sensitivity = val,
            Message::AnimModeChanged(mode) => self.anim_mode = mode,
            Message::MonitorTargetChanged(target) => self.monitor_target = target,
            Message::OrientationChanged(orient) => self.orientation = orient,
            Message::InvertXChanged(val) => self.invert_x = val,
            Message::InvertYChanged(val) => self.invert_y = val,
            Message::SimulateMotionChanged(val) => {
                self.simulate_motion = val;
                let _ = self.write_config();
                self.status_message = if val {
                    "Simulate Motion ON (testing dots at desk)".to_string()
                } else {
                    "Simulate Motion OFF (using live sensor)".to_string()
                };
            }
            Message::AutostartChanged(val) => {
                if set_autostart_enabled(val) {
                    self.autostart_windows = val;
                    self.status_message = if val {
                        "Enabled Run on Windows Startup!".to_string()
                    } else {
                        "Removed from Windows Startup.".to_string()
                    };
                } else {
                    self.status_message = "Could not update Windows Startup registry.".to_string();
                }
            }
            Message::IpChanged(ip) => self.phone_ip = ip,
            Message::SetUsbMode => {
                self.phone_ip = "127.0.0.1".to_string();
                let _ = new_silent_command("adb")
                    .args(["forward", "tcp:8765", "tcp:8765"])
                    .output();
                self.status_message = "USB Mode set (127.0.0.1:8765)".to_string();
            }
            Message::AutoDetectHotspot => match detect_windows_gateway_ip() {
                Some(ip) => {
                    self.phone_ip = ip;
                    self.status_message = "Hotspot gateway IP detected!".to_string();
                }
                None => {
                    self.status_message = "Could not detect default gateway.".to_string();
                }
            },
            Message::RefreshStatus => {
                self.overlay_running = is_overlay_running();
                self.sensor_status = read_sensor_status();
                self.status_message = "Status refreshed.".to_string();
            }
            Message::ToggleOverlay => {
                if is_overlay_running() {
                    let _ = new_silent_command("taskkill")
                        .args(["/F", "/IM", "Mewtion.exe"])
                        .output();
                    self.overlay_running = false;
                    self.status_message = "Overlay stopped.".to_string();
                } else if let Some(overlay_path) = find_overlay_exe() {
                    match new_silent_command(overlay_path.to_str().unwrap_or("Mewtion.exe")).spawn()
                    {
                        Ok(_) => {
                            self.overlay_running = true;
                            self.status_message = "Overlay started!".to_string();
                        }
                        Err(_) => {
                            self.status_message = "Failed to launch Mewtion.exe.".to_string();
                        }
                    }
                } else {
                    self.status_message = "Mewtion.exe not found next to control_panel.exe".to_string();
                }
            }
            Message::SaveClicked => match self.write_config() {
                Ok(_) => self.status_message = "Settings applied live!".to_string(),
                Err(_) => self.status_message = "Error saving config".to_string(),
            },
        }
        Task::none()
    }

    fn view(&self) -> Element<Message> {
        let title = text("Mewtion Settings").size(22).color(UI_FG_TEXT);

        let toggle_label = if self.overlay_running {
            "Stop Overlay"
        } else {
            "Start Overlay"
        };
        let toggle_color = if self.overlay_running {
            COLOR_RED
        } else {
            COLOR_GREEN
        };
        let toggle_btn = button(text(toggle_label).size(12).color(Color::WHITE))
            .on_press(Message::ToggleOverlay)
            .padding([6, 12])
            .style(move |_theme, _status| button::Style {
                background: Some(toggle_color.into()),
                border: iced::Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        let header_row =
            row![title, container(toggle_btn).align_x(Alignment::End).width(Length::Fill)]
                .align_y(Alignment::Center);

        let status_color = if !self.overlay_running {
            COLOR_RED
        } else if self.sensor_status.starts_with("Active:")
            || self.sensor_status.starts_with("Connected")
        {
            COLOR_GREEN
        } else {
            COLOR_YELLOW
        };

        let status_label = if self.overlay_running {
            &self.sensor_status
        } else {
            "Overlay is stopped"
        };

        let refresh_btn = button(text("Refresh").size(11).color(UI_FG_TEXT))
            .on_press(Message::RefreshStatus)
            .padding([4, 8])
            .style(move |_theme, _status| button::Style {
                background: Some(UI_BG_MAIN.into()),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        let status_banner = container(
            row![
                text(status_label)
                    .size(12)
                    .color(status_color)
                    .width(Length::Fill),
                refresh_btn
            ]
            .align_y(Alignment::Center)
            .spacing(8),
        )
        .padding(10)
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(UI_BG_CARD.into()),
            border: iced::Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

        let preview_size = self.selected_size.pixels();
        let preview_dot = container(text(""))
            .width(Length::Fixed(preview_size))
            .height(Length::Fixed(preview_size))
            .style(move |_| container::Style {
                background: Some(
                    Color {
                        a: self.opacity,
                        ..self.selected_color.to_color()
                    }
                    .into(),
                ),
                border: iced::Border {
                    radius: (preview_size / 2.0).into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        let preview_area = container(preview_dot)
            .width(Length::Fill)
            .height(Length::Fixed(64.0))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_| container::Style {
                background: Some(UI_BG_CARD.into()),
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        let sizes_row = row![
            self.size_button(DotSize::Small),
            self.size_button(DotSize::Medium),
            self.size_button(DotSize::Large)
        ]
        .spacing(10);
        let colors_row = row![
            self.color_button(DotColor::White),
            self.color_button(DotColor::Blue),
            self.color_button(DotColor::Green),
            self.color_button(DotColor::Red),
            self.color_button(DotColor::Yellow),
            self.color_button(DotColor::Purple),
            self.color_button(DotColor::Cyan)
        ]
        .spacing(10);

        let ip_input = text_input("e.g. 192.168.43.1", &self.phone_ip)
            .on_input(Message::IpChanged)
            .padding(8)
            .style(move |_theme, _status| text_input::Style {
                background: UI_BG_CARD.into(),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                icon: UI_FG_TEXT,
                placeholder: UI_FG_MUTED,
                value: UI_FG_TEXT,
                selection: COLOR_BLUE,
            });

        let usb_btn = button(text("USB Mode").size(12).color(UI_FG_TEXT))
            .on_press(Message::SetUsbMode)
            .padding(8)
            .style(move |_theme, _status| button::Style {
                background: Some(UI_BG_CARD.into()),
                ..Default::default()
            });

        let auto_btn = button(text("Auto-Detect Hotspot").size(12).color(UI_FG_TEXT))
            .on_press(Message::AutoDetectHotspot)
            .padding(8)
            .style(move |_theme, _status| button::Style {
                background: Some(UI_BG_CARD.into()),
                ..Default::default()
            });

        let save_btn = button(
            text("Save & Apply")
                .color(Color::WHITE)
                .width(Length::Fill)
                .align_x(Alignment::Center),
        )
        .on_press(Message::SaveClicked)
        .padding(12)
        .style(|_theme, status| button::Style {
            background: Some(
                if status == button::Status::Hovered {
                    Color::from_rgb(0.05, 0.55, 1.00).into()
                } else {
                    COLOR_BLUE.into()
                },
            ),
            border: iced::Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

        let content = column![
            header_row,
            text("Sensor & Connection Status").size(13).color(UI_FG_MUTED),
            status_banner,
            text("Live Preview").size(13).color(UI_FG_MUTED),
            preview_area,
            text("Connection Target").size(13).color(UI_FG_MUTED),
            row![ip_input, usb_btn, auto_btn].spacing(10),
            text("Dot Size").size(13).color(UI_FG_MUTED),
            sizes_row,
            text("Dot Color").size(13).color(UI_FG_MUTED),
            colors_row,
            text(format!("Opacity: {:.0}%", self.opacity * 100.0))
                .size(13)
                .color(UI_FG_MUTED),
            slider(0.1..=1.0, self.opacity, Message::OpacityChanged).step(0.05),
            text(format!("Edge Margin: {:.0}%", self.margin_pct * 100.0))
                .size(13)
                .color(UI_FG_MUTED),
            slider(0.0..=0.15, self.margin_pct, Message::MarginChanged).step(0.01),
            text(format!("Motion Sensitivity: {:.1}", self.sensitivity))
                .size(13)
                .color(UI_FG_MUTED),
            slider(1.0..=10.0, self.sensitivity, Message::SensitivityChanged).step(0.5),
            text("Animation Behavior").size(13).color(UI_FG_MUTED),
            pick_list(
                &[AnimMode::Fluid, AnimMode::Rigid][..],
                Some(self.anim_mode),
                Message::AnimModeChanged
            )
            .width(Length::Fill),
            text("Display Target").size(13).color(UI_FG_MUTED),
            pick_list(
                &[MonitorTarget::Primary, MonitorTarget::All][..],
                Some(self.monitor_target),
                Message::MonitorTargetChanged
            )
            .width(Length::Fill),
            text("Sensor Orientation & Calibration")
                .size(13)
                .color(UI_FG_MUTED),
            pick_list(
                &[
                    Orientation::Deg0,
                    Orientation::Deg90,
                    Orientation::Deg180,
                    Orientation::Deg270
                ][..],
                Some(self.orientation),
                Message::OrientationChanged
            )
            .width(Length::Fill),
            row![
                checkbox(self.invert_x)
                    .label("Invert Lateral (X)")
                    .on_toggle(Message::InvertXChanged),
                checkbox(self.invert_y)
                    .label("Invert Forward (Y)")
                    .on_toggle(Message::InvertYChanged),
            ]
            .spacing(16),
            text("Diagnostics & System").size(13).color(UI_FG_MUTED),
            checkbox(self.simulate_motion)
                .label("Simulate Motion (Desk Test Mode)")
                .on_toggle(Message::SimulateMotionChanged),
            checkbox(self.autostart_windows)
                .label("Run Mewtion on Windows Startup")
                .on_toggle(Message::AutostartChanged),
            save_btn,
            text(&self.status_message).size(12).color(COLOR_GREEN),
        ]
        .spacing(14)
        .padding(24)
        .max_width(420);

        container(scrollable(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .style(|_| container::Style {
                background: Some(UI_BG_MAIN.into()),
                text_color: Some(UI_FG_TEXT),
                ..Default::default()
            })
            .into()
    }

    fn size_button(&self, size: DotSize) -> Element<Message> {
        let is_selected = self.selected_size == size;
        button(
            text(size.label()).color(if is_selected {
                Color::WHITE
            } else {
                UI_FG_TEXT
            }),
        )
        .on_press(Message::SizeSelected(size))
        .padding([8, 16])
        .style(move |_theme, status| button::Style {
            background: Some(
                if is_selected {
                    COLOR_BLUE.into()
                } else if status == button::Status::Hovered {
                    UI_BG_CARD.into()
                } else {
                    UI_BG_MAIN.into()
                },
            ),
            border: iced::Border {
                radius: 6.0.into(),
                width: 1.0,
                color: if is_selected { COLOR_BLUE } else { UI_BG_CARD },
            },
            ..Default::default()
        })
        .into()
    }

    fn color_button(&self, color: DotColor) -> Element<Message> {
        let is_selected = self.selected_color == color;
        button(text(""))
            .on_press(Message::ColorSelected(color))
            .width(Length::Fixed(26.0))
            .height(Length::Fixed(26.0))
            .style(move |_theme, _status| button::Style {
                background: Some(color.to_color().into()),
                border: iced::Border {
                    radius: 13.0.into(),
                    width: if is_selected { 3.0 } else { 0.0 },
                    color: Color::WHITE,
                },
                ..Default::default()
            })
            .into()
    }
}
