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

/// Appends the L120 checksum byte and frame terminator to a payload.
///
/// MasterCraft's `generateCheckDigit` sums every payload byte, keeps the low
/// byte and appends `0x16`. The host then prefixes HID report ID `0x10`.
pub fn l120_frame(payload: &[u8]) -> Vec<u8> {
    let checksum = payload.iter().fold(0u8, |acc, &byte| acc.wrapping_add(byte));
    let mut frame = Vec::with_capacity(payload.len() + 3);
    frame.push(0x10);
    frame.extend_from_slice(payload);
    frame.push(checksum);
    frame.push(0x16);
    frame
}

/// The 19-byte L120 telemetry report recovered from MasterCraft 1.0.6.
///
/// ```text
/// 10 | 68 01 10 0C 01 02 | power BE u16 | unit | temp BE f32 |
///      load u8 | clock BE u16 | checksum | 16
/// ```
pub fn encode_l120_packet(telemetry: &Telemetry, unit: TempUnit) -> Vec<u8> {
    let mut payload = vec![0x68, 0x01, 0x10, 0x0C, 0x01, 0x02];
    payload.extend_from_slice(&telemetry.power_w.min(999).to_be_bytes());
    payload.push(match unit {
        TempUnit::Celsius => 0x00,
        TempUnit::Fahrenheit => 0x01,
    });
    let temperature = match unit {
        TempUnit::Celsius => telemetry.temperature_c,
        TempUnit::Fahrenheit => telemetry.temperature_c * 9.0 / 5.0 + 32.0,
    };
    payload.extend_from_slice(&temperature.to_be_bytes());
    payload.push(telemetry.usage_pct);
    payload.extend_from_slice(&telemetry.frequency_mhz.to_be_bytes());
    l120_frame(&payload)
}

/// MasterCraft's `displayOpen` command: `68 01 10 02 01 01 00...`.
pub fn encode_l120_display_open() -> Vec<u8> {
    l120_frame(&[0x68, 0x01, 0x10, 0x02, 0x01, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
}

/// MasterCraft's `displayClose` command: `68 01 10 0C 01 00 00...`.
pub fn encode_l120_display_close() -> Vec<u8> {
    l120_frame(&[0x68, 0x01, 0x10, 0x0C, 0x01, 0x00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
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
    fn l120_frame_matches_mastercraft_encoding() {
        let telemetry = Telemetry::new(47.0, 16, 40, 5025).unwrap();
        let frame = encode_l120_packet(&telemetry, TempUnit::Celsius);
        // 10 | 68 01 10 0C 01 02 | 0028 | 00 | 423c0000 | 10 | 13a1 | csum | 16
        assert_eq!(frame.len(), 19);
        assert_eq!(&frame[..7], &[0x10, 0x68, 0x01, 0x10, 0x0C, 0x01, 0x02]);
        assert_eq!(frame[18], 0x16);
        assert_eq!(
            frame[17],
            frame[1..17].iter().fold(0u8, |a, &b| a.wrapping_add(b))
        );
        assert_eq!(encode_l120_display_open()[18], 0x16);
        assert_eq!(encode_l120_display_close()[18], 0x16);
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
