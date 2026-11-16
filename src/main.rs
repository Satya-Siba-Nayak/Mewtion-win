#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod dotview;
mod sensor;
mod tcp;
mod win_sensor;

use config::MewtionConfig;
#[cfg(windows)]
use dotview::DotView;
use sensor::MotionSample;
use std::sync::{Arc, Mutex};

struct Rng {
    state: u32,
}
impl Rng {
    fn new() -> Self {
        Rng { state: 42 }
    }
    fn next_f64(&mut self) -> f64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        (self.state as f64) / (u32::MAX as f64)
    }
}

struct Particle {
    nx: f64,
    ny: f64,
    life: f64,
    fade_rate: f64,
    size: f64,
    is_anchor: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SampleOrigin {
    PhoneTcp,
    WinSensor,
}

struct AppState {
    target_x: f64,
    target_y: f64,
    current_x: f64,
    current_y: f64,
    particles: Vec<Particle>,
    rng: Rng,
    config: MewtionConfig,
    tick_count: u32,
    paused: bool,
    was_simulating: bool,
    last_phone_sample_tick: Option<u32>,
}

impl AppState {
    fn new(config: MewtionConfig) -> Self {
        let was_simulating = config.simulate_motion;
        Self {
            target_x: 0.0,
            target_y: 0.0,
            current_x: 0.0,
            current_y: 0.0,
            particles: Vec::new(),
            rng: Rng::new(),
            config,
            tick_count: 0,
            paused: false,
            was_simulating,
            last_phone_sample_tick: None,
        }
    }

    fn stop_simulation_immediately(&mut self) {
        self.target_x = 0.0;
        self.target_y = 0.0;
        self.current_x = 0.0;
        self.current_y = 0.0;
        self.particles.retain(|p| p.is_anchor);
    }

