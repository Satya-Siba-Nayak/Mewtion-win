//! Built-in motion sensors on Windows via the WinRT `Windows.Devices.Sensors` API.
//!
//! Windows laptops, 2-in-1 convertibles, and tablets with an integrated IMU or
//! HID Sensor Hub expose their sensors through `Accelerometer::GetDefault()`
//! and `Gyrometer::GetDefault()`.
//!
//! Just like the Linux IIO backend, if the machine has no built-in
//! accelerometer, `WinSensorSource::discover()` returns `None` and Mewtion
//! automatically falls back to the Android companion app over TCP.

use crate::sensor::MotionSample;
use std::thread::sleep;
use std::time::{Duration, Instant};

#[cfg(windows)]
use windows::Devices::Sensors::{Accelerometer, AccelerometerReadingType, Gyrometer};

/// Standard gravity in m/s² used to convert WinRT G-force readings to m/s².
const G_TO_MPS2: f32 = 9.80665;

/// Sampling faster than 100 Hz gains nothing: the overlay redraws at ~60 FPS
/// and vehicle motion has no content above these frequencies.
const MAX_POLL_HZ: f32 = 100.0;

/// Default loop frequency if the sensor does not report an interval.
const DEFAULT_POLL_HZ: f32 = 60.0;

/// Cutoff of the gravity estimator (0.25 Hz). Low enough to pass sustained
/// device tilt/orientation while rejecting vehicle linear acceleration.
const GRAVITY_CUTOFF_HZ: f32 = 0.25;

#[cfg(windows)]
pub struct WinSensorSource {
    accel: Accelerometer,
    gyro: Option<Gyrometer>,
    hardware_linear: bool,
    poll_hz: f32,
}

#[cfg(not(windows))]
pub struct WinSensorSource;

impl WinSensorSource {
    /// Probes the Windows Sensor API for a built-in accelerometer and optional
    /// gyrometer. Returns `None` on desktop PCs or laptops without an IMU.
    #[cfg(windows)]
    pub fn discover() -> Option<WinSensorSource> {
        let (accel, hardware_linear) =
            match Accelerometer::GetDefaultWithAccelerometerReadingType(
                AccelerometerReadingType::Linear,
            ) {
                Ok(linear_accel) => (linear_accel, true),
                Err(_) => (Accelerometer::GetDefault().ok()?, false),
            };

        // Request ~100 Hz (10 ms report interval) bounded by the hardware's minimum.
        let min_interval_ms = accel.MinimumReportInterval().unwrap_or(16).max(10);
        let _ = accel.SetReportInterval(min_interval_ms);
        let actual_ms = accel.ReportInterval().unwrap_or(min_interval_ms).max(1);
        let poll_hz = (1000.0 / actual_ms as f32).clamp(10.0, MAX_POLL_HZ);

        let gyro = Gyrometer::GetDefault().ok();
        if let Some(ref g) = gyro {
            let g_min = g.MinimumReportInterval().unwrap_or(16).max(10);
            let _ = g.SetReportInterval(g_min);
        }

        Some(WinSensorSource {
            accel,
            gyro,
            hardware_linear,
            poll_hz,
        })
    }

    #[cfg(not(windows))]
    pub fn discover() -> Option<WinSensorSource> {
        None
    }

    #[cfg(windows)]
    pub fn describe(&self) -> String {
        let gyro = match self.gyro {
            Some(_) => "gyrometer",
            None => "no gyrometer",
        };
        let mode = if self.hardware_linear {
            "hardware linear acceleration"
        } else {
            "low-pass gravity filter"
        };
        format!(
            "WinRT accelerometer at {:.0} Hz, {gyro}, {mode}",
            self.poll_hz
        )
    }

    #[cfg(not(windows))]
    pub fn describe(&self) -> String {
        "unsupported platform".to_string()
    }
}

/// Streams motion samples until the process exits, calling `on_sample` for each reading.
#[cfg(windows)]
pub fn run_win_sensor_blocking<F>(source: WinSensorSource, mut on_sample: F)
where
    F: FnMut(MotionSample),
{
    let period = Duration::from_secs_f32(1.0 / source.poll_hz.max(1.0));
    let rc = 1.0 / (2.0 * std::f32::consts::PI * GRAVITY_CUTOFF_HZ);

    let mut estimated_gravity: Option<[f32; 3]> = None;
    let mut last_sample_at: Option<Instant> = None;

    loop {
        let started = Instant::now();

        let Ok(reading) = source.accel.GetCurrentReading() else {
            sleep(period);
            continue;
        };

        let ax_raw = reading.AccelerationX().unwrap_or(0.0) as f32 * G_TO_MPS2;
        let ay_raw = reading.AccelerationY().unwrap_or(0.0) as f32 * G_TO_MPS2;
        let az_raw = reading.AccelerationZ().unwrap_or(0.0) as f32 * G_TO_MPS2;
        let accel = [ax_raw, ay_raw, az_raw];

        let g_est = if source.hardware_linear {
            [0.0, 0.0, 0.0]
        } else {
            let dt = last_sample_at
                .map(|at| at.elapsed().as_secs_f32())
                .unwrap_or(period.as_secs_f32())
                .clamp(1e-3, 1.0);
            let alpha = dt / (rc + dt);

            // Seed gravity from the first reading to prevent an initial lurch while converging.
            let gravity = estimated_gravity.get_or_insert(accel);
            for (axis, sampled) in gravity.iter_mut().zip(accel) {
                *axis += alpha * (sampled - *axis);
            }
            *gravity
        };

        let gz = source
            .gyro
            .as_ref()
            .and_then(|g| g.GetCurrentReading().ok())
            .and_then(|r| r.AngularVelocityZ().ok())
            .map(|deg_per_sec| (deg_per_sec as f32).to_radians())
            .unwrap_or(0.0);

        last_sample_at = Some(Instant::now());

        on_sample(MotionSample {
            ax: accel[0] - g_est[0],
            ay: accel[1] - g_est[1],
            az: accel[2] - g_est[2],
            gx: 0.0,
            gy: 0.0,
            gz,
        });

        if let Some(remaining) = period.checked_sub(started.elapsed()) {
            sleep(remaining);
        }
    }
}

#[cfg(not(windows))]
pub fn run_win_sensor_blocking<F>(_source: WinSensorSource, _on_sample: F)
where
    F: FnMut(MotionSample),
{
    let _ = DEFAULT_POLL_HZ;
    let _ = G_TO_MPS2;
    let _ = MAX_POLL_HZ;
    let _ = GRAVITY_CUTOFF_HZ;
}
