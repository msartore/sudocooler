//! Settings screen: polling, data source, units and registry override.

use nsh_base::theme;
use nsh_core::prelude::*;

use super::{AppState, header, rescan};
use crate::composition::AppDeps;
use crate::ui;

pub fn screen(state: AppState) -> impl IntoElement {
    let deps = use_context::<AppDeps>().expect("the app provides its dependencies");

    view()
        .flex_col()
        .gap_4()
        .max_w(px(820.0))
        .child(header(
            "Settings",
            "Tune polling and choose between simulated data and the real daemon.",
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
                        "Data source",
                        "Demo works without an attached cooler",
                        move || if state.demo.get() { "Demo" } else { "Daemon" }.to_owned(),
                        move || {
                            let next = !state.demo.get();
                            deps.demo.set(next);
                            state.settings.update(|settings| {
                                *settings = settings.clone().with_demo(next)
                            });
                            rescan(state);
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
                        "Device registry",
                        "Override with the daemon's --config path",
                        move || {
                            state
                                .settings
                                .get()
                                .config_path
                                .clone()
                                .unwrap_or_else(|| "built-in".to_owned())
                        },
                        move || {
                            // Clear any override and fall back to the embedded
                            // profiles shipped in the submodule.
                            state.settings.update(|settings| {
                                *settings = settings.clone().with_config_path(None)
                            });
                            rescan(state);
                        },
                    )),
            ),
        )
        .child(
            ui::card(
                "About",
                view()
                    .gap_1()
                    .child(
                        text("SK700V · 64-byte HID reports · VID 0x381C")
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
