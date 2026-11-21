//! Composition root: the only place that names concrete adapters.
//!
//! Presentation asks for "a sample" or "the device list"; which adapter
//! answers is decided here, never in a screen.

use std::sync::OnceLock;

use nsh_core::prelude::Signal;

use crate::application::AppError;
use crate::application::ports::TelemetrySource;
use crate::domain::device::Device;
use crate::domain::telemetry::Telemetry;
use crate::infrastructure::cli::CliDaemon;
use crate::infrastructure::demo::{DemoDevices, DemoTelemetry};

/// Dependencies handed to the UI through context.
#[derive(Clone, Copy)]
pub struct AppDeps {
    /// Whether the simulated adapter is in use. A signal because the user can
    /// switch modes without restarting the app.
    pub demo: Signal<bool>,
}

impl AppDeps {
    pub fn new(demo: Signal<bool>) -> Self {
        Self { demo }
    }
}

static DEMO_TELEMETRY: OnceLock<DemoTelemetry> = OnceLock::new();

fn demo_telemetry() -> &'static DemoTelemetry {
    DEMO_TELEMETRY.get_or_init(DemoTelemetry::new)
}

/// Ordered device list, from the simulated or the real adapter.
pub fn discover(demo: bool, config_path: Option<&str>) -> Vec<Device> {
    if demo {
        crate::application::use_cases::discover_devices(&DemoDevices)
    } else {
        crate::application::use_cases::discover_devices(&CliDaemon::new(
            config_path.map(str::to_owned),
        ))
    }
}

/// One telemetry sample and the name of the backend that produced it.
pub struct Sample {
    pub telemetry: Telemetry,
    pub backend: &'static str,
}

pub fn sample(demo: bool, config_path: Option<&str>) -> Result<Sample, AppError> {
    if demo {
        let source = demo_telemetry();
        let telemetry = crate::application::use_cases::sample(source)?;
        Ok(Sample {
            telemetry,
            backend: source.backend_name(),
        })
    } else {
        let source = CliDaemon::new(config_path.map(str::to_owned));
        let telemetry = crate::application::use_cases::sample(&source)?;
        Ok(Sample {
            telemetry,
            backend: source.backend_name(),
        })
    }
}
