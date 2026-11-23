//! The live overview: telemetry, cooler display preview and daemon controls.

use nsh_base::theme;
use nsh_core::prelude::*;

use super::{AppState, header, rescan};
use crate::domain::device::Protocol;
use crate::ui;

pub fn screen(state: AppState) -> impl IntoElement {
    view()
        .flex_col()
        .gap_4()
        .max_w(px(1040.0))
        .child(header(
            "Overview",
            "Live CPU telemetry and the frame sent to the cooler display.",
        ))
        .child(error_banner(state))
        .child(sensor_notice(state))
        .child(unsupported_notice(state))
        .child(
            view()
                .flex_row()
                .gap_4()
                .style_when(|| window().width.0 < 900.0, |style| style.flex_col())
                .child(ui::card(
                    "Cooler display preview",
                    ui::segment_display(
                        move || state.temperature_in_unit(),
                        move || state.power_bar_pct(),
                        move || state.unit_label(),
                    ),
                ))
                .child(
                    view()
                        .flex_1()
                        .grid()
                        .grid_cols(2)
                        .gap_3()
                        .child(ui::metric(
                            asset!("icons/temperature.svg"),
                            "Temperature",
                            move || {
                                if state.sampled.get() {
                                    format!("{:.1}", state.temperature_in_unit())
                                } else {
                                    "—".to_owned()
                                }
                            },
                            move || state.unit_label(),
                            true,
                        ))
                        .child(ui::metric(
                            asset!("icons/bolt.svg"),
                            "Package power",
                            move || {
                                if state.sampled.get() {
                                    state.telemetry.get().power_w.to_string()
                                } else {
                                    "—".to_owned()
                                }
                            },
                            || "W",
                            false,
                        ))
                        .child(ui::metric(
                            asset!("icons/activity.svg"),
                            "CPU load",
                            move || {
                                if state.sampled.get() {
                                    state.telemetry.get().usage_pct.to_string()
                                } else {
                                    "—".to_owned()
                                }
                            },
                            || "%",
                            false,
                        ))
                        .child(ui::metric(
                            asset!("icons/clock.svg"),
                            "Core clock",
                            move || {
                                if state.sampled.get() {
                                    state.telemetry.get().frequency_mhz.to_string()
                                } else {
                                    "—".to_owned()
                                }
                            },
                            || "MHz",
                            false,
                        )),
                ),
        )
        .child(
            view()
                .flex_row()
                .flex_wrap()
                .gap_2()
                .child(
                    when(
                        move || state.running.get(),
                        move || {
                            ui::action_with_icon(
                                asset!("icons/player-pause.svg"),
                                || "Pause".to_owned(),
                                true,
                                move || state.running.update(|running| *running = !*running),
                            )
                        },
                    )
                    .otherwise(move || {
                        ui::action_with_icon(
                            asset!("icons/player-play.svg"),
                            || "Resume".to_owned(),
                            true,
                            move || state.running.update(|running| *running = !*running),
                        )
                    }),
                )
                .child(ui::action_with_icon(
                    asset!("icons/refresh.svg"),
                    || "Rescan devices".to_owned(),
                    false,
                    move || rescan(state),
                )),
        )
}

/// The daemon's own explanation for zero temperature/power readings.
fn sensor_notice(state: AppState) -> impl IntoElement {
    when(
        move || {
            let sample = state.telemetry.get();
            state.sampled.get() && sample.temperature_c == 0.0 && sample.power_w == 0
        },
        move || {
            view()
                .p_3()
                .rounded(px(10.0))
                .bg(move || {
                    theme()
                        .color("warning")
                        .unwrap_or(theme().destructive)
                        .opacity(0.14)
                })
                .child(
                    text(move || {
                        use crate::application::PawnIoStatus;
                        match state.pawnio.get() {
                            Some(PawnIoStatus::Denied) => {
                                "PawnIO access denied: sudocooler is not running as administrator. Restart it as administrator to read CPU temperature and power.".to_owned()
                            }
                            Some(PawnIoStatus::NotInstalled) => {
                                "CPU temperature and power need PawnIO, which is not installed. Open Settings → Set up PawnIO.".to_owned()
                            }
                            Some(PawnIoStatus::Failed(error)) => format!("PawnIO error: {error}"),
                            _ => state.sensor_notice.get().unwrap_or_else(|| {
                                "CPU temperature and power are unavailable; the daemon did not report a reason."
                                    .to_owned()
                            }),
                        }
                    })
                    .text_size(px(12.0))
                    .text_color(move || theme().foreground),
                )
                .child(when(
                    move || matches!(state.pawnio.get(), Some(crate::application::PawnIoStatus::Denied)),
                    move || {
                        ui::action(
                            || "Restart as administrator".to_owned(),
                            true,
                            move || crate::presentation::request_admin(state),
                        )
                    },
                ))
        },
    )
}

/// Shown for a detected cooler outside the daemon's verified profile set.
fn unsupported_notice(state: AppState) -> impl IntoElement {
    when(
        move || {
            state
                .devices
                .get()
                .iter()
                .any(|device| device.connected && !device.enabled)
        },
        move || {
            view()
                .p_3()
                .rounded(px(10.0))
                .bg(move || theme().muted.opacity(0.45))
                .child(
                    text(move || {
                        state
                            .devices
                            .get()
                            .iter()
                            .find(|device| device.connected && !device.enabled)
                            .map(|device| {
                                if device.protocol == Protocol::L120 {
                                    format!(
                                        "{} detected ({}). Live L120 frames are streamed to your cooler automatically; the preview uses the SK700V seven-segment layout.",
                                        device.name,
                                        device.id()
                                    )
                                } else {
                                    format!(
                                        "{} detected ({}). This model is not yet verified, so it stays read-only until the daemon supports it.",
                                        device.name,
                                        device.id()
                                    )
                                }
                            })
                            .unwrap_or_default()
                    })
                    .text_size(px(12.0))
                    .text_color(move || theme().foreground),
                )
        },
    )
}

fn error_banner(state: AppState) -> impl IntoElement {
    when(
        move || state.error.get().is_some(),
        move || {
            view()
                .p_3()
                .rounded(px(10.0))
                .bg(move || theme().destructive.opacity(0.12))
                .child(
                    text(move || state.error.get().unwrap_or_default())
                        .text_size(px(12.0))
                        .text_color(move || theme().destructive),
                )
        },
    )
}
