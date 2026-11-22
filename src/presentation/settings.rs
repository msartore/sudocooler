//! Settings screen: only the controls that affect live behaviour.

use nsh_base::theme;
use nsh_core::prelude::*;

use super::{AppState, header};
use crate::composition;
use crate::ui;

pub fn screen(state: AppState) -> impl IntoElement {

    view()
        .flex_col()
        .gap_4()
        .max_w(px(820.0))
        .child(header(
            "Settings",
            "Polling speed and the temperature unit sent to the cooler.",
        ))
        .child(
            ui::card(
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
                                    state.settings.update(|settings| {
                                        *settings = settings.clone().stepped(-100)
                                    })
                                },
                            ))
                            .child(ui::action(
                                || "+100 ms".to_owned(),
                                false,
                                move || {
                                    state.settings.update(|settings| {
                                        *settings = settings.clone().stepped(100)
                                    })
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
                            let _ = composition::set_autostart(next);
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
            ),
        )
        .child(
            ui::card(
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
            ),
        )
        .child(
            ui::card(
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
            ),
        )
}

fn link_row(icon: nsh_core::element::ImageAsset, title: &'static str, url: &'static str) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .py(px(10.0))
        .on_press(move || {
            let _ = open_url(url);
        })
        .child(
            image(icon)
                .size(px(16.0))
                .tint_color(move || theme().muted_foreground),
        )
        .child(
            view()
                .flex_1()
                .child(text(title).text_size(px(13.0)).font_weight(FontWeight::MEDIUM))
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
