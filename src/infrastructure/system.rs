//! Real HID enumeration for Sudokoo coolers.
//!
//! This adapter reads the operating system's device list directly, so the app
//! detects the cooler that is actually plugged in even when the daemon binary
//! has not been built. Detection never opens or writes to the device.

#[cfg(windows)]
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

use crate::application::ports::DeviceSource;
use crate::domain::device::{Device, Protocol, SUDOKOO_VID, default_profiles, model_name_for_pid};

/// Enumerates HID interfaces with the Sudokoo vendor ID.
pub struct SystemDevices;

#[cfg(windows)]
fn detected_pids() -> Vec<u16> {
    const SCRIPT: &str = "Get-PnpDevice | Where-Object { $_.InstanceId -match 'VID_381C' } | Select-Object -ExpandProperty InstanceId";
    let mut command = Command::new("powershell.exe");
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let Ok(output) = command
        .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT])
        .output()
    else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut pids = Vec::new();
    for line in text.lines() {
        if let Some(pos) = line.find("PID_") {
            let hex: String = line[pos + 4..]
                .chars()
                .take_while(|c| c.is_ascii_hexdigit())
                .take(4)
                .collect();
            if let Ok(pid) = u16::from_str_radix(&hex, 16)
                && !pids.contains(&pid)
            {
                pids.push(pid);
            }
        }
    }
    pids
}

#[cfg(target_os = "linux")]
fn detected_pids() -> Vec<u16> {
    let mut pids = Vec::new();
    let Ok(entries) = std::fs::read_dir("/sys/class/hidraw") else {
        return pids;
    };
    for entry in entries.flatten() {
        let Ok(uevent) = std::fs::read_to_string(entry.path().join("device/uevent")) else {
            continue;
        };
        for line in uevent.lines() {
            let Some(value) = line.strip_prefix("HID_ID=") else {
                continue;
            };
            let parts: Vec<&str> = value.split(':').collect();
            if parts.len() == 3
                && parts[1].eq_ignore_ascii_case("0000381C")
                && let Ok(pid) =
                    u16::from_str_radix(&parts[2][parts[2].len().saturating_sub(4)..], 16)
                && !pids.contains(&pid)
            {
                pids.push(pid);
            }
        }
    }
    pids
}

#[cfg(not(any(windows, target_os = "linux")))]
fn detected_pids() -> Vec<u16> {
    Vec::new()
}

impl DeviceSource for SystemDevices {
    fn discover(&self) -> Vec<Device> {
        let pids = detected_pids();
        let mut devices = default_profiles();
        for device in &mut devices {
            device.connected = pids.contains(&device.pid);
        }
        for pid in pids {
            if !devices.iter().any(|device| device.pid == pid) {
                devices.push(Device {
                    name: format!("Sudokoo device ({})", model_name_for_pid(pid)),
                    vid: SUDOKOO_VID,
                    pid,
                    protocol: Protocol::Sk700v,
                    enabled: false,
                    verified: false,
                    connected: true,
                    path: None,
                });
            }
        }
        devices
    }
}
