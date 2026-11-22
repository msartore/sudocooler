//! sudocooler — a Rust-only NSH app for Sudokoo CPU coolers.
//!
//! Layers, from the inside out:
//! `domain` (std only) → `application` (ports/use cases) → `infrastructure`
//! (daemon CLI, OS adapters) → `composition` (wiring) → `presentation`
//! and `ui` (the only consumers of NSH). There is no demo mode: readings
//! always come from the real daemon.

pub mod application;
pub mod composition;
pub mod domain;
pub mod infrastructure;
pub mod presentation;
pub mod ui;

use nsh_core::prelude::*;

/// The companion's logo, shipped from `assets/logo.png`.
pub const LOGO: ImageAsset = asset!("logo.png");

/// The crate's UI set, should a TSX entry be added later.
#[native_module("sudocooler/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>().assets(assets!());
}

/// The root component named by `nsh.json` and mounted by `cargo run`.
#[component]
pub fn App() -> impl IntoElement {
    presentation::app()
}
