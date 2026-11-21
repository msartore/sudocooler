//! Pure domain layer: product rules and value objects.
//!
//! Nothing in this module imports `nsh`, talks to a process or touches the
//! clock. That keeps the rules testable with `cargo test` alone and makes the
//! dependency rule physical rather than conventional.

pub mod device;
pub mod packet;
pub mod settings;
pub mod telemetry;

/// A validation failure in a domain value object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainError {
    /// The field that failed validation.
    pub field: &'static str,
    /// A human-readable explanation of the rule that was broken.
    pub message: String,
}

impl DomainError {
    pub fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}
