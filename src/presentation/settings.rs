//! Settings screen: only the controls that affect live behaviour.

use nsh_base::theme;
use nsh_core::prelude::*;

use super::{AppState, push_log};
use crate::composition;
use crate::ui;

/// Settings cards, rendered inside the Tools screen.
pub fn cards(state: AppState) -> impl IntoElement {
    view()
        .flex_col()
        .gap_4()
        .child(ui::card(
            "Daemon",
            view()
                .flex_col()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .gap_3()
                        .py(px(10.0))
                        .child(
                            view()
                                .flex_1()
                                .child(text("Polling interval").text_size(px(14.0)))
                                .child(
                                    text("Minimum 100 ms, same as the daemon's CLI")
                                        .text_size(px(11.0))
                                        .text_color(move || theme().muted_foreground),
                                ),
                        )
                        .child(
                            text(move || format!("{} ms", state.settings.get().interval_ms))
                                .text_size(px(13.0))
                                .font_weight(FontWeight::SEMIBOLD),
                        ),
                )
                .child(
                    view()
                        .flex_row()
                        .gap_2()
                        .pb(px(10.0))
                        .child(ui::action(
                            || "-100 ms".to_owned(),
                            false,
                            move || {
                                state
                                    .settings
                                    .update(|settings| *settings = settings.clone().stepped(-100))
                            },
                        ))
                        .child(ui::action(
                            || "+100 ms".to_owned(),
                            false,
                            move || {
                                state
                                    .settings
                                    .update(|settings| *settings = settings.clone().stepped(100))
                            },
                        )),
                )
                .child(ui::setting_row(
                    "Start at login",
                    "Windows logon autostart",
                    move || {
                        if composition::autostart_enabled() {
                            "On".to_owned()
                        } else {
                            "Off".to_owned()
                        }
                    },
                    move || {
                        let next = !composition::autostart_enabled();
                        match composition::set_autostart(next) {
                            Ok(()) => push_log(
                                state,
                                "info",
                                format!("Autostart {}", if next { "enabled" } else { "disabled" }),
                            ),
                            Err(error) => push_log(state, "error", format!("Autostart: {error}")),
                        }
                    },
                ))
                .child(ui::setting_row(
                    "Temperature unit",
                    "Sent as the unit flag in byte 10",
                    move || state.settings.get().temp_unit.label().to_owned(),
                    move || {
                        use crate::domain::packet::TempUnit;
                        state.settings.update(|settings| {
                            let unit = match settings.temp_unit {
                                TempUnit::Celsius => TempUnit::Fahrenheit,
                                TempUnit::Fahrenheit => TempUnit::Celsius,
                            };
                            *settings = settings.clone().with_temp_unit(unit)
                        });
                    },
                ))
                .child(ui::setting_row(
                    "Software update",
                    "Check GitHub Releases for a newer version",
                    move || state.update_status.get(),
                    move || {
                        let status = state.update_status;
                        spawn(async move {
                            let message = match composition::check_update() {
                                Ok(message) => message,
                                Err(error) => format!("Check failed: {error}"),
                            };
                            push_log(state, "info", message.clone());
                            status.set(message);
                        });
                    },
                ))
                .child(ui::setting_row(
                    "Install update",
                    "Download, verify and replace the running app",
                    || "Run".to_owned(),
                    move || {
                        let status = state.update_status;
                        spawn(async move {
                            let message = match composition::install_update() {
                                Ok(message) => message,
                                Err(error) => format!("Install failed: {error}"),
                            };
                            push_log(state, "info", message.clone());
                            status.set(message);
                        });
                    },
                )),
        ))
        .child(ui::card(
            "Links",
            view()
                .flex_col()
                .child(link_row(
                    asset!("icons/brand-github.svg"),
                    "sudocooler app",
                    "https://github.com/msartore/sudocooler",
                ))
                .child(link_row(
                    asset!("icons/brand-github.svg"),
                    "sudokooler daemon",
                    "https://github.com/BGW-10/sudokooler",
                ))
                .child(link_row(
                    asset!("icons/world-www.svg"),
                    "msartore.dev",
                    "https://msartore.dev",
                )),
        ))
        .child(ui::card(
            "About",
            view()
                .gap_1()
                .child(
                    text("Live daemon telemetry · ENERGLO 360 L120 frames stream automatically")
                        .text_size(px(12.0))
                        .font_weight(FontWeight::MEDIUM),
                )
                .child(
                    text("The daemon source lives in the sudokooler/ git submodule.")
                        .text_size(px(11.0))
                        .text_color(move || theme().muted_foreground),
                ),
        ))
}

fn link_row(
    icon: nsh_core::element::ImageAsset,
    title: &'static str,
    url: &'static str,
) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .py(px(10.0))
        .on_press(move || {
            spawn(async move {
                let _ = open_url(url).await;
            });
        })
        .child(
            image(icon)
                .size(px(16.0))
                .tint_color(move || theme().muted_foreground),
        )
        .child(
            view()
                .flex_1()
                .child(
                    text(title)
                        .text_size(px(13.0))
                        .font_weight(FontWeight::MEDIUM),
                )
                .child(
                    text(url)
                        .text_size(px(11.0))
                        .text_color(move || theme().muted_foreground),
                ),
        )
        .child(
            image(asset!("icons/external-link.svg"))
                .size(px(14.0))
                .tint_color(move || theme().muted_foreground),
        )
}
