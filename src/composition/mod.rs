//! Composition root: the only place that names concrete adapters.
//!
//! Presentation asks for "a sample" or "the device list"; which adapter
//! answers is decided here, never in a screen. The real daemon and OS HID
//! tree are always used — there is no demo switch.

use crate::application::ports::TelemetrySource;
use crate::application::AppError;
use crate::domain::device::Device;
use crate::domain::telemetry::Telemetry;
use crate::infrastructure::cli::CliDaemon;
use crate::infrastructure::system::SystemDevices;

/// Dependencies handed to the UI through context.
#[derive(Clone, Copy)]
pub struct AppDeps;

impl AppDeps {
    pub fn new() -> Self {
        Self
    }
}

/// Ordered device list, read from the operating system's HID tree.
pub fn discover() -> Vec<Device> {
    crate::application::use_cases::discover_devices(&SystemDevices)
}

/// One telemetry sample and the name of the backend that produced it.
pub struct Sample {
    pub telemetry: Telemetry,
    pub backend: &'static str,
}

/// Writes one L120 frame to the ENERGLO 360. Called every polling tick.
pub fn write_l120(
    sample: &Telemetry,
    unit: crate::domain::packet::TempUnit,
) -> Result<usize, String> {
    crate::infrastructure::hid_writer::write_sample(sample, unit)
}

pub fn close_l120() {
    crate::infrastructure::hid_writer::close();
}

/// Windows logon autostart controls.
pub fn autostart_enabled() -> bool {
    crate::infrastructure::autostart::is_enabled()
}

pub fn set_autostart(enabled: bool) -> Result<(), String> {
    if enabled {
        crate::infrastructure::autostart::enable()
    } else {
        crate::infrastructure::autostart::disable()
    }
}

/// Live telemetry, always from the `sudokooler` daemon.
pub fn sample(config_path: Option<&str>) -> Result<Sample, AppError> {
    let source = CliDaemon::new(config_path.map(str::to_owned));
    let telemetry = crate::application::use_cases::sample(&source)?;
    Ok(Sample {
        telemetry,
        backend: source.backend_name(),
    })
}
