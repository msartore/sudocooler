//! Runtime settings with the same bounds as the daemon's CLI flags.

use super::DomainError;
use super::packet::TempUnit;

/// Lower bound accepted by `--interval-ms` after the daemon's own clamp.
pub const MIN_INTERVAL_MS: u32 = 100;
/// Upper bound chosen so a typo cannot accidentally busy-loop a cooler.
pub const MAX_INTERVAL_MS: u32 = 60_000;
pub const DEFAULT_INTERVAL_MS: u32 = 1000;

/// Configuration owned by the UI and passed to the daemon adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub interval_ms: u32,
    pub config_path: Option<String>,
    pub temp_unit: TempUnit,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            interval_ms: DEFAULT_INTERVAL_MS,
            config_path: None,
            temp_unit: TempUnit::Celsius,
        }
    }
}

impl Settings {
    /// Validates the polling interval. Keeps `0` out of the UI entirely so the
    /// daemon cannot be asked to spin at full speed.
    pub fn with_interval(mut self, interval_ms: u32) -> Result<Self, DomainError> {
        if !(MIN_INTERVAL_MS..=MAX_INTERVAL_MS).contains(&interval_ms) {
            return Err(DomainError::new(
                "interval_ms",
                "polling interval must be between 100 and 60000 ms",
            ));
        }
        self.interval_ms = interval_ms;
        Ok(self)
    }

    /// Steps the interval by `delta` ms, clamped to the allowed range.
    pub fn stepped(mut self, delta: i32) -> Self {
        let next = self.interval_ms as i64 + delta as i64;
        self.interval_ms = next.clamp(MIN_INTERVAL_MS as i64, MAX_INTERVAL_MS as i64) as u32;
        self
    }

    pub fn with_config_path(mut self, path: Option<String>) -> Self {
        self.config_path = path.filter(|p| !p.trim().is_empty());
        self
    }

    pub fn with_temp_unit(mut self, unit: TempUnit) -> Self {
        self.temp_unit = unit;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_interval() {
        assert!(Settings::default().with_interval(0).is_err());
        assert_eq!(
            Settings::default().with_interval(1000).unwrap().interval_ms,
            1000
        );
    }

    #[test]
    fn step_clamps_to_bounds() {
        assert_eq!(Settings::default().stepped(-100_000).interval_ms, MIN_INTERVAL_MS);
        assert_eq!(Settings::default().stepped(100_000).interval_ms, MAX_INTERVAL_MS);
    }
}
