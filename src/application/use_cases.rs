//! Policy that coordinates ports and domain rules.

use crate::domain::device::Device;
use crate::domain::packet::{encode_packet, format_hex};
use crate::domain::settings::Settings;
use crate::domain::telemetry::Telemetry;

use super::AppError;
use super::ports::{DeviceSource, TelemetrySource};

/// One rendered frame in the packet log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketEntry {
    pub sequence: u32,
    pub hex: String,
    pub temperature_c: String,
}

impl PacketEntry {
    /// Encodes a preview frame from a sample and the active profile settings.
    ///
    /// `max_watts` comes from the profile (`power_bar_max_w`); 160 is the
    /// daemon's default for the SK700V.
    pub fn from_sample(
        sequence: u32,
        sample: &Telemetry,
        settings: &Settings,
        max_watts: u16,
    ) -> Self {
        let packet = encode_packet(sample, max_watts, settings.temp_unit);
        Self {
            sequence,
            hex: format_hex(&packet),
            temperature_c: format!("{:.1}", sample.temperature_c),
        }
    }
}

/// A small bounded log: the dashboard only needs the latest frames.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PacketLog {
    entries: Vec<PacketEntry>,
    next_sequence: u32,
}

impl PacketLog {
    pub const CAPACITY: usize = 60;

    pub fn push(&mut self, entry: PacketEntry) {
        self.next_sequence = self.next_sequence.wrapping_add(1);
        self.entries.insert(0, entry);
        self.entries.truncate(Self::CAPACITY);
    }

    pub fn entries(&self) -> &[PacketEntry] {
        &self.entries
    }

    pub fn latest(&self) -> Option<&PacketEntry> {
        self.entries.first()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

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
