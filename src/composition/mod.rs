//! Composition root: the only place that names concrete adapters.
//!
//! Presentation asks for "a sample" or "the device list"; which adapter
//! answers is decided here, never in a screen. The real daemon and OS HID
//! tree are always used — there is no demo switch.

use crate::application::AppError;
use crate::application::ports::TelemetrySource;
use crate::domain::device::Device;
use crate::domain::telemetry::Telemetry;
use crate::infrastructure::cli::CliDaemon;
use crate::infrastructure::system::SystemDevices;

/// Dependencies handed to the UI through context.
#[derive(Clone, Copy)]
pub struct AppDeps;

impl AppDeps {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AppDeps {
    fn default() -> Self {
        Self::new()
    }
}

/// Ordered device list, read from the operating system's HID tree.
pub fn discover() -> Vec<Device> {
    crate::application::use_cases::discover_devices(&SystemDevices)
}

/// One telemetry sample and the name of the backend that produced it.
pub struct Sample {
    pub telemetry: Telemetry,
    pub backend: &'static str,
    /// The daemon's own warning about unavailable sensors, if any.
    pub notice: Option<String>,
}

/// Writes one L120 frame to the ENERGLO 360. Called every polling tick.
pub fn write_l120(
    sample: &Telemetry,
    unit: crate::domain::packet::TempUnit,
) -> Result<usize, String> {
    crate::infrastructure::hid_writer::write_sample(sample, unit)
}

pub fn close_l120() {
    crate::infrastructure::hid_writer::close();
}

/// Stops the long-lived sampling daemon (pause, settings change, quit).
pub fn stop_sampling() {
    crate::infrastructure::cli::shutdown();
}

/// What this process can do with PawnIO (temperature/power need it on Windows).
pub fn pawnio_status() -> crate::application::PawnIoStatus {
    crate::infrastructure::pawnio::status()
}

/// Whether this process is elevated, and whether it could ask.
pub fn elevation() -> crate::infrastructure::elevation::Elevation {
    crate::infrastructure::elevation::state()
}

/// Asks Windows for administrator rights and relaunches elevated. `true` means
/// the elevated copy took over and this process should exit; `false` means the
/// user declined the prompt.
pub fn request_elevation() -> Result<bool, String> {
    use crate::infrastructure::elevation::Outcome;
    match crate::infrastructure::elevation::request()? {
        Outcome::Launched => Ok(true),
        Outcome::Declined => Ok(false),
        Outcome::Unsupported => Err("elevation is not available on this platform".to_owned()),
    }
}

/// Runs the bundled PawnIO installer, then reports the new status.
pub fn install_pawnio() -> Result<String, String> {
    let path = crate::infrastructure::pawnio::installer().ok_or_else(|| {
        "the PawnIO installer is not bundled; run the sudocooler setup again".to_owned()
    })?;
    crate::infrastructure::pawnio::run_installer(&path)?;
    match pawnio_status() {
        crate::application::PawnIoStatus::Ready { .. } => {
            Ok("PawnIO is installed; restart sudocooler as administrator".to_owned())
        }
        _ => Ok("PawnIO setup finished; restart sudocooler as administrator".to_owned()),
    }
}

/// Self-update from GitHub Releases.
pub fn check_update() -> Result<String, String> {
    crate::infrastructure::updates::check()
}

pub fn install_update() -> Result<String, String> {
    crate::infrastructure::updates::install()
}

/// Windows logon autostart controls.
pub fn autostart_enabled() -> bool {
    crate::infrastructure::autostart::is_enabled()
}

/// Retires a logon entry left by a build that started elevated (see the
/// adapter); a no-op unless this process runs elevated.
pub fn migrate_autostart() {
    crate::infrastructure::autostart::migrate();
}

pub fn set_autostart(enabled: bool) -> Result<(), String> {
    if enabled {
        crate::infrastructure::autostart::enable()
    } else {
        crate::infrastructure::autostart::disable()
    }
}

/// Live telemetry, always from the `sudokooler` daemon. One process feeds
/// every sample; `interval_ms` is the cadence it is started with.
pub fn sample(config_path: Option<&str>, interval_ms: u32) -> Result<Sample, AppError> {
    let source = CliDaemon::new(config_path.map(str::to_owned), interval_ms);
    let telemetry = crate::application::use_cases::sample(&source)?;
    Ok(Sample {
        telemetry,
        backend: source.backend_name(),
        notice: source.notice(),
    })
}
