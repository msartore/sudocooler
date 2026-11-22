//! Optional L120 HID writer for the ENERGLO 360 (VID 0x381C, PID 0x0005).
//!
//! This module is the only place in the app that can write to a cooler. It is
//! inert until the user explicitly enables writes in Settings.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use hidapi::{HidApi, HidDevice};

use crate::domain::packet::{
    TempUnit, encode_l120_display_close, encode_l120_display_open, encode_l120_packet,
};
use crate::domain::telemetry::Telemetry;

const VID: u16 = 0x381C;
const PID: u16 = 0x0005;

struct Connection {
    // Keep the API object alive for as long as the device handle exists.
    _api: HidApi,
    device: HidDevice,
}

static CONNECTION: OnceLock<Mutex<Option<Connection>>> = OnceLock::new();
static OPENED: AtomicBool = AtomicBool::new(false);

fn slot() -> &'static Mutex<Option<Connection>> {
    CONNECTION.get_or_init(|| Mutex::new(None))
}

/// Opens the vendor HID collection once, not a sibling interface of the same
/// VID/PID. MasterCraft uses usage page 0xFF, usage 1.
pub fn open() -> Result<(), String> {
    let mut guard = slot().lock().map_err(|e| e.to_string())?;
    if guard.is_some() {
        return Ok(());
    }
    let api = HidApi::new().map_err(|e| format!("cannot initialise HID: {e}"))?;
    let devices: Vec<_> = api.device_list().collect();
    let info = devices
        .iter()
        .find(|device| {
            device.vendor_id() == VID
                && device.product_id() == PID
                && device.usage_page() == 0xFF
                && device.usage() == 0x01
        })
        .or_else(|| {
            devices
                .iter()
                .find(|device| device.vendor_id() == VID && device.product_id() == PID)
        })
        .ok_or_else(|| format!("no HID interface for {VID:04X}:{PID:04X}"))?;
    let device = api
        .open_path(&info.path())
        .map_err(|e| format!("cannot open ENERGLO 360 ({VID:04X}:{PID:04X}): {e}"))?;
    *guard = Some(Connection { _api: api, device });
    Ok(())
}

/// Sends one telemetry frame. `displayOpen` is sent once at init only —
/// MasterCraft does not repeat it, and repeating it resets the panel.
pub fn write_sample(sample: &Telemetry, unit: TempUnit) -> Result<usize, String> {
    open()?;
    let mut guard = slot().lock().map_err(|e| e.to_string())?;
    let Some(connection) = guard.as_mut() else {
        return Err("cooler is not connected".to_owned());
    };
    if !OPENED.swap(true, Ordering::SeqCst) {
        connection
            .device
            .write(&encode_l120_display_open())
            .map_err(|e| format!("L120 display-open write failed: {e}"))?;
    }
    let frame = encode_l120_packet(sample, unit);
    connection
        .device
        .write(&frame)
        .map_err(|e| format!("L120 telemetry write failed: {e}"))?;
    Ok(frame.len())
}

/// Sends the display-off command and releases the handle.
pub fn close() {
    OPENED.store(false, Ordering::SeqCst);
    if let Ok(mut guard) = slot().lock()
        && let Some(connection) = guard.take()
    {
        let _ = connection.device.write(&encode_l120_display_close());
    }
}
