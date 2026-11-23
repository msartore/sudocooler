//! Screens and the app shell. Presentation reads use cases and renders
//! `ScreenState`s; no process, platform or transport code lives here.

pub mod dashboard;
pub mod settings;

use std::time::Duration;

use nsh_base::theme;
use nsh_core::desktop::WindowAttributes;
use nsh_core::element::desktop::{MenuItem, current_window, on_app_quit, set_tray, tray};
use nsh_core::prelude::*;
use nsh_core::storage::Storage;

use crate::application::PawnIoStatus;
use crate::composition::{self, AppDeps};
use crate::domain::device::Device;
use crate::domain::settings::Settings;
use crate::domain::telemetry::Telemetry;
use crate::ui;

/// Profile default from `profiles/default_devices.toml`.
const POWER_BAR_MAX_W: u16 = 160;

const KEY_INTERVAL: &str = "settings.interval_ms";
const KEY_UNIT: &str = "settings.temp_unit";

/// Loads persisted settings, falling back to defaults for anything missing or
/// invalid (so a corrupt value never stops the app from starting).
fn load_settings() -> Settings {
    let mut settings = Settings::default();
    if let Some(interval) = Storage::get_item(KEY_INTERVAL).and_then(|v| v.parse::<u32>().ok())
        && let Ok(next) = settings.clone().with_interval(interval)
    {
        settings = next;
    }
    if let Some(unit) = Storage::get_item(KEY_UNIT) {
        let unit = match unit.as_str() {
            "fahrenheit" => crate::domain::packet::TempUnit::Fahrenheit,
            _ => crate::domain::packet::TempUnit::Celsius,
        };
        settings = settings.with_temp_unit(unit);
    }
    settings
}

fn save_settings(settings: &Settings) {
    let _ = Storage::set_item(KEY_INTERVAL, settings.interval_ms.to_string());
    let unit = match settings.temp_unit {
        crate::domain::packet::TempUnit::Celsius => "celsius",
        crate::domain::packet::TempUnit::Fahrenheit => "fahrenheit",
    };
    let _ = Storage::set_item(KEY_UNIT, unit);
}

/// All state the screens share. Signals are `Copy` handles, so screens get a
/// cheap snapshot of the app without owning a global singleton.
#[derive(Clone, Copy)]
pub struct AppState {
    pub telemetry: Signal<Telemetry>,
    /// Whether a real sample has landed yet; until then the UI shows dashes.
    pub sampled: Signal<bool>,
    pub devices: Signal<Vec<Device>>,
    pub running: Signal<bool>,
    pub settings: Signal<Settings>,
    pub backend: Signal<&'static str>,
    pub error: Signal<Option<String>>,
    /// Status of the last update check or install.
    pub update_status: Signal<String>,
    /// The daemon's own reason for missing telemetry, when readings are zero.
    pub sensor_notice: Signal<Option<String>>,
    /// What this process can do with PawnIO; `None` until probed.
    pub pawnio: Signal<Option<PawnIoStatus>>,
    /// Whether the app is registered to start at logon. A signal so the
    /// toggle repaints immediately after a change.
    pub autostart: Signal<bool>,
    /// Heartbeat counter, used to pace sampling at the configured interval.
    pub tick: Signal<u64>,
    /// True while a sample is in flight, so ticks never pile up.
    pub sampling: Signal<bool>,
}

impl AppState {
    pub fn power_bar_pct(&self) -> u8 {
        self.telemetry.get().power_bar_pct(POWER_BAR_MAX_W)
    }

    /// The temperature in the unit the cooler is set to display.
    pub fn temperature_in_unit(&self) -> f32 {
        let celsius = self.telemetry.get().temperature_c;
        match self.settings.get().temp_unit {
            crate::domain::packet::TempUnit::Celsius => celsius,
            crate::domain::packet::TempUnit::Fahrenheit => celsius * 9.0 / 5.0 + 32.0,
        }
    }

    /// `"C"` or `"F"`, matching [`Self::temperature_in_unit`].
    pub fn unit_label(&self) -> &'static str {
        self.settings.get().temp_unit.label()
    }
}

