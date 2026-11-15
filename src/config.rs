use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct MewtionConfig {
    pub dot_size: f64,
    pub color_rgb: (f64, f64, f64),
    pub opacity: f64,
    pub margin_pct: f64,
    pub sensitivity: f64,
    pub animation_mode: String,
    pub phone_ip: String,
    /// `"Primary"` for the primary display, or `"All"` to span the entire virtual desktop.
    pub monitor_mode: String,
    /// Sensor orientation in degrees clockwise: `0`, `90`, `180`, or `270`.
    pub orientation_deg: i32,
    pub invert_x: bool,
    pub invert_y: bool,
    /// When `true`, synthesizes smooth figure-8 vehicle acceleration so users can
    /// preview and tune the motion cues while sitting at a desk.
    pub simulate_motion: bool,
}

impl Default for MewtionConfig {
    fn default() -> Self {
        Self {
            dot_size: 16.0,
            color_rgb: (1.0, 1.0, 1.0),
            opacity: 0.85,
            margin_pct: 0.03,
            sensitivity: 3.5,
            animation_mode: "Fluid".to_string(),
            phone_ip: "127.0.0.1".to_string(), // Defaults to USB (ADB port-forward)
            monitor_mode: "Primary".to_string(),
            orientation_deg: 0,
            invert_x: false,
            invert_y: false,
            simulate_motion: false,
        }
    }
}

impl MewtionConfig {
    /// Resolves `mewtion_config.txt` in the current working directory if present,
    /// otherwise next to the running executable so double-clicking in Windows
    /// File Explorer always shares the same config between `Mewtion.exe` and `control_panel.exe`.
    pub fn config_path() -> PathBuf {
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

    /// Resolves `mewtion_status.txt` next to `mewtion_config.txt` for live status
    /// reporting from the overlay process to the Control Panel GUI.
    pub fn status_path() -> PathBuf {
        let cfg = Self::config_path();
        if let Some(parent) = cfg.parent() {
            if !parent.as_os_str().is_empty() {
                return parent.join("mewtion_status.txt");
            }
        }
        PathBuf::from("mewtion_status.txt")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if !Path::new(&path).exists() {
            return Self::default();
        }

        let mut config = Self::default();

        if let Ok(contents) = fs::read_to_string(&path) {
            config.apply_str(&contents);
        }
        config
    }

    pub fn apply_str(&mut self, contents: &str) {
        for line in contents.lines() {
            let parts: Vec<&str> = line.split('=').collect();
            if parts.len() != 2 {
                continue;
            }

            let key = parts[0].trim();
            let value = parts[1].trim();

            match key {
                "size" => {
                    if let Ok(v) = value.parse::<f64>() {
                        self.dot_size = v;
                    }
                }
                "color" => {
                    if let Some(rgb) = parse_hex_color(value) {
                        self.color_rgb = rgb;
                    }
                }
                "opacity" => {
                    if let Ok(v) = value.parse::<f64>() {
                        self.opacity = v;
                    }
                }
                "margin" => {
                    if let Ok(v) = value.parse::<f64>() {
                        self.margin_pct = v;
                    }
                }
                "sensitivity" => {
                    if let Ok(v) = value.parse::<f64>() {
                        self.sensitivity = v;
                    }
                }
                "animation" => self.animation_mode = value.to_string(),
                "ip" => self.phone_ip = value.to_string(),
                "monitor" => self.monitor_mode = value.to_string(),
                "orientation" => {
                    if let Ok(v) = value.parse::<i32>() {
                        self.orientation_deg = match v.rem_euclid(360) {
                            90 => 90,
                            180 => 180,
                            270 => 270,
                            _ => 0,
                        };
                    }
                }
                "invert_x" => self.invert_x = matches!(value, "true" | "1" | "yes"),
                "invert_y" => self.invert_y = matches!(value, "true" | "1" | "yes"),
                "simulate" => self.simulate_motion = matches!(value, "true" | "1" | "yes"),
                _ => {}
            }
        }
    }

    /// Transforms raw planar `(ax, ay)` according to user orientation & axis inversion settings.
    pub fn transform_axes(&self, ax: f32, ay: f32) -> (f32, f32) {
        let (mut rx, mut ry) = match self.orientation_deg {
            90 => (ay, -ax),
            180 => (-ax, -ay),
            270 => (-ay, ax),
            _ => (ax, ay),
        };
        if self.invert_x {
            rx = -rx;
        }
        if self.invert_y {
            ry = -ry;
        }
        (rx, ry)
    }
}

pub fn parse_hex_color(hex: &str) -> Option<(f64, f64, f64)> {
    let clean_hex = hex.trim().trim_start_matches('#');
    if clean_hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&clean_hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&clean_hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&clean_hex[4..6], 16).ok()?;
    Some((r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        let white = parse_hex_color("#ffffff").unwrap();
        assert!((white.0 - 1.0).abs() < 1e-6);
        assert!((white.1 - 1.0).abs() < 1e-6);
        assert!((white.2 - 1.0).abs() < 1e-6);

        let blue = parse_hex_color("007aff").unwrap();
        assert!((blue.0 - 0.0).abs() < 1e-6);
        assert!((blue.1 - (122.0 / 255.0)).abs() < 1e-6);
        assert!((blue.2 - 1.0).abs() < 1e-6);

        assert!(parse_hex_color("invalid").is_none());
    }

    #[test]
    fn test_apply_str_and_transform() {
        let mut cfg = MewtionConfig::default();
        cfg.apply_str(
            "size=24\ncolor=#007aff\nopacity=0.5\nmargin=0.08\nsensitivity=5.0\nanimation=Rigid\nip=192.168.43.1\nmonitor=All\norientation=90\ninvert_x=true\ninvert_y=false\nsimulate=true\n",
        );
        assert_eq!(cfg.dot_size, 24.0);
        assert_eq!(cfg.opacity, 0.5);
        assert_eq!(cfg.margin_pct, 0.08);
        assert_eq!(cfg.sensitivity, 5.0);
        assert_eq!(cfg.animation_mode, "Rigid");
        assert_eq!(cfg.phone_ip, "192.168.43.1");
        assert_eq!(cfg.monitor_mode, "All");
        assert_eq!(cfg.orientation_deg, 90);
        assert!(cfg.invert_x);
        assert!(!cfg.invert_y);
        assert!(cfg.simulate_motion);

        // 90 deg maps (ax, ay) -> (ay, -ax), then invert_x negates rx -> (-ay, -ax)
        let (tx, ty) = cfg.transform_axes(2.0, 3.0);
        assert!((tx - (-3.0)).abs() < 1e-5);
        assert!((ty - (-2.0)).abs() < 1e-5);
    }
}
