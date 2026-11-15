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

struct AppState {
    target_x: f64,
    target_y: f64,
    current_x: f64,
    current_y: f64,
    particles: Vec<Particle>,
    rng: Rng,
    config: MewtionConfig,
    tick_count: u32,
}

impl AppState {
    fn new(config: MewtionConfig) -> Self {
        Self {
            target_x: 0.0,
            target_y: 0.0,
            current_x: 0.0,
            current_y: 0.0,
            particles: Vec::new(),
            rng: Rng::new(),
            config,
            tick_count: 0,
        }
    }

    fn step(
        &mut self,
        receiver: &std::sync::mpsc::Receiver<MotionSample>,
        shared_ip: &Arc<Mutex<String>>,
        width: f64,
        height: f64,
    ) -> ((f32, f32, f32), Vec<dotview::Dot>) {
        const GYRO_YAW_WEIGHT: f32 = 0.8;

        self.tick_count = self.tick_count.wrapping_add(1);
        if self.tick_count % 60 == 0 {
            self.config = MewtionConfig::load();

            // Push any IP changes live to the network thread
            if let Ok(mut locked_ip) = shared_ip.lock() {
                *locked_ip = self.config.phone_ip.clone();
            }
        }

        let current_sens = (self.config.sensitivity as f32).max(0.1);

        while let Ok(sample) = receiver.try_recv() {
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

            let combined_lateral = (ax / current_sens) + (gz * GYRO_YAW_WEIGHT);
            let combined_forward = ay / current_sens;

            self.target_x = (combined_lateral as f64).clamp(-1.0, 1.0);
            self.target_y = (combined_forward as f64).clamp(-1.0, 1.0);
        }

        self.current_x += (self.target_x - self.current_x) * 0.05;
        self.current_y += (self.target_y - self.current_y) * 0.05;

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
    use windows::core::w;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetSystemMetrics, KillTimer,
        LoadCursorW, PeekMessageW, PostQuitMessage, RegisterClassW, SetTimer, SetWindowPos,
        ShowWindow, TranslateMessage, HWND_TOPMOST, IDC_ARROW, MSG, PM_REMOVE, SM_CXSCREEN,
        SM_CYSCREEN, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOWNOACTIVATE, WM_DESTROY,
        WM_DISPLAYCHANGE, WM_QUIT, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
    };

    const TIMER_ID_FRAME: usize = 1;

    pub unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_DESTROY => {
                let _ = KillTimer(hwnd, TIMER_ID_FRAME);
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    pub fn run() {
        unsafe {
            // Opt into Per-Monitor V2 DPI awareness so dots render at crisp native physical resolution.
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

            let hinstance = GetModuleHandleW(None).expect("Failed to get module handle");
            let class_name = w!("MewtionOverlayWindowClass");

            let wc = WNDCLASSW {
                lpfnWndProc: Some(wnd_proc),
                hInstance: hinstance.into(),
                lpszClassName: class_name,
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                ..Default::default()
            };
            RegisterClassW(&wc);

            let mut screen_w = GetSystemMetrics(SM_CXSCREEN).max(1);
            let mut screen_h = GetSystemMetrics(SM_CYSCREEN).max(1);

            // WS_EX_LAYERED | WS_EX_TRANSPARENT: 100% OS click-through + per-pixel alpha
            // WS_EX_TOPMOST: Always on top of normal windows
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
                0,
                0,
                screen_w,
                screen_h,
                None,
                None,
                hinstance,
                None,
            )
            .expect("Failed to create Win32 overlay window");

            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);

            let loaded_config = MewtionConfig::load();
            let shared_ip = Arc::new(Mutex::new(loaded_config.phone_ip.clone()));
            let mut state = AppState::new(loaded_config);

            let mut dot_view =
                DotView::new(screen_w, screen_h).expect("Failed to allocate layered DIB canvas");

            let (sender, receiver) = std::sync::mpsc::channel::<MotionSample>();
            let tcp_shared_ip = shared_ip.clone();

            // Prefer the machine's built-in Windows sensors; fall back to the Android companion app over TCP.
            std::thread::spawn(move || match win_sensor::WinSensorSource::discover() {
                Some(source) => {
                    println!("Mewtion: using built-in Windows sensors ({})", source.describe());
                    win_sensor::run_win_sensor_blocking(source, move |sample| {
                        let _ = sender.send(sample);
                    });
                }
                None => {
                    println!(
                        "Mewtion: no built-in Windows accelerometer found, waiting for companion app over TCP"
                    );
                    tcp::run_tcp_bridge_blocking(tcp_shared_ip, move |sample| {
                        let _ = sender.send(sample);
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

                    if msg.message == WM_DISPLAYCHANGE {
                        let new_w = GetSystemMetrics(SM_CXSCREEN).max(1);
                        let new_h = GetSystemMetrics(SM_CYSCREEN).max(1);
                        if new_w != screen_w || new_h != screen_h {
                            screen_w = new_w;
                            screen_h = new_h;
                            let _ = SetWindowPos(
                                hwnd,
                                HWND_TOPMOST,
                                0,
                                0,
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
                        dot_view.render(hwnd, color, &dots);

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

                // Wait briefly until next message or timer tick to keep idle CPU near 0%
                windows::Win32::UI::WindowsAndMessaging::WaitMessage().ok();
            }
        }
    }
}

fn main() {
    #[cfg(windows)]
    win_app::run();

    #[cfg(not(windows))]
    {
        eprintln!("Mewtion-win is a Windows-native port. Build and run on Windows (or target x86_64-pc-windows-msvc).");
    }
}
