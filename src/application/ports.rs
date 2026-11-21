//! Interfaces the UI needs, expressed purely in domain types.

use crate::domain::device::Device;
use crate::domain::telemetry::Telemetry;

use super::AppError;

/// Discovers Cooler HID interfaces and profiles.
pub trait DeviceSource {
    fn discover(&self) -> Vec<Device>;
}

/// Reads one CPU telemetry sample and names the backend it came from.
pub trait TelemetrySource {
    fn read(&self) -> Result<Telemetry, AppError>;
    fn backend_name(&self) -> &'static str;
}
