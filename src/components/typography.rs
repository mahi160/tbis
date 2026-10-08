//! Text styles on the three sizes: titles are semibold `title`, everything else is
//! `body`, secondary details are `meta`. Hierarchy beyond that comes from weight and
//! muted color, not more sizes.

use gpui_kit::component::h_flex;
use gpui_kit::*;

use crate::design::{color, text};

/// Title: screen and section headings, item names, wordmark, pairing codes, stat values.
pub fn title(content: impl IntoElement) -> Div {
    div()
        .text_size(text::title())
        .font_weight(FontWeight::SEMIBOLD)
        .child(content)
}

/// shadcn `small`: label over a block (stats, panels, form fields).
pub fn subheading(content: impl IntoElement) -> Div {
    div()
        .text_size(text::body())
        .font_weight(FontWeight::MEDIUM)
        .child(content)
}

/// Semibold single line, truncated.
pub fn strong(content: impl IntoElement) -> Div {
    div()
        .font_weight(FontWeight::SEMIBOLD)
        .truncate()
        .child(content)
}

/// Muted secondary text.
pub fn caption(content: impl IntoElement, cx: &App) -> Div {
    div()
        .text_size(text::meta())
        .text_color(color::muted_foreground(cx))
        .child(content)
}

/// Error message in the theme's danger color.
pub fn error_text(content: impl IntoElement, cx: &App) -> Div {
    div()
        .text_size(text::body())
        .text_color(color::danger(cx))
        .child(content)
}

/// Body paragraph clamped to `lines`.
pub fn paragraph(content: impl Into<SharedString>, lines: usize) -> Div {
    div()
        .text_size(text::body())
        .line_clamp(lines)
        .text_ellipsis()
        .child(content.into())
}

/// Label and muted value on one line, split to the edges.
pub fn key_value(key: impl IntoElement, value: impl IntoElement, cx: &App) -> Div {
    h_flex()
        .justify_between()
        .text_size(text::body())
        .child(div().truncate().child(key))
        .child(
            div()
                .flex_shrink_0()
                .text_color(color::muted_foreground(cx))
                .child(value),
        )
}
