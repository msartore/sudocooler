//! Settings panel: polling, display, updates, about and links.

use nsh_base::theme;
use nsh_core::prelude::*;

use super::AppState;
use crate::composition;
use crate::domain::packet::TempUnit;
use crate::ui;

/// Settings cards, rendered inside the modal. Two columns on a wide window,
/// stacked on a narrow one (like colorutil's Settings screen).
pub fn cards(state: AppState) -> impl IntoElement {
    view()
        .flex_row()
        .gap_4()
        .items_start()
        .style_when(|| window().width.0 < 760.0, |style| style.flex_col())
        .child(
            view()
                .flex_col()
                .gap_4()
                .flex_1()
                .child(daemon_panel(state))
                .child(updates_panel(state))
                .child(about_panel()),
        )
        .child(
            view()
                .flex_col()
                .gap_4()
                .flex_1()
                .child(display_panel(state))
                .child(links_panel()),
        )
}

fn daemon_panel(state: AppState) -> impl IntoElement {
    ui::panel(
        "Daemon",
        "Sampling and startup behaviour",
        view()
            .flex_col()
            .gap_3()
            .child(
                view()
                    .flex_col()
                    .gap_2()
                    .child(
                        view()
                            .flex_row()
                            .items_baseline()
                            .justify_between()
                            .child(
                                text("Polling interval")
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::MEDIUM),
                            )
                            .child(
                                text(move || format!("{} ms", state.settings.get().interval_ms))
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(move || theme().primary),
                            ),
                    )
                    .child(
                        text("How often the daemon is asked for a fresh sample")
                            .text_size(px(11.0))
                            .text_color(move || theme().muted_foreground),
                    )
                    .child(
                        view()
                            .flex_row()
                            .gap_2()
                            .child(ui::choice(
                                || "-100 ms".to_owned(),
                                || false,
                                move || {
                                    state.settings.update(|settings| {
                                        *settings = settings.clone().stepped(-100)
                                    });
                                },
                            ))
                            .child(ui::choice(
                                || "+100 ms".to_owned(),
                                || false,
                                move || {
                                    state.settings.update(|settings| {
                                        *settings = settings.clone().stepped(100)
                                    });
                                },
                            )),
                    ),
            )
            .child(ui::toggle_row(
                composition::autostart_enabled,
                "Start at login",
                "Launch sudocooler when you sign in",
                move |_| {
                    let next = !composition::autostart_enabled();
                    let _ = composition::set_autostart(next);
                },
            )),
    )
}

fn display_panel(state: AppState) -> impl IntoElement {
    let unit = move || state.settings.get().temp_unit;
    ui::panel(
        "Display",
        "Units sent to the cooler",
        view()
            .flex_col()
            .gap_2()
            .child(
                text("Temperature unit")
                    .text_size(px(13.0))
                    .font_weight(FontWeight::MEDIUM),
            )
            .child(
                text("The unit flag embedded in every telemetry frame")
                    .text_size(px(11.0))
                    .text_color(move || theme().muted_foreground),
            )
            .child(
                view()
                    .flex_row()
                    .gap_2()
                    .child(ui::choice(
                        || "Celsius".to_owned(),
                        move || unit() == TempUnit::Celsius,
                        move || {
                            state.settings.update(|settings| {
                                *settings = settings.clone().with_temp_unit(TempUnit::Celsius)
                            });
                        },
                    ))
                    .child(ui::choice(
                        || "Fahrenheit".to_owned(),
                        move || unit() == TempUnit::Fahrenheit,
                        move || {
                            state.settings.update(|settings| {
                                *settings = settings.clone().with_temp_unit(TempUnit::Fahrenheit)
                            });
                        },
                    )),
            ),
    )
}

fn updates_panel(state: AppState) -> impl IntoElement {
    ui::panel(
        "Updates",
        "GitHub Releases, verified before install",
        view()
            .flex_col()
            .gap_3()
            .child(ui::key_value("version", || {
                env!("CARGO_PKG_VERSION").to_owned()
            }))
            .child(
                text(move || state.update_status.get())
                    .text_size(px(11.0))
                    .text_color(move || theme().muted_foreground),
            )
            .child(
                view()
                    .flex_row()
                    .gap_2()
                    .child(ui::choice(
                        || "Check now".to_owned(),
                        || false,
                        move || {
                            let status = state.update_status;
                            spawn(async move {
                                let message = match composition::check_update() {
                                    Ok(message) => message,
                                    Err(error) => format!("Check failed: {error}"),
                                };
                                status.set(message);
                            });
                        },
                    ))
                    .child(ui::choice(
                        || "Install update".to_owned(),
                        || false,
                        move || {
                            let status = state.update_status;
                            spawn(async move {
                                let message = match composition::install_update() {
                                    Ok(message) => message,
                                    Err(error) => format!("Install failed: {error}"),
                                };
                                status.set(message);
                            });
                        },
                    )),
            ),
    )
}

fn about_panel() -> impl IntoElement {
    ui::panel(
        "About",
        "sudocooler",
        view()
            .flex_col()
            .gap_2()
            .child(
                text(concat!("Version ", env!("CARGO_PKG_VERSION")))
                    .text_size(px(13.0))
                    .font_weight(FontWeight::SEMIBOLD),
            )
            .child(
                text("A native companion app for Sudokoo CPU coolers.")
                    .text_size(px(12.0))
                    .text_color(move || theme().muted_foreground),
            )
            .child(
                text("Built with Native Shell · GPL-3.0 · msartore.dev")
                    .text_size(px(11.0))
                    .text_color(move || theme().muted_foreground),
            )
            .child(ui::key_value("App id", || {
                "dev.msartore.sudocooler".to_owned()
            })),
    )
}

fn links_panel() -> impl IntoElement {
    ui::panel(
        "Links",
        "Project and credits",
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
    )
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
        .py(px(8.0))
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
