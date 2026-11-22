//! Device registry view with the daemon's safety state made explicit.

use nsh_base::theme;
use nsh_core::prelude::*;

use super::{AppState, header, rescan};
use crate::domain::device::Device;
use crate::ui::{self, Tone};

pub fn screen(state: AppState) -> impl IntoElement {
    view()
        .flex_col()
        .gap_4()
        .max_w(px(980.0))
        .child(header(
            "Devices",
            "Only enabled profiles with VID 0x381C are ever written to.",
        ))
        .child(view().flex_row().gap_2().child(ui::action(
            || "Rescan USB".to_owned(),
            true,
            move || rescan(state),
        )))
        .child(ui::card(
            "USB HID interfaces",
            each(
                state.devices,
                |device| (device.vid, device.pid),
                move |device, _| device_row(device),
            ),
        ))
}

fn device_row(device: ReadSignal<Device>) -> impl IntoElement {
    let tone = if device.get().is_writable() {
        Tone::Success
    } else if device.get().connected {
        Tone::Warning
    } else {
        Tone::Neutral
    };

    view()
        .flex_row()
        .items_center()
        .gap_3()
        .py(px(9.0))
        .border_b(px(1.0))
        .border_color(move || theme().border)
        .child(
            view()
                .flex_1()
                .gap_1()
                .child(
                    text(move || device.get().name.clone())
                        .text_size(px(13.0))
                        .font_weight(FontWeight::MEDIUM),
                )
                .child(
                    text(move || {
                        format!(
                            "{} · {} · {}",
                            device.get().id(),
                            device.get().protocol.label(),
                            if device.get().verified {
                                "verified"
                            } else {
                                "unverified"
                            }
                        )
                    })
                    .text_size(px(11.0))
                    .text_color(move || theme().muted_foreground),
                ),
        )
        .child(ui::pill(
            move || device.get().status_label().to_owned(),
            tone,
        ))
}
