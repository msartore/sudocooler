//! Screens and the app shell. Presentation reads use cases and renders
//! `ScreenState`s; no process, platform or transport code lives here.

pub mod dashboard;
pub mod devices;
pub mod packets;
pub mod settings;

use std::time::Duration;

use nsh_base::theme;
use nsh_core::desktop::WindowAttributes;
use nsh_core::element::desktop::{MenuItem, current_window, set_tray, tray};
use nsh_core::prelude::*;

use crate::application::use_cases::{PacketEntry, PacketLog};
use crate::composition::{self, AppDeps};
use crate::domain::device::Device;
use crate::domain::settings::Settings;
use crate::domain::telemetry::Telemetry;

/// Profile default from `profiles/default_devices.toml`.
const POWER_BAR_MAX_W: u16 = 160;

/// The section shown in the content column.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Devices,
    Packets,
    Settings,
}

/// All state the screens share. Signals are `Copy` handles, so screens get a
/// cheap snapshot of the app without owning a global singleton.
#[derive(Clone, Copy)]
pub struct AppState {
    pub telemetry: Signal<Telemetry>,
    pub devices: Signal<Vec<Device>>,
    pub packets: Signal<PacketLog>,
    pub running: Signal<bool>,
    pub settings: Signal<Settings>,
    pub backend: Signal<&'static str>,
    pub error: Signal<Option<String>>,
    /// Status of the last update check or install.
    pub update_status: Signal<String>,
    /// Heartbeat counter, used to pace sampling at the configured interval.
    pub tick: Signal<u64>,
}

impl AppState {
    pub fn power_bar_pct(&self) -> u8 {
        self.telemetry.get().power_bar_pct(POWER_BAR_MAX_W)
    }
}

/// Root mounted by `nsh.json` and `cargo run`.
pub fn app() -> impl IntoElement {
    let initial = Telemetry::new(47.0, 16, 40, 5025).expect("valid initial sample");
    let state = AppState {
        telemetry: signal(initial),
        devices: signal(composition::discover()),
        packets: signal(PacketLog::default()),
        running: signal(true),
        settings: signal(Settings::default()),
        backend: signal("sudocooler CLI"),
        error: signal(None),
        update_status: signal("Not checked".to_owned()),
        tick: signal(0_u64),
    };

    provide_context(AppDeps::new());

    // Closing the window hides it into the tray; the Quit tray item exits.
    // Set the attribute in code as well as nsh.json: direct cargo runs do not
    // load the app manifest.
    let window = current_window();
    let mut attributes = WindowAttributes::default();
    attributes.close_on_press = Some(false);
    window.set(attributes);
    let hide_on_close = window.clone();
    window.on_close_press(move |_| hide_on_close.hide());

    // System tray: always available, with a live pause/resume action.
    let tray_running = state.running;
    effect(move || {
        let running = tray_running.get();
        set_tray(
            tray()
                .icon(crate::LOGO)
                .tooltip("sudocooler")
                .menu([
                    MenuItem::new("Show window").on_select(move || current_window().show()),
                    MenuItem::new(if running { "Pause" } else { "Resume" })
                        .on_select(move || tray_running.update(|value| *value = !*value)),
                    MenuItem::new("Quit").on_select(move || {
                        composition::close_l120();
                        std::process::exit(0);
                    }),
                ])
                .on_click(move |_| current_window().show()),
        );
    });

    // A 100 ms heartbeat; sampling happens only when the configured interval
    // has elapsed. Tasks and timers end with the view that created them.
    set_interval(Duration::from_millis(100), move || {
        let tick = state.tick.get() + 1;
        state.tick.set(tick);
        if !state.running.get() {
            return;
        }
        let settings = state.settings.get();
        let period = (settings.interval_ms / 100).max(1) as u64;
        if !tick.is_multiple_of(period) {
            return;
        }
        match composition::sample(settings.config_path.as_deref()) {
            Ok(result) => {
                batch(|| {
                    state.telemetry.set(result.telemetry);
                    state.backend.set(result.backend);
                    state.error.set(None);
                    state.packets.update(|log| {
                        let sequence = log.latest().map_or(1, |entry| entry.sequence + 1);
                        let is_l120 = state
                            .devices
                            .get()
                            .iter()
                            .any(|device| device.pid == 0x0005 && device.connected);
                        log.push(if is_l120 {
                            PacketEntry::from_l120_sample(
                                sequence,
                                &result.telemetry,
                                settings.temp_unit,
                            )
                        } else {
                            PacketEntry::from_sample(
                                sequence,
                                &result.telemetry,
                                &settings,
                                POWER_BAR_MAX_W,
                            )
                        });
                    });
                });

                if state
                    .devices
                    .get()
                    .iter()
                    .any(|device| device.pid == 0x0005 && device.connected)
                    && let Err(error) =
                        composition::write_l120(&result.telemetry, settings.temp_unit)
                {
                    state.error.set(Some(format!("L120 write: {error}")));
                }
            }
            Err(error) => state.error.set(Some(error.message)),
        }
    });

    view()
        .flex_1()
        .bg(move || theme().background)
        .text_color(move || theme().foreground)
        .p_safe_area()
        .child(shell(state))
}