    fn step(
        &mut self,
        receiver: &std::sync::mpsc::Receiver<(SampleOrigin, MotionSample)>,
        shared_ip: &Arc<Mutex<String>>,
        width: f64,
        height: f64,
    ) -> ((f32, f32, f32), Vec<dotview::Dot>) {
        const GYRO_YAW_WEIGHT: f32 = 0.8;

        self.tick_count = self.tick_count.wrapping_add(1);
        // Reload config every 10 ticks (~160 ms) for near-instant UI responsiveness
        if self.tick_count % 10 == 0 {
            self.config = MewtionConfig::load();

            // Push any IP changes live to the network thread
            if let Ok(mut locked_ip) = shared_ip.lock() {
                if *locked_ip != self.config.phone_ip {
                    *locked_ip = self.config.phone_ip.clone();
                }
            }
        }

        // If Simulate Motion was just turned off, immediately stop motion and clear flying dots
        if self.was_simulating && !self.config.simulate_motion {
            self.stop_simulation_immediately();
        }
        self.was_simulating = self.config.simulate_motion;

        let current_sens = (self.config.sensitivity as f32).max(0.1);
        let mut received_sample = false;

        while let Ok((origin, sample)) = receiver.try_recv() {
            if origin == SampleOrigin::PhoneTcp {
                self.last_phone_sample_tick = Some(self.tick_count);
            }

            if self.config.simulate_motion {
                continue;
            }

            // Respect user's sensor_source preference ("Auto", "Phone", "BuiltIn")
            let src_pref = self.config.sensor_source.as_str();
            if src_pref.eq_ignore_ascii_case("Phone") && origin != SampleOrigin::PhoneTcp {
                continue;
            }
            if src_pref.eq_ignore_ascii_case("BuiltIn") && origin != SampleOrigin::WinSensor {
                continue;
            }
            // In "Auto" mode, if a phone sent a sample within the last ~2 seconds (120 ticks),
            // ignore built-in sensor readings so a stationary laptop sensor never overrides the phone.
            if origin == SampleOrigin::WinSensor {
                if let Some(last_tick) = self.last_phone_sample_tick {
                    if self.tick_count.wrapping_sub(last_tick) < 120 {
                        continue;
                    }
                }
            }

            received_sample = true;

            // Clamp tiny sensor fluctuations to exactly 0.0 to prevent drifting
            // when the device or phone is completely still on a table.
            let mut ax = sample.ax;
            let mut ay = sample.ay;
            let mut gz = sample.gz;

            if ax.abs() < 0.15 {
                ax = 0.0;
            }
            if ay.abs() < 0.15 {
                ay = 0.0;
            }
            if gz.abs() < 0.05 {
                gz = 0.0;
            }

            let (tx, ty) = self.config.transform_axes(ax, ay);

            let combined_lateral = (tx / current_sens) + (gz * GYRO_YAW_WEIGHT);
            let combined_forward = ty / current_sens;

            self.target_x = (combined_lateral as f64).clamp(-1.0, 1.0);
            self.target_y = (combined_forward as f64).clamp(-1.0, 1.0);
        }

        if self.config.simulate_motion {
            // Smooth figure-8 synthetic vehicle motion so users can test at a desk
            let t = (self.tick_count as f64) * 0.04;
            let sim_ax = (t.sin() * 2.2) as f32;
            let sim_ay = ((t * 0.5).cos() * 1.8) as f32;
            let (tx, ty) = self.config.transform_axes(sim_ax, sim_ay);
            self.target_x = ((tx / current_sens) as f64).clamp(-1.0, 1.0);
            self.target_y = ((ty / current_sens) as f64).clamp(-1.0, 1.0);
        } else if !received_sample {
            // Gently decay target toward 0 when no new sample arrived this tick
            // so a disconnected sensor never leaves dots drifting indefinitely.
            self.target_x *= 0.90;
            self.target_y *= 0.90;
            if self.target_x.abs() < 0.005 {
                self.target_x = 0.0;
            }
            if self.target_y.abs() < 0.005 {
                self.target_y = 0.0;
            }
        }

        if self.paused {
            let (r, g, b) = self.config.color_rgb;
            return ((r as f32, g as f32, b as f32), Vec::new());
        }

        self.current_x += (self.target_x - self.current_x) * 0.08;
        self.current_y += (self.target_y - self.current_y) * 0.08;

        let speed = (self.current_x.powi(2) + self.current_y.powi(2)).sqrt();
        let is_moving = speed > 0.02;
        let margin = self.config.margin_pct;

        if is_moving {
            for p in &mut self.particles {
                if p.is_anchor {
                    p.is_anchor = false;
                    p.life = 0.5;
                    p.fade_rate = 0.005 + self.rng.next_f64() * 0.01;
                }
            }

            let target_count = 35 + (speed * 30.0) as usize;
            if self.particles.len() < target_count {
                for _ in 0..3 {
                    if self.particles.len() >= target_count {
                        break;
                    }

                    let is_left = self.rng.next_f64() < 0.5;
                    let nx = if is_left {
                        margin + self.rng.next_f64() * 0.05
                    } else {
                        (1.0 - margin) - self.rng.next_f64() * 0.05
                    };
                    let ny = self.rng.next_f64();

                    let random_fade = 0.005 + self.rng.next_f64() * 0.01;
                    let base_size = self.config.dot_size;
                    let particle_size = if self.config.animation_mode == "Rigid" {
                        base_size
                    } else {
                        (base_size * 0.85) + self.rng.next_f64() * (base_size * 0.3)
                    };

                    self.particles.push(Particle {
                        nx,
                        ny,
                        life: 1.0,
                        fade_rate: random_fade,
                        size: particle_size,
                        is_anchor: false,
                    });
                }
            }
        } else {
            let anchor_count = self.particles.iter().filter(|p| p.is_anchor).count();
            if anchor_count == 0 {
                for i in 0..10 {
                    let ny = (i as f64 + 0.5) / 10.0;
                    let base_size = self.config.dot_size;
                    self.particles.push(Particle {
                        nx: margin,
                        ny,
                        life: 0.0,
                        fade_rate: 0.0,
                        size: base_size,
                        is_anchor: true,
                    });
                    self.particles.push(Particle {
                        nx: 1.0 - margin,
                        ny,
                        life: 0.0,
                        fade_rate: 0.0,
                        size: base_size,
                        is_anchor: true,
                    });
                }
            }
        }

        let cx = self.current_x;
        let cy = self.current_y;

        for p in &mut self.particles {
            if p.is_anchor {
                p.life += (0.75 - p.life) * 0.05;
            } else {
                p.nx += cx * 0.01;
                p.ny += cy * 0.01;
                p.life -= p.fade_rate;
            }
        }

        self.particles
            .retain(|p| p.life > 0.0 && p.nx > -0.1 && p.nx < 1.1 && p.ny > -0.1 && p.ny < 1.1);

        let opacity = self.config.opacity;
        let (r, g, b) = self.config.color_rgb;

        let dots = self
            .particles
            .iter()
            .map(|p| {
                let alpha = if p.is_anchor {
                    p.life
                } else {
                    (p.life * std::f64::consts::PI).sin() * 0.75
                };
                (
                    (p.nx * width) as f32,
                    (p.ny * height) as f32,
                    p.size as f32,
                    (alpha * opacity) as f32,
                )
            })
            .collect();

        ((r as f32, g as f32, b as f32), dots)
    }
}

#[cfg(windows)]
mod win_app {
    use super::*;
    use std::os::windows::process::CommandExt;
    use std::path::PathBuf;
    use std::process::Command;
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{
        ERROR_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, POINT, WPARAM,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    use windows::Win32::UI::Shell::{
        Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
        DispatchMessageW, GetCursorPos, GetSystemMetrics, KillTimer, LoadCursorW, LoadIconW,
        PeekMessageW, PostQuitMessage, RegisterClassW, SetForegroundWindow, SetTimer,
        SetWindowPos, ShowWindow, TrackPopupMenu, TranslateMessage, WaitMessage, HICON,
        HWND_TOPMOST, IDC_ARROW, IDI_APPLICATION, MF_CHECKED, MF_SEPARATOR, MF_STRING, MSG,
        PM_REMOVE, SM_CXSCREEN, SM_CXVIRTUALSCREEN, SM_CYSCREEN, SM_CYVIRTUALSCREEN,
        SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
        SW_SHOWNOACTIVATE, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RIGHTBUTTON, WM_APP, WM_COMMAND,
        WM_DESTROY, WM_DISPLAYCHANGE, WM_LBUTTONDBLCLK, WM_QUIT, WM_RBUTTONUP, WM_TIMER,
        WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
        WS_EX_TRANSPARENT, WS_POPUP,
    };

    const TIMER_ID_FRAME: usize = 1;
    const WM_TRAYICON: u32 = WM_APP + 1;
    const TRAY_ICON_ID: u32 = 1001;

    const IDM_OPEN_SETTINGS: usize = 2001;
    const IDM_TOGGLE_PAUSE: usize = 2002;
    const IDM_TOGGLE_SIMULATE: usize = 2003;
    const IDM_EXIT: usize = 2004;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    fn target_bounds(monitor_mode: &str) -> (i32, i32, i32, i32) {
        unsafe {
            if monitor_mode.eq_ignore_ascii_case("All") {
                let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
                let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
                let w = GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1);
                let h = GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1);
                (x, y, w, h)
            } else {
                let w = GetSystemMetrics(SM_CXSCREEN).max(1);
                let h = GetSystemMetrics(SM_CYSCREEN).max(1);
                (0, 0, w, h)
            }
        }
    }

