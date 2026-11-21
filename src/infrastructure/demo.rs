//! Deterministic simulated data so the UI is useful without attached hardware.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::application::ports::{DeviceSource, TelemetrySource};
use crate::application::AppError;
use crate::domain::device::{Device, default_profiles};
use crate::domain::telemetry::Telemetry;

/// Reports the daemon's default registry with the SK700V "connected".
pub struct DemoDevices;

impl DeviceSource for DemoDevices {
    fn discover(&self) -> Vec<Device> {
        let mut devices = default_profiles();
        if let Some(primary) = devices.first_mut() {
            primary.connected = true;
            primary.path = Some(r"\\?\hid#vid_381c&pid_0003#...".to_owned());
        }
        devices
    }
}

/// A smooth, repeatable telemetry waveform driven by its own sample counter.
pub struct DemoTelemetry {
    tick: AtomicU64,
}

impl DemoTelemetry {
    pub fn new() -> Self {
        Self {
            tick: AtomicU64::new(0),
        }
    }
}

impl Default for DemoTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetrySource for DemoTelemetry {
    fn read(&self) -> Result<Telemetry, AppError> {
        let tick = self.tick.fetch_add(1, Ordering::Relaxed);
        let phase = tick as f64 * 0.16;
        let temperature = 47.0 + 8.0 * phase.sin() + 1.6 * (phase * 2.7).sin();
        let usage = (26.0 + 28.0 * (phase * 0.7).sin()).clamp(0.0, 100.0) as u8;
        let power = (42.0 + 26.0 * (phase * 1.1).sin()).clamp(0.0, 999.0) as u16;
        let frequency = (4850.0 + 260.0 * (phase * 0.9).sin()).clamp(0.0, 10_000.0) as u16;

        Telemetry::new(temperature as f32, usage, power, frequency).map_err(Into::into)
    }

    fn backend_name(&self) -> &'static str {
        "demo"
    }
}