/// Rescan devices in the active adapter and publish the result.
pub fn rescan(state: AppState) {
    state.devices.set(composition::discover());
    state.error.set(None);
}

fn shell(state: AppState) -> impl IntoElement {
    let tab = signal(Tab::Dashboard);
    view()
        .flex_1()
        .flex_row()
        .style_when(|| window().width.0 < 700.0, |style| style.flex_col())
        .child(sidebar(state, tab))
        .child(
            scroll().flex_1().p(px(24.0)).child(
                when(
                    move || tab.get() == Tab::Dashboard,
                    move || dashboard::screen(state),
                )
                .otherwise(move || {
                    when(
                        move || tab.get() == Tab::Devices,
                        move || devices::screen(state),
                    )
                    .otherwise(move || {
                        when(
                            move || tab.get() == Tab::Packets,
                            move || packets::screen(state),
                        )
                        .otherwise(move || settings::screen(state))
                    })
                }),
            ),
        )
}

fn sidebar(state: AppState, tab: Signal<Tab>) -> impl IntoElement {
    let items = [
        (
            Tab::Dashboard,
            "Overview",
            asset!("icons/layout-dashboard.svg"),
        ),
        (Tab::Devices, "Devices", asset!("icons/cpu.svg")),
        (Tab::Packets, "Packets", asset!("icons/package.svg")),
        (Tab::Settings, "Settings", asset!("icons/settings.svg")),
    ];

    let mut nav = view().flex_col().gap_1().child(
        view()
            .flex_row()
            .items_center()
            .gap_2()
            .px_3()
            .pb_5()
            .child(image(crate::LOGO).size(px(34.0)))
            .child(
                view()
                    .gap_0p5()
                    .child(
                        text("sudocooler")
                            .text_size(px(15.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(move || theme().foreground),
                    )
                    .child(
                        text("CPU cooler dashboard")
                            .text_size(px(11.0))
                            .text_color(move || theme().muted_foreground),
                    ),
            ),
    );

    for (value, label, icon) in items {
        nav = nav.child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .px_3()
                .py(px(10.0))
                .rounded(px(9.0))
                .bg(move || {
                    if tab.get() == value {
                        theme().sidebar_accent
                    } else {
                        theme().sidebar
                    }
                })
                .on_press(move || tab.set(value))
                .child(image(icon).size(px(17.0)).tint_color(move || {
                    if tab.get() == value {
                        theme().sidebar_foreground
                    } else {
                        theme().muted_foreground
                    }
                }))
                .child(
                    text(label)
                        .text_size(px(13.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(move || {
                            if tab.get() == value {
                                theme().sidebar_foreground
                            } else {
                                theme().muted_foreground
                            }
                        }),
                ),
        );
    }

    view()
        .w(px(196.0))
        .flex_col()
        .p(px(16.0))
        .bg(move || theme().sidebar)
        .child(nav)
        .child(view().flex_grow(1.0))
        .child(
            view().px_3().child(
                text(move || state.backend.get().to_owned())
                    .text_size(px(10.0))
                    .text_color(move || theme().muted_foreground),
            ),
        )
}

pub fn header(title: &'static str, subtitle: &'static str) -> impl IntoElement {
    view()
        .gap_1()
        .child(
            text(title)
                .text_size(px(24.0))
                .font_weight(FontWeight::BOLD)
                .text_color(move || theme().foreground),
        )
        .child(
            text(subtitle)
                .text_size(px(13.0))
                .text_color(move || theme().muted_foreground),
        )
}
