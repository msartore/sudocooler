//! In-app event log, reachable from Home → Tools → Logs.

use nsh_base::theme;
use nsh_core::element::asset;
use nsh_core::prelude::*;

use super::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub sequence: u32,
    pub time: String,
    pub level: &'static str,
    pub message: String,
}

pub fn screen(state: AppState) -> impl IntoElement {
    view()
        .flex_col()
        .gap_2()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    image(asset!("icons/activity.svg"))
                        .size(px(16.0))
                        .tint_color(move || theme().muted_foreground),
                )
                .child(
                    text("Logs")
                        .text_size(px(14.0))
                        .font_weight(FontWeight::SEMIBOLD),
                )
                .child(
                    text("last 100 events")
                        .text_size(px(11.0))
                        .text_color(move || theme().muted_foreground),
                ),
        )
        .child(when(
            move || state.logs.get().is_empty(),
            move || {
                text("No events yet.")
                    .text_size(px(12.0))
                    .text_color(move || theme().muted_foreground)
            },
        ))
        .child(each(
            state.logs,
            |entry| entry.sequence,
            move |entry, _| {
                view()
                    .flex_row()
                    .items_start()
                    .gap_3()
                    .py(px(6.0))
                    .border_b(px(1.0))
                    .border_color(move || theme().border)
                    .child(
                        text(move || entry.get().time.clone())
                            .text_size(px(11.0))
                            .text_color(move || theme().muted_foreground),
                    )
                    .child(
                        text(move || entry.get().level.to_owned())
                            .text_size(px(11.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(move || {
                                if entry.get().level == "error" {
                                    theme().destructive
                                } else {
                                    theme().muted_foreground
                                }
                            }),
                    )
                    .child(text(move || entry.get().message.clone()).text_size(px(12.0)))
            },
        ))
}