/// Root mounted by `nsh.json` and `cargo run`.
pub fn app() -> impl IntoElement {
    // A second copy while the elevated one runs cannot reach NSH's mutex across
    // the integrity boundary; focus the existing window and exit before this
    // one is shown (two samplers must never write the same cooler).
    let _ = crate::infrastructure::elevation::focus_running_instance();

    // No fabricated readings: the UI shows dashes until the first real sample.
    let initial = Telemetry::new(0.0, 0, 0, 0).expect("valid zero sample");
    let state = AppState {
        telemetry: signal(initial),
        sampled: signal(false),
        // Do not block the first frame on the OS HID scan: on Windows it shells
        // out to PowerShell, which can take several seconds on a cold start.
        // The real scan runs off-thread just below and fills this in.
        devices: signal(Vec::new()),
        running: signal(true),
        settings: signal(load_settings()),
        backend: signal("sudocooler CLI"),
        error: signal(None),
        update_status: signal("Not checked".to_owned()),
        sensor_notice: signal(None),
        pawnio: signal(None),
        autostart: signal(false),
        tick: signal(0_u64),
        sampling: signal(false),
    };

    provide_context(AppDeps::new());

    // Run the real OS HID scan on a blocking thread so the first frame paints
    // immediately. Device data is only read by the unsupported-device notice
    // and the L120 check, both of which tolerate an empty list until it lands.
    spawn(async move {
        if let Ok(devices) = blocking(composition::discover).await {
            state.devices.set(devices);
        }
    });

    // Read the logon-autostart state off-thread too (Windows reads the Run key
    // and retires a task left by an older build).
    spawn(async move {
        if let Ok(enabled) = blocking(|| {
            composition::migrate_autostart();
            composition::autostart_enabled()
        })
        .await
        {
            state.autostart.set(enabled);
        }
    });

    // Probe PawnIO off-thread (it loads a DLL and opens a kernel device).
    refresh_pawnio(state);

    // Persist settings whenever they change, so polling and unit survive a
    // restart.
    let settings_to_save = state.settings;
    effect(move || {
        let settings = settings_to_save.get();
        save_settings(&settings);
    });

    // The close button hides the window into the tray; the tray's Quit is the
    // real exit. `close_on_press` false routes the press to this handler. Set
    // the attribute in code too: direct cargo runs do not load the manifest.
    let window = current_window();
    let mut attributes = WindowAttributes::default();
    attributes.close_on_press = Some(false);
    window.set(attributes);
    let hide_on_close = window.clone();
    window.on_close_press(move |_| hide_on_close.hide());

    // A sign-in launch (`--minimized`) keeps the tray icon and hides the
    // window once the first frame has been shown: NSH only applies `set_tray`
    // and the other desktop commands in a shown window.
    if std::env::args().any(|arg| arg == "--minimized") {
        let hidden = window.clone();
        spawn(async move {
            timer(Duration::from_millis(400)).await;
            hidden.hide();
        });
    }

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
                        composition::stop_sampling();
                        composition::close_l120();
                        std::process::exit(0);
                    }),
                ])
                .on_click(move |_| current_window().show()),
        );
    });

    // Pausing stops the sampling process too, so it cannot fill its pipe with
    // readings nobody reads; resuming starts a new one on the next tick.
    let was_running = std::cell::Cell::new(true);
    effect(move || {
        let running = state.running.get();
        if was_running.get() && !running {
            // Off the UI thread: the sampler may be in the middle of a read.
            spawn(async {
                let _ = blocking(composition::stop_sampling).await;
            });
        }
        was_running.set(running);
    });

    // Every exit path (the tray's Quit included) must take the daemon with it.
    on_app_quit(|| async {
        composition::stop_sampling();
    });

    // A 100 ms heartbeat. Sampling runs on a blocking thread so spawning the
    // daemon process never stalls the UI; `sampling` keeps ticks from piling up
    // while a sample is in flight.
    set_interval(Duration::from_millis(100), move || {
        let tick = state.tick.get() + 1;
        state.tick.set(tick);
        if !state.running.get() || state.sampling.get() {
            return;
        }
        let settings = state.settings.get();
        // Round the interval up to the heartbeat's 100 ms grid, so we never
        // sample faster than requested.
        let period = u64::from(settings.interval_ms.max(100)).div_ceil(100);
        if !(tick - 1).is_multiple_of(period) {
            return;
        }
        let config = settings.config_path.clone();
        let unit = settings.temp_unit;
        let is_l120 = state
            .devices
            .get()
            .iter()
            .any(|device| device.pid == 0x0005 && device.connected);

        state.sampling.set(true);
        spawn(async move {
            let outcome = blocking(move || {
                let sample = composition::sample(config.as_deref(), settings.interval_ms);
                let write_error = match (&sample, is_l120) {
                    (Ok(result), true) => composition::write_l120(&result.telemetry, unit).err(),
                    _ => None,
                };
                (sample, write_error)
            })
            .await;

            match outcome {
                Ok((Ok(result), write_error)) => batch(|| {
                    state.telemetry.set(result.telemetry);
                    state.sampled.set(true);
                    state.backend.set(result.backend);
                    state.sensor_notice.set(result.notice);
                    state
                        .error
                        .set(write_error.map(|error| format!("L120 write: {error}")));
                }),
                Ok((Err(error), _)) => {
                    state.sensor_notice.set(None);
                    state.error.set(Some(error.message));
                }
                Err(_) => {
                    state.sensor_notice.set(None);
                    state
                        .error
                        .set(Some("telemetry sampling failed".to_owned()));
                }
            }
            state.sampling.set(false);
        });
    });

    view()
        .flex_1()
        .bg(move || theme().background)
        .text_color(move || theme().foreground)
        .p_safe_area()
        .child(shell(state))
}