    fn launch_control_panel() {
        let candidates = [
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("control_panel.exe"))),
            Some(PathBuf::from("control_panel.exe")),
            Some(PathBuf::from("target/release/control_panel.exe")),
        ];
        for cand in candidates.into_iter().flatten() {
            if cand.exists() {
                let _ = Command::new(cand)
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn();
                return;
            }
        }
    }

    unsafe fn add_tray_icon(hwnd: HWND, hicon: HICON) {
        let mut nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ICON_ID,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: hicon,
            ..Default::default()
        };
        let tip = "Mewtion — Vehicle Motion Cues";
        for (dst, src) in nid.szTip.iter_mut().zip(tip.encode_utf16()) {
            *dst = src;
        }
        let _ = Shell_NotifyIconW(NIM_ADD, &nid as *const NOTIFYICONDATAW);
    }

    unsafe fn remove_tray_icon(hwnd: HWND) {
        let nid = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ICON_ID,
            ..Default::default()
        };
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid as *const NOTIFYICONDATAW);
    }

    unsafe fn show_tray_context_menu(hwnd: HWND, paused: bool, simulating: bool) {
        let Ok(hmenu) = CreatePopupMenu() else {
            return;
        };

        let _ = AppendMenuW(
            hmenu,
            MF_STRING,
            IDM_OPEN_SETTINGS,
            w!("Open Mewtion Settings"),
        );

        let pause_flags = if paused {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING
        };
        let _ = AppendMenuW(hmenu, pause_flags, IDM_TOGGLE_PAUSE, w!("Pause Motion Cues"));

        let sim_flags = if simulating {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING
        };
        let _ = AppendMenuW(
            hmenu,
            sim_flags,
            IDM_TOGGLE_SIMULATE,
            w!("Simulate Motion (Desk Test)"),
        );

        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(hmenu, MF_STRING, IDM_EXIT, w!("Exit Mewtion"));

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt as *mut POINT);
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(
            hmenu,
            TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON,
            pt.x,
            pt.y,
            0,
            hwnd,
            None,
        );
        let _ = DestroyMenu(hmenu);
    }

    pub unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_DESTROY => {
                remove_tray_icon(hwnd);
                let _ = KillTimer(hwnd, TIMER_ID_FRAME);
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    pub fn run() {
        unsafe {
            // 1. Single-instance guard so launching Mewtion.exe multiple times never stacks overlays.
            let _mutex = CreateMutexW(None, false, w!("Local\\MewtionOverlaySingletonMutex"));
            if GetLastError() == ERROR_ALREADY_EXISTS {
                return;
            }

            // 2. Opt into Per-Monitor V2 DPI awareness so dots render at native physical pixels.
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

            let hinstance = GetModuleHandleW(None).expect("Failed to get module handle");
            let class_name = w!("MewtionOverlayWindowClass");
            let app_icon = LoadIconW(hinstance, PCWSTR(1usize as *const u16))
                .or_else(|_| LoadIconW(None, IDI_APPLICATION))
                .unwrap_or_default();

            let wc = WNDCLASSW {
                lpfnWndProc: Some(wnd_proc),
                hInstance: hinstance.into(),
                lpszClassName: class_name,
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                hIcon: app_icon,
                ..Default::default()
            };
            RegisterClassW(&wc);

            let loaded_config = MewtionConfig::load();
            let mut active_monitor_mode = loaded_config.monitor_mode.clone();
            let (mut screen_x, mut screen_y, mut screen_w, mut screen_h) =
                target_bounds(&active_monitor_mode);

            // WS_EX_LAYERED | WS_EX_TRANSPARENT: 100% OS click-through + per-pixel alpha
            // WS_EX_TOPMOST: Always on top of normal windows and borderless games
            // WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE: Hidden from taskbar/Alt+Tab and never steals focus
            let ex_style = WS_EX_LAYERED
                | WS_EX_TRANSPARENT
                | WS_EX_TOPMOST
                | WS_EX_TOOLWINDOW
                | WS_EX_NOACTIVATE;

            let hwnd = CreateWindowExW(
                ex_style,
                class_name,
                w!("Mewtion Overlay"),
                WS_POPUP,
                screen_x,
                screen_y,
                screen_w,
                screen_h,
                None,
                None,
                hinstance,
                None,
            )
            .expect("Failed to create Win32 overlay window");

            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            add_tray_icon(hwnd, app_icon);

            let shared_ip = Arc::new(Mutex::new(loaded_config.phone_ip.clone()));
            let mut state = AppState::new(loaded_config);

            let mut dot_view =
                DotView::new(screen_w, screen_h).expect("Failed to allocate layered DIB canvas");

            let (sender, receiver) =
                std::sync::mpsc::channel::<(SampleOrigin, MotionSample)>();
            let tcp_shared_ip = shared_ip.clone();
            let tcp_sender = sender.clone();

            // Always run the TCP phone bridge thread so connecting an Android phone over
            // Wi-Fi, Hotspot, or USB (ADB) always works even on laptops with built-in/dummy HID sensors.
            std::thread::spawn(move || {
                tcp::run_tcp_bridge_blocking(tcp_shared_ip, move |sample| {
                    let _ = tcp_sender.send((SampleOrigin::PhoneTcp, sample));
                });
            });

            // Also poll built-in Windows sensors if the hardware exposes them.
            std::thread::spawn(move || {
                if let Some(source) = win_sensor::WinSensorSource::discover() {
                    let desc = source.describe();
                    println!("Mewtion: discovered built-in Windows sensors ({desc})");
                    win_sensor::run_win_sensor_blocking(source, move |sample| {
                        let _ = sender.send((SampleOrigin::WinSensor, sample));
                    });
                }
            });

            // ~60 FPS frame timer (16 ms)
            let _ = SetTimer(hwnd, TIMER_ID_FRAME, 16, None);

            let mut msg = MSG::default();
            'running: loop {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == WM_QUIT {
                        break 'running;
                    }

                    if msg.message == WM_TRAYICON {
                        let event = (msg.lParam.0 as u32) & 0xFFFF;
                        if event == WM_RBUTTONUP {
                            show_tray_context_menu(
                                hwnd,
                                state.paused,
                                state.config.simulate_motion,
                            );
                        } else if event == WM_LBUTTONDBLCLK {
                            launch_control_panel();
                        }
                        continue;
                    }

                    if msg.message == WM_COMMAND {
                        let cmd_id = msg.wParam.0 & 0xFFFF;
                        match cmd_id {
                            IDM_OPEN_SETTINGS => launch_control_panel(),
                            IDM_TOGGLE_PAUSE => {
                                state.paused = !state.paused;
                            }
                            IDM_TOGGLE_SIMULATE => {
                                state.config.simulate_motion = !state.config.simulate_motion;
                                state.was_simulating = state.config.simulate_motion;
                                if !state.config.simulate_motion {
                                    state.stop_simulation_immediately();
                                }
                                let _ = state.config.save();
                            }
                            IDM_EXIT => {
                                remove_tray_icon(hwnd);
                                break 'running;
                            }
                            _ => {}
                        }
                        continue;
                    }

                    if msg.message == WM_DISPLAYCHANGE {
                        let (nx, ny, nw, nh) = target_bounds(&active_monitor_mode);
                        if nx != screen_x || ny != screen_y || nw != screen_w || nh != screen_h {
                            screen_x = nx;
                            screen_y = ny;
                            screen_w = nw;
                            screen_h = nh;
                            let _ = SetWindowPos(
                                hwnd,
                                HWND_TOPMOST,
                                screen_x,
                                screen_y,
                                screen_w,
                                screen_h,
                                SWP_NOACTIVATE,
                            );
                            if let Some(new_view) = DotView::new(screen_w, screen_h) {
                                dot_view = new_view;
                            }
                        }
                    }

                    if msg.message == WM_TIMER && msg.wParam.0 == TIMER_ID_FRAME {
                        let (color, dots) = state.step(
                            &receiver,
                            &shared_ip,
                            dot_view.width() as f64,
                            dot_view.height() as f64,
                        );

                        // Check if monitor mode changed in mewtion_config.txt
                        if state.config.monitor_mode != active_monitor_mode {
                            active_monitor_mode = state.config.monitor_mode.clone();
                            let (nx, ny, nw, nh) = target_bounds(&active_monitor_mode);
                            if nx != screen_x || ny != screen_y || nw != screen_w || nh != screen_h
                            {
                                screen_x = nx;
                                screen_y = ny;
                                screen_w = nw;
                                screen_h = nh;
                                let _ = SetWindowPos(
                                    hwnd,
                                    HWND_TOPMOST,
                                    screen_x,
                                    screen_y,
                                    screen_w,
                                    screen_h,
                                    SWP_NOACTIVATE,
                                );
                                if let Some(new_view) = DotView::new(screen_w, screen_h) {
                                    dot_view = new_view;
                                }
                            }
                        }

                        dot_view.render(hwnd, screen_x, screen_y, color, &dots);

                        // Periodically re-assert topmost z-order without stealing focus
                        if state.tick_count % 60 == 0 {
                            let _ = SetWindowPos(
                                hwnd,
                                HWND_TOPMOST,
                                0,
                                0,
                                0,
                                0,
                                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                            );
                        }
                    } else {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }

                // Sleep until next message or timer tick to keep idle CPU near 0%
                WaitMessage().ok();
            }

            remove_tray_icon(hwnd);
        }
    }
}

