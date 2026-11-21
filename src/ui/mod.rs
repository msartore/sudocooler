//! The app's own component library: styled controls built on NSH primitives.
//!
//! Screens use these instead of importing `nsh_base` directly, so a design
//! change happens here, once.

use nsh_base::theme;
use nsh_core::prelude::*;

/// The cooler's illuminated segment color.
pub fn display_on() -> AdaptiveColor {
    theme()
        .color("display-on")
        .unwrap_or_else(|| theme().color("warning").unwrap_or(theme().primary))
}

/// The cooler's unlit segment color.
pub fn display_off() -> AdaptiveColor {
    theme()
        .color("display-off")
        .unwrap_or_else(|| theme().muted.opacity(0.35))
}

/// The power bar's blue accent.
pub fn display_bar() -> AdaptiveColor {
    theme()
        .color("display-bar")
        .unwrap_or_else(|| theme().primary)
}

fn tone_color(tone: Tone) -> AdaptiveColor {
    match tone {
        Tone::Success => theme()
            .color("success")
            .unwrap_or_else(|| theme().color("chart-1").unwrap_or(theme().primary)),
        Tone::Warning => theme()
            .color("warning")
            .unwrap_or_else(|| theme().destructive),
        Tone::Danger => theme().destructive,
        Tone::Neutral => theme().muted_foreground,
    }
}

/// Semantic color for status pills.
#[derive(Clone, Copy)]
pub enum Tone {
    Success,
    Warning,
    Danger,
    Neutral,
}