/// Re-probe PawnIO off the UI thread and publish the result.
pub fn refresh_pawnio(state: AppState) {
    spawn(async move {
        if let Ok(status) = blocking(composition::pawnio_status).await {
            state.pawnio.set(Some(status));
        }
    });
}

/// Asks Windows for administrator rights; when the elevated copy takes over,
/// this process exits (the sampling daemon goes with it).
pub fn request_admin(state: AppState) {
    let status = state.update_status;
    let error = state.error;
    spawn(async move {
        match blocking(composition::request_elevation).await {
            Ok(Ok(true)) => {
                let _ = blocking(composition::stop_sampling).await;
                std::process::exit(0);
            }
            Ok(Ok(false)) => status.set("Elevation cancelled.".to_owned()),
            Ok(Err(message)) => error.set(Some(format!("Elevation: {message}"))),
            Err(_) => error.set(Some("Elevation request failed".to_owned())),
        }
    });
}

/// Rescan devices in the active adapter and publish the result. The scan
/// shells out on Windows, so it runs off the UI thread.
pub fn rescan(state: AppState) {
    spawn(async move {
        if let Ok(devices) = blocking(composition::discover).await {
            state.devices.set(devices);
            state.error.set(None);
        }
    });
}

fn shell(state: AppState) -> impl IntoElement {
    let settings_open = signal(false);
    view()
        .flex_1()
        .child(top_bar(settings_open))
        .child(
            scroll()
                .flex_1()
                .p(px(24.0))
                .child(dashboard::screen(state)),
        )
        .child(when(
            move || settings_open.get(),
            move || settings_modal(state, settings_open),
        ))
}

/// Top bar: branding on the left, Settings (modal) on the right.
fn top_bar(settings_open: Signal<bool>) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .px_4()
        .pt_3()
        .child(image(crate::LOGO).size(px(28.0)))
        .child(
            text("sudocooler")
                .text_size(px(15.0))
                .font_weight(FontWeight::BOLD)
                .text_color(move || theme().foreground),
        )
        .child(view().flex_1())
        .child(ui::action_with_icon(
            asset!("icons/settings.svg"),
            || "Settings".to_owned(),
            false,
            move || settings_open.set(true),
        ))
}

/// A modal settings panel over the Home screen.
///
/// Two sibling overlays: a full-screen backdrop and a centered panel. A plain
/// `view().absolute()` is laid out inside the flex parent and gets clipped;
/// `overlay()` sits outside the parent's layout and covers the root view.
fn settings_modal(state: AppState, open: Signal<bool>) -> Vec<impl IntoElement> {
    let backdrop = overlay()
        .inset_0()
        .bg(rgba(0x00000099))
        .on_press(move || open.set(false));

    let panel = overlay()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .child(
            view()
                .w(percentage(0.92))
                .max_w(px(1080.0))
                .h(percentage(0.88))
                .max_h(px(760.0))
                .flex_col()
                .gap(px(16.0))
                .p(px(24.0))
                .rounded(move || theme().radius)
                .border_1()
                .border_color(move || theme().border)
                .bg(move || theme().popover)
                .child(
                    view()
                        .flex_row()
                        .items_start()
                        .justify_between()
                        .gap_4()
                        .child(
                            view()
                                .flex_col()
                                .gap(px(2.0))
                                .child(
                                    text("Settings")
                                        .text_size(px(25.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(move || theme().foreground),
                                )
                                .child(
                                    text("Polling, display, updates and about")
                                        .text_size(px(12.0))
                                        .text_color(move || theme().muted_foreground),
                                ),
                        )
                        .child(
                            view()
                                .px(px(12.0))
                                .py(px(6.0))
                                .rounded(px(7.0))
                                .border_1()
                                .border_color(move || theme().border)
                                .on_press(move || open.set(false))
                                .child(
                                    text("Close")
                                        .text_size(px(12.0))
                                        .text_color(move || theme().foreground),
                                ),
                        ),
                )
                .child(scroll().flex_1().child(settings::cards(state))),
        );

    vec![backdrop.into_element(), panel.into_element()]
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
