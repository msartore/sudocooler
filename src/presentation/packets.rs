//! Packet inspector: the same 64-byte frames the daemon writes over USB HID.

use nsh_base::theme;
use nsh_core::prelude::*;

use super::{AppState, header};
use crate::application::use_cases::PacketEntry;
use crate::ui;

pub fn screen(state: AppState) -> impl IntoElement {
    let entries = computed(move || state.packets.get().entries().to_vec());

    view()
        .flex_col()
        .gap_4()
        .max_w(px(980.0))
        .child(header(
            "Packets",
            "Preview of the SK700V telemetry report, zero-padded to 64 bytes.",
        ))
        .child(
            ui::card(
                "Latest frame",
                view()
                    .gap_2()
                    .child(
                        text(move || {
                            let hex = state
                                .packets
                                .get()
                                .latest()
                                .map(|entry| entry.hex.clone())
                                .unwrap_or_default();
                            if hex.is_empty() {
                                "Waiting for the first sample...".to_owned()
                            } else {
                                hex.chars().take(64).collect()
                            }
                        })
                        .font_family(
                            theme()
                                .font_mono
                                .clone()
                                .unwrap_or_else(|| "monospace".into()),
                        )
                        .text_size(px(12.0))
                        .text_color(move || theme().foreground),
                    )
                    .child(
                        text(move || {
                            state
                                .packets
                                .get()
                                .latest()
                                .map(|entry| entry.hex.chars().skip(64).collect::<String>())
                                .unwrap_or_default()
                        })
                        .font_family(
                            theme()
                                .font_mono
                                .clone()
                                .unwrap_or_else(|| "monospace".into()),
                        )
                        .text_size(px(12.0))
                        .text_color(move || theme().foreground),
                    )
                    .child(
                        text("10 68 01 09 0D 01 02 | power | temp f32 | load | freq | csum | 16")
                            .text_size(px(11.0))
                            .text_color(move || theme().muted_foreground),
                    ),
            ),
        )
        .child(
            ui::card(
                "Frame log",
                when(
                    move || state.packets.get().is_empty(),
                    move || {
                        text("No frames yet — resume streaming on the overview.")
                            .text_size(px(12.0))
                            .text_color(move || theme().muted_foreground)
                    },
                )
                .otherwise(move || each(entries, |entry| entry.sequence, move |entry, _| row(entry))),
            ),
        )
}

fn row(entry: ReadSignal<PacketEntry>) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .py(px(7.0))
        .border_b(px(1.0))
        .border_color(move || theme().border)
        .child(
            text(move || format!("#{:03}", entry.get().sequence))
                .text_size(px(11.0))
                .text_color(move || theme().muted_foreground),
        )
        .child(
            text(move || format!("{} C", entry.get().temperature_c))
                .text_size(px(12.0))
                .font_weight(FontWeight::MEDIUM),
        )
        .child(
            text(move || {
                let hex = entry.get().hex;
                let head = hex.chars().take(40).collect::<String>();
                format!("{head}...")
            })
            .font_family(
                theme()
                    .font_mono
                    .clone()
                    .unwrap_or_else(|| "monospace".into()),
            )
            .text_size(px(11.0))
            .text_color(move || theme().muted_foreground),
        )
}
