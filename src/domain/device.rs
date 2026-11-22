//! USB device identity, profiles and the safety rules around writing to them.

/// Sudokoo's USB vendor ID. Profiles with any other VID are never writable.
pub const SUDOKOO_VID: u16 = 0x381C;

/// Wire protocol family of a cooler profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// The verified 64-byte SK700V HID protocol.
    Sk700v,
    /// The L120 protocol used by the ENERGLO 360 (recovered from MasterCraft).
    L120,
}

impl Protocol {
    pub fn label(self) -> &'static str {
        match self {
            Self::Sk700v => "sk700v",
            Self::L120 => "l120",
        }
    }
}

/// A USB HID interface discovered on the machine, or a documented profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub name: String,
    pub vid: u16,
    pub pid: u16,
    pub protocol: Protocol,
    pub enabled: bool,
    pub verified: bool,
    pub connected: bool,
    pub path: Option<String>,
}

impl Device {
    /// The `VID:PID` label printed in the UI and by `--list-devices`.
    pub fn id(&self) -> String {
        format!("{:04X}:{:04X}", self.vid, self.pid)
    }

    /// Only an enabled, allowlisted profile may ever be written to.
    /// Unverified PIDs stay disabled in the daemon's default registry.
    pub fn is_writable(&self) -> bool {
        self.vid == SUDOKOO_VID && self.enabled && self.connected
    }

    /// A short status word for the device list.
    pub fn status_label(&self) -> &'static str {
        match (self.connected, self.enabled, self.verified) {
            (true, true, true) => "Active",
            (true, true, false) => "Unverified",
            (true, false, false) => "Detected · read-only",
            (true, false, true) => "Disabled",
            (false, _, _) => "Not detected",
        }
    }
}

/// Human-readable model name for a known Sudokoo product ID.
pub fn model_name_for_pid(pid: u16) -> &'static str {
    match pid {
        0x0003 => "SK700V",
        0x0005 => "ENERGLO 360 (L120)",
        _ => "unverified model",
    }
}

/// The daemon registry's default profiles, verbatim from
/// `sudokooler/profiles/default_devices.toml`.
pub fn default_profiles() -> Vec<Device> {
    let mut profiles = Vec::new();
    for (pid, verified) in [
        (0x0003u16, true),
        (0x0001, false),
        (0x0002, false),
        (0x0004, false),
        (0x0005, false),
    ] {
        // PID 0x0005 was identified as the ENERGLO 360 by the device owner.
        // It is still disabled: the daemon has no verified packet schema for
        // MasterCraft's L120 platform, so this is identification, not support.
        let name = if verified || pid == 0x0005 {
            format!("Sudokoo {}", model_name_for_pid(pid))
        } else {
            format!("Sudokoo Unverified (PID {pid:#06x})")
        };
        profiles.push(Device {
            name,
            vid: SUDOKOO_VID,
            pid,
            protocol: if pid == 0x0005 {
                Protocol::L120
            } else {
                Protocol::Sk700v
            },
            enabled: verified,
            verified,
            connected: false,
            path: None,
        });
    }
    profiles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_has_one_enabled_verified_profile() {
        let profiles = default_profiles();
        assert_eq!(profiles.len(), 5);
        assert_eq!(profiles.iter().filter(|d| d.enabled).count(), 1);
        assert_eq!(profiles[0].id(), "381C:0003");
        assert!(profiles[0].verified);
        assert_eq!(profiles[4].name, "Sudokoo ENERGLO 360 (L120)");
        assert!(!profiles[4].enabled);
    }

    #[test]
    fn unverified_or_absent_devices_are_never_writable() {
        let mut profiles = default_profiles();
        assert!(!profiles[1].is_writable()); // disabled
        profiles[0].connected = true;
        assert!(profiles[0].is_writable());
        profiles[0].vid = 0x3633;
        assert!(!profiles[0].is_writable()); // wrong vendor
    }
}
