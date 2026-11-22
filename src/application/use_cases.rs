//! Policy that coordinates ports and domain rules.

use crate::domain::device::Device;
use crate::domain::telemetry::Telemetry;

use super::AppError;
use super::ports::{DeviceSource, TelemetrySource};

/// Reads devices through a port and applies the UI's ordering policy:
/// writable devices first, then by VID/PID.
pub fn discover_devices(source: &impl DeviceSource) -> Vec<Device> {
    let mut devices = source.discover();
    devices.sort_by_key(|device| {
        (
            std::cmp::Reverse(device.is_writable()),
            device.vid,
            device.pid,
        )
    });
    devices
}

/// Reads one sample through a port; the port is responsible for translating
/// transport failures into `AppError`.
pub fn sample(source: &impl TelemetrySource) -> Result<Telemetry, AppError> {
    source.read()
}
