use crate::config::{sanitize_ip, MewtionConfig};
use crate::sensor::MotionSample;
use std::fs;
use std::io::{BufRead, BufReader};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

pub fn parse_sample_line(line: &str) -> Option<MotionSample> {
    let parts: Vec<&str> = line.trim().split(',').collect();
    if parts.len() == 6 {
        let parse_f32 = |s: &str| -> Option<f32> {
            let cleaned = s.trim();
            cleaned
                .parse::<f32>()
                .ok()
                .or_else(|| cleaned.replace(',', ".").parse::<f32>().ok())
        };
        let ax = parse_f32(parts[0])?;
        let ay = parse_f32(parts[1])?;
        let az = parse_f32(parts[2])?;
        let gx = parse_f32(parts[3])?;
        let gy = parse_f32(parts[4])?;
        let gz = parse_f32(parts[5])?;
        Some(MotionSample {
            ax,
            ay,
            az,
            gx,
            gy,
            gz,
        })
    } else if parts.len() == 12 {
        // Mewtion-Android uses String.format("%.3f,...") without Locale.US, which formats
        // decimals with commas on European/non-US phone locales (12 comma-separated tokens).
        let parse_pair = |int_part: &str, frac_part: &str| -> Option<f32> {
            let combined = format!("{}.{}", int_part.trim(), frac_part.trim());
            combined.parse::<f32>().ok()
        };
        let ax = parse_pair(parts[0], parts[1])?;
        let ay = parse_pair(parts[2], parts[3])?;
        let az = parse_pair(parts[4], parts[5])?;
        let gx = parse_pair(parts[6], parts[7])?;
        let gy = parse_pair(parts[8], parts[9])?;
        let gz = parse_pair(parts[10], parts[11])?;
        Some(MotionSample {
            ax,
            ay,
            az,
            gx,
            gy,
            gz,
        })
    } else {
        None
    }
}

fn write_status(status: &str) {
    let _ = fs::write(MewtionConfig::status_path(), status);
}

pub fn run_tcp_bridge_blocking<F>(ip_ref: Arc<Mutex<String>>, mut on_sample: F)
where
    F: FnMut(MotionSample),
{
    loop {
        let target_ip = {
            let ip = ip_ref.lock().unwrap();
            sanitize_ip(&ip)
        };
        let target_address = format!("{target_ip}:8765");

        let connect_result = target_address
            .to_socket_addrs()
            .ok()
            .and_then(|mut addrs| addrs.next())
            .map(|addr: SocketAddr| TcpStream::connect_timeout(&addr, Duration::from_millis(1200)))
            .unwrap_or_else(|| TcpStream::connect(&target_address));

        match connect_result {
            Ok(stream) => {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(4)));
                let _ = stream.set_nodelay(true);
                write_status(&format!("Connected to Phone ({target_address})"));

                let mut reader = BufReader::new(stream);
                let mut line = String::new();

                while let Ok(bytes_read) = reader.read_line(&mut line) {
                    if bytes_read == 0 {
                        break; // Connection closed
                    }
                    if let Some(sample) = parse_sample_line(&line) {
                        on_sample(sample);
                    }
                    line.clear();

                    // Reconnect immediately if the user changed the target IP in Settings
                    let latest_ip = sanitize_ip(&ip_ref.lock().unwrap());
                    if latest_ip != target_ip {
                        break;
                    }
                }
                write_status(&format!("Disconnected — waiting for {target_address}..."));
            }
            Err(_) => {
                write_status(&format!("Waiting for Phone at {target_address}..."));
                sleep(Duration::from_millis(800));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sample_line() {
        let sample = parse_sample_line("1.25,-0.5,0.0,0.01,-0.02,0.35\n").unwrap();
        assert!((sample.ax - 1.25).abs() < 1e-5);
        assert!((sample.ay - (-0.5)).abs() < 1e-5);
        assert!((sample.gz - 0.35).abs() < 1e-5);

        assert!(parse_sample_line("invalid,line").is_none());
        assert!(parse_sample_line("1,2,3,4,5,not_a_float").is_none());
    }
}
