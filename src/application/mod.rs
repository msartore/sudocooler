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
