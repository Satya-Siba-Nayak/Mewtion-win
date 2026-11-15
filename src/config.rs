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
                // Check parent directory if running from `bin/Mewtion.exe`
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

    pub fn load() -> Self {
        let path = Self::config_path();
        if !Path::new(&path).exists() {
            return Self::default();
        }

        let mut config = Self::default();

        if let Ok(contents) = fs::read_to_string(&path) {
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
                            config.dot_size = v;
                        }
                    }
                    "color" => {
                        if let Some(rgb) = parse_hex_color(value) {
                            config.color_rgb = rgb;
                        }
                    }
                    "opacity" => {
                        if let Ok(v) = value.parse::<f64>() {
                            config.opacity = v;
                        }
                    }
                    "margin" => {
                        if let Ok(v) = value.parse::<f64>() {
                            config.margin_pct = v;
                        }
                    }
                    "sensitivity" => {
                        if let Ok(v) = value.parse::<f64>() {
                            config.sensitivity = v;
                        }
                    }
                    "animation" => config.animation_mode = value.to_string(),
                    "ip" => config.phone_ip = value.to_string(),
                    _ => {}
                }
            }
        }
        config
    }
}

fn parse_hex_color(hex: &str) -> Option<(f64, f64, f64)> {
    let clean_hex = hex.trim().trim_start_matches('#');
    if clean_hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&clean_hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&clean_hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&clean_hex[4..6], 16).ok()?;
    Some((r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0))
}
