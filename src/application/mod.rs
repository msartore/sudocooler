//! Application layer: ports and use cases. No `nsh`, no process, no platform.

pub mod ports;
pub mod use_cases;

/// Error kinds a presentation layer can branch on without sniffing messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The daemon binary or its registry could not be read.
    Unavailable,
    /// A telemetry sample failed validation or parsing.
    InvalidSample,
    /// A domain rule rejected a value.
    InvalidSetting,
}

/// What this process can do with PawnIO right now, so the UI can tell
/// "not installed" (offer to set it up) from "installed but not elevated"
/// (ask to restart as administrator). Modelled on OpenColorHub's status probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PawnIoStatus {
    /// The driver is installed and this process may use it.
    Ready { version: String },
    /// `PawnIOLib.dll` was not found: PawnIO is not installed.
    NotInstalled,
    /// Present, but the driver refuses this process: it is not elevated.
    Denied,
    /// Any other failure, with a human-readable detail.
    Failed(String),
}

impl PawnIoStatus {
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready { .. })
    }
}

/// A translated application error with a message suitable for humans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppError {
    pub kind: ErrorKind,
    pub message: String,
}

impl AppError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl From<crate::domain::DomainError> for AppError {
    fn from(error: crate::domain::DomainError) -> Self {
        Self::new(ErrorKind::InvalidSetting, error.to_string())
    }
}
