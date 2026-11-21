//! CPU telemetry value object, mirroring the daemon's `Telemetry` contract.

use super::DomainError;

/// One CPU telemetry sample sent to a Sudokoo cooler display.
///
/// The bounds match `sudokooler`'s SK700V encoder: temperature in `C`, usage
/// in `0..=100`, power in `0..=999` W and frequency in MHz.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Telemetry {
    pub temperature_c: f32,
    pub usage_pct: u8,
    pub power_w: u16,
    pub frequency_mhz: u16,
}

impl Telemetry {
    /// Constructs a validated sample. Invalid readings are rejected at the
    /// boundary instead of producing nonsense HID packets.
    pub fn new(
        temperature_c: f32,
        usage_pct: u8,
        power_w: u16,
        frequency_mhz: u16,
    ) -> Result<Self, DomainError> {
        if !temperature_c.is_finite() || !(-40.0..=125.0).contains(&temperature_c) {
            return Err(DomainError::new(
                "temperature_c",
                "temperature must be a finite value between -40 and 125 C",
            ));
        }
        if usage_pct > 100 {
            return Err(DomainError::new(
                "usage_pct",
                "CPU usage must be between 0 and 100 percent",
            ));
        }
        if power_w > 999 {
            return Err(DomainError::new(
                "power_w",
                "package power must be between 0 and 999 watts",
            ));
        }
        if frequency_mhz > 10_000 {
            return Err(DomainError::new(
                "frequency_mhz",
                "frequency must be between 0 and 10000 MHz",
            ));
        }
        Ok(Self {
            temperature_c,
            usage_pct,
            power_w,
            frequency_mhz,
        })
    }

    /// Percentage of the cooler's power bar that should light up.
    ///
    /// Integer floor, exactly as the daemon's protocol encoder does it.
    pub fn power_bar_pct(&self, max_watts: u16) -> u8 {
        let max = u32::from(max_watts.max(1));
        ((u32::from(self.power_w) * 100) / max).min(100) as u8
    }

    /// Whether the reading is in the thermal throttling range.
    pub fn is_hot(&self) -> bool {
        self.temperature_c >= 90.0
    }

    /// Display text for the cooler's four-digit panel, e.g. `"47.0"`.
    pub fn display_text(&self) -> String {
        let temp = self.temperature_c.clamp(0.0, 99.9);
        format!("{temp:04.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_realistic_sample() {
        let t = Telemetry::new(47.0, 16, 40, 5025).unwrap();
        assert_eq!(t.display_text(), "47.0");
        assert!(!t.is_hot());
    }

    #[test]
    fn power_bar_uses_integer_floor_and_clamps() {
        let t = Telemetry::new(60.0, 50, 40, 4000).unwrap();
        assert_eq!(t.power_bar_pct(156), 25);
        assert_eq!(t.power_bar_pct(160), 25);
        let hot = Telemetry::new(60.0, 100, 999, 5000).unwrap();
        assert_eq!(hot.power_bar_pct(160), 100);
    }

    #[test]
    fn rejects_out_of_range_readings() {
        assert!(Telemetry::new(f32::NAN, 0, 0, 0).is_err());
        assert!(Telemetry::new(20.0, 101, 0, 0).is_err());
        assert!(Telemetry::new(20.0, 0, 1000, 0).is_err());
    }
}
