//! Pure SK700V packet encoding, matching the daemon's wire contract byte for byte.
//!
//! This lives in the domain so the UI can render a trustworthy preview of the
//! frame the daemon will send, without opening USB or spawning a process.

use super::telemetry::Telemetry;

/// Temperature unit flag carried in byte 10 of a telemetry report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempUnit {
    Celsius,
    Fahrenheit,
}

impl TempUnit {
    pub fn label(self) -> &'static str {
        match self {
            Self::Celsius => "C",
            Self::Fahrenheit => "F",
        }
    }
}

/// 8-bit wrapping checksum over bytes `1..18` (the report ID is excluded).
pub fn checksum(frame: &[u8]) -> u8 {
    frame
        .get(1..18)
        .unwrap_or_default()
        .iter()
        .fold(0u8, |acc, &byte| acc.wrapping_add(byte))
}

/// Encodes one telemetry sample into a zero-padded 64-byte SK700V HID report.
pub fn encode_packet(telemetry: &Telemetry, max_watts: u16, unit: TempUnit) -> [u8; 64] {
    let mut packet = [0u8; 64];
    packet[0] = 0x10; // HID report ID
    packet[1] = 0x68;
    packet[2] = 0x01;
    packet[3] = 0x09;
    packet[4] = 0x0D; // 13 payload bytes
    packet[5] = 0x01; // telemetry command
    packet[6] = 0x02; // display active

    let power = telemetry.power_w.min(999);
    packet[7..9].copy_from_slice(&power.to_be_bytes());
    packet[9] = telemetry.power_bar_pct(max_watts);
    packet[10] = match unit {
        TempUnit::Celsius => 0x00,
        TempUnit::Fahrenheit => 0x01,
    };

    let temperature = match unit {
        TempUnit::Celsius => telemetry.temperature_c,
        TempUnit::Fahrenheit => telemetry.temperature_c * 9.0 / 5.0 + 32.0,
    };
    packet[11..15].copy_from_slice(&temperature.to_be_bytes());
    packet[15] = telemetry.usage_pct;
    packet[16..18].copy_from_slice(&telemetry.frequency_mhz.to_be_bytes());
    packet[18] = checksum(&packet);
    packet[19] = 0x16; // frame terminator
    packet
}

/// Lowercase contiguous hex, as printed by `sudokooler --dry-run`.
pub fn format_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_captured_sk700v_frame_byte_for_byte() {
        // Captured on a Ryzen 9800X3D and checked into the daemon as a parity test.
        let telemetry = Telemetry::new(47.0, 16, 40, 5025).unwrap();
        let frame = encode_packet(&telemetry, 156, TempUnit::Celsius);
        let captured = "106801090d010200281900423c00001013a10516";
        let mut expected = captured.to_owned();
        expected.push_str(&"00".repeat(64 - captured.len() / 2));
        assert_eq!(format_hex(&frame), expected);
    }

    #[test]
    fn fahrenheit_flag_and_conversion() {
        let telemetry = Telemetry::new(25.0, 5, 0, 3600).unwrap();
        let frame = encode_packet(&telemetry, 160, TempUnit::Fahrenheit);
        assert_eq!(frame[10], 0x01);
        assert_eq!(&frame[11..15], &77.0f32.to_be_bytes());
        assert_eq!(frame[18], checksum(&frame));
        assert_eq!(frame[19], 0x16);
    }
}
