//! Sudokooler companion UI — a Rust-only NSH app.
//!
//! Layers, from the inside out:
//! `domain` (std only) → `application` (ports/use cases) → `infrastructure`
//! (daemon CLI + demo adapters) → `composition` (wiring) → `presentation`
//! and `ui` (the only consumers of NSH).

pub mod application;
pub mod composition;
pub mod domain;
pub mod infrastructure;
pub mod presentation;
pub mod ui;

use nsh_core::prelude::*;

/// The crate's UI set, should a TSX entry be added later.
#[native_module("sudokooler-ui/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

/// The root component named by `nsh.json` and mounted by `cargo run`.
#[component]
pub fn App() -> impl IntoElement {
    presentation::app()
}