fn main() {
    #[cfg(windows)]
    win_app::run();

    #[cfg(not(windows))]
    {
        eprintln!("Mewtion-win is a Windows-native port. Build and run on Windows (x86_64 or ARM64).");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_simulation_and_simulation_mode() {
        let mut cfg = MewtionConfig::default();
        cfg.simulate_motion = true;
        let mut state = AppState::new(cfg);
        let (_tx, rx) = std::sync::mpsc::channel::<(SampleOrigin, MotionSample)>();
        let shared_ip = Arc::new(Mutex::new("127.0.0.1".to_string()));

        let mut max_dots = 0;
        for _ in 0..8 {
            let (_color, dots) = state.step(&rx, &shared_ip, 1920.0, 1080.0);
            max_dots = max_dots.max(dots.len());
        }
        assert!(max_dots > 0, "Expected simulated motion to spawn dots");

        // Turning off simulate_motion should immediately reset velocity and clear non-anchor particles
        state.config.simulate_motion = false;
        let (_color, after_stop_dots) = state.step(&rx, &shared_ip, 1920.0, 1080.0);
        assert_eq!(state.current_x, 0.0);
        assert_eq!(state.current_y, 0.0);
        assert!(after_stop_dots.len() <= 20);

        state.paused = true;
        let (_color, paused_dots) = state.step(&rx, &shared_ip, 1920.0, 1080.0);
        assert!(paused_dots.is_empty(), "Expected paused state to return zero dots");
    }
}