/// A grouped surface with a title and one or more children.
pub fn card(title: &'static str, body: impl IntoElement) -> impl IntoElement {
    view()
        .flex_col()
        .gap_3()
        .p(px(18.0))
        .rounded(move || theme().radius)
        .border_1()
        .border_color(move || theme().border)
        .bg(move || theme().card)
        .child(
            text(title)
                .text_size(px(12.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().muted_foreground),
        )
        .child(body)
}

/// A status pill with a reactive label.
pub fn pill(label: impl Fn() -> String + 'static, tone: Tone) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .px(px(10.0))
        .py(px(5.0))
        .rounded_full()
        .bg(move || tone_color(tone).opacity(0.14))
        .child(
            view()
                .size(px(7.0))
                .rounded_full()
                .bg(move || tone_color(tone)),
        )
        .child(
            text(label)
                .text_size(px(12.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || tone_color(tone)),
        )
}

/// A compact label/value tile.
pub fn metric(
    label: &'static str,
    value: impl Fn() -> String + 'static,
    unit: &'static str,
    accent: bool,
) -> impl IntoElement {
    view()
        .flex_col()
        .gap_1()
        .p(px(16.0))
        .rounded(px(12.0))
        .bg(move || theme().muted.opacity(if accent { 0.65 } else { 0.35 }))
        .child(
            text(label)
                .text_size(px(11.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().muted_foreground),
        )
        .child(
            view()
                .flex_row()
                .items_end()
                .gap_1()
                .child(
                    text(value)
                        .text_size(px(26.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(move || {
                            if accent {
                                display_on()
                            } else {
                                theme().foreground
                            }
                        }),
                )
                .child(
                    text(unit)
                        .text_size(px(12.0))
                        .text_color(move || theme().muted_foreground),
                ),
        )
}

/// The app's button: `nsh_base::button` owns behavior, this owns the look.
pub fn action(
    label: impl Fn() -> String + 'static,
    primary: bool,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    nsh_base::button()
        .px(px(16.0))
        .py(px(9.0))
        .rounded(px(10.0))
        .bg(move || {
            if primary {
                theme().primary
            } else {
                theme().secondary
            }
        })
        .active(|style| style.opacity(0.7))
        .on_press(on_press)
        .child(
            text(label)
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || {
                    if primary {
                        theme().primary_foreground
                    } else {
                        theme().foreground
                    }
                }),
        )
}

/// A pressable settings row with a reactive trailing label.
pub fn setting_row(
    title: &'static str,
    detail: &'static str,
    value: impl Fn() -> String + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .gap_3()
        .py(px(10.0))
        .on_press(on_press)
        .child(
            view()
                .flex_1()
                .gap_1()
                .child(text(title).text_size(px(14.0)).font_weight(FontWeight::MEDIUM))
                .child(
                    text(detail)
                        .text_size(px(12.0))
                        .text_color(move || theme().muted_foreground),
                ),
        )
        .child(
            text(value)
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().primary),
        )
}

// Segment order: top, top-right, bottom-right, bottom, bottom-left,
// top-left, middle.
const DIGIT_SEGMENTS: [[bool; 7]; 10] = [
    [true, true, true, true, true, true, false],
    [false, true, true, false, false, false, false],
    [true, true, false, true, true, false, true],
    [true, true, true, true, false, false, true],
    [false, true, true, false, false, true, true],
    [true, false, true, true, false, true, true],
    [true, false, true, true, true, true, true],
    [true, true, true, false, false, false, false],
    [true, true, true, true, true, true, true],
    [true, true, true, true, false, true, true],
];

const SEGMENT_BOXES: [(f32, f32, f32, f32); 7] = [
    (7.0, 2.0, 20.0, 5.0),   // top
    (27.0, 7.0, 5.0, 20.0),  // top-right
    (27.0, 31.0, 5.0, 20.0), // bottom-right
    (7.0, 53.0, 20.0, 5.0),  // bottom
    (2.0, 31.0, 5.0, 20.0),  // bottom-left
    (2.0, 7.0, 5.0, 20.0),   // top-left
    (7.0, 27.0, 20.0, 5.0),  // middle
];

/// One seven-seglyph, redrawn natively as the value changes.
pub fn segment_digit(digit: impl Fn() -> Option<u8> + Clone + 'static) -> impl IntoElement {
    let mut glyph = view().w(px(34.0)).h(px(60.0));
    for (index, (left, top, width, height)) in SEGMENT_BOXES.into_iter().enumerate() {
        let digit = digit.clone();
        glyph = glyph.child(
            view()
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(width))
                .h(px(height))
                .rounded(px(2.0))
                .bg(move || {
                    let lit = digit()
                        .and_then(|value| DIGIT_SEGMENTS.get(value as usize))
                        .is_some_and(|segments| segments[index]);
                    if lit {
                        display_on()
                    } else {
                        display_off()
                    }
                }),
        );
    }
    glyph
}

/// A stylized preview of the cooler's four-character temperature panel.
pub fn segment_display(
    temperature: impl Fn() -> f32 + Clone + 'static,
    bar: impl Fn() -> u8 + Clone + 'static,
) -> impl IntoElement {
    let for_tens = temperature.clone();
    let for_ones = temperature.clone();
    let for_tenths = temperature;
    let tens = move || {
        let value = for_tens().clamp(0.0, 99.9);
        Some(((value / 10.0) as usize % 10) as u8)
    };
    let ones = move || {
        let value = for_ones().clamp(0.0, 99.9);
        Some((value as usize % 10) as u8)
    };
    let tenths = move || {
        let value = for_tenths().clamp(0.0, 99.9);
        Some((((value * 10.0) as usize) % 10) as u8)
    };

    view()
        .flex_col()
        .gap_4()
        .p(px(22.0))
        .rounded(px(14.0))
        .bg(move || theme().color("display-bg").unwrap_or(theme().background))
        .child(
            view()
                .flex_row()
                .items_center()
                .justify_center()
                .gap_2()
                .child(segment_digit(tens))
                .child(segment_digit(ones))
                .child(
                    view()
                        .flex_col()
                        .justify_end()
                        .h(px(60.0))
                        .pb(px(6.0))
                        .child(
                            view()
                                .size(px(7.0))
                                .rounded_full()
                                .bg(move || display_on()),
                        ),
                )
                .child(segment_digit(tenths))
                .child(
                    text("C")
                        .text_size(px(20.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(move || display_on()),
                ),
        )
        .child(
            text("CPU PACKAGE TEMPERATURE")
                .text_size(px(10.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().muted_foreground),
        )
        .child(power_bar(bar))
}

/// Twenty-cell power bar, scaled to the profile's `power_bar_max_w`.
pub fn power_bar(bar: impl Fn() -> u8 + Clone + 'static) -> impl IntoElement {
    let mut row = view().flex_row().gap_1();
    for index in 0..20u8 {
        let bar = bar.clone();
        row = row.child(
            view()
                .flex_1()
                .h(px(9.0))
                .rounded(px(2.0))
                .bg(move || {
                    if bar() >= (index + 1) * 5 {
                        display_bar()
                    } else {
                        display_off()
                    }
                }),
        );
    }
    row
}
