//! The live overview: telemetry, cooler display preview and daemon controls.

use nsh_base::theme;
use nsh_core::prelude::*;

use super::{AppState, header, rescan};
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
        .child(unsupported_notice(state))
        .child(
            view()
                .flex_row()
                .gap_4()
                .style_when(|| window().width.0 < 900.0, |style| style.flex_col())
                .child(
                    ui::card(
                        "Cooler display preview",
                        ui::segment_display(
                            move || state.telemetry.get().temperature_c,
                            move || state.power_bar_pct(),
                        ),
                    ),
                )
                .child(
                    view()
                        .flex_1()
                        .grid()
                        .grid_cols(2)
                        .gap_3()
                        .child(ui::metric(
                            asset!("icons/temperature.svg"),
                            "Temperature",
                            move || format!("{:.1}", state.telemetry.get().temperature_c),
                            "C",
                            true,
                        ))
                        .child(ui::metric(
                            asset!("icons/bolt.svg"),
                            "Package power",
                            move || state.telemetry.get().power_w.to_string(),
                            "W",
                            false,
                        ))
                        .child(ui::metric(
                            asset!("icons/activity.svg"),
                            "CPU load",
                            move || state.telemetry.get().usage_pct.to_string(),
                            "%",
                            false,
                        ))
                        .child(ui::metric(
                            asset!("icons/clock.svg"),
                            "Core clock",
                            move || state.telemetry.get().frequency_mhz.to_string(),
                            "MHz",
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

/// Shown when a detected device is outside the daemon's verified profile set.
fn unsupported_notice(state: AppState) -> impl IntoElement {
    when(
        move || state.devices.get().iter().any(|device| device.connected && !device.enabled),
        move || {
            view()
                .p_3()
                .rounded(px(10.0))
                .bg(move || theme().muted.opacity(0.45))
                .child(
                    text("ENERGLO 360 detected (PID 381C:0005). The seven-segment preview emulates SK700V; live L120 frames are streamed to your cooler automatically.")
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
