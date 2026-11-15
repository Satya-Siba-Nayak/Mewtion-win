use crate::config::MewtionConfig;
use crate::sensor::MotionSample;
use std::fs;
use std::io::{BufRead, BufReader};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

pub fn parse_sample_line(line: &str) -> Option<MotionSample> {
    let parts: Vec<&str> = line.trim().split(',').collect();
    if parts.len() != 6 {
        return None;
    }
    let ax = parts[0].trim().parse::<f32>().ok()?;
    let ay = parts[1].trim().parse::<f32>().ok()?;
    let az = parts[2].trim().parse::<f32>().ok()?;
    let gx = parts[3].trim().parse::<f32>().ok()?;
    let gy = parts[4].trim().parse::<f32>().ok()?;
    let gz = parts[5].trim().parse::<f32>().ok()?;
    Some(MotionSample {
        ax,
        ay,
        az,
        gx,
        gy,
        gz,
    })
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
            ip.clone()
        };
        let target_address = format!("{target_ip}:8765");

        let connect_result = target_address
            .to_socket_addrs()
            .ok()
            .and_then(|mut addrs| addrs.next())
            .map(|addr: SocketAddr| TcpStream::connect_timeout(&addr, Duration::from_millis(1500)))
            .unwrap_or_else(|| TcpStream::connect(&target_address));

        match connect_result {
            Ok(stream) => {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                write_status(&format!("Connected to companion app ({target_address})"));

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
                    let latest_ip = ip_ref.lock().unwrap().clone();
                    if latest_ip != target_ip {
                        break;
                    }
                }
                write_status(&format!("Disconnected — waiting for {target_address}..."));
            }
            Err(_) => {
                write_status(&format!("Waiting for companion app at {target_address}..."));
                sleep(Duration::from_millis(1000));
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
