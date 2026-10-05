//! Text styles following shadcn's typography: semibold headings (h2 `text-3xl`,
//! h3 `text-2xl`, h4 `text-xl`), `small` as `text-sm font-medium`, `muted` as
//! `text-sm text-muted-foreground`. Sizes come from `design::text`.

use gpui_kit::component::h_flex;
use gpui_kit::*;

use crate::design::color;
use crate::design::text as scale;

/// h2: oversized text (wordmark, pairing codes).
pub fn display(content: impl IntoElement) -> Div {
    div()
        .text_size(scale::x3l())
        .font_weight(FontWeight::SEMIBOLD)
        .child(content)
}

/// h3: page title (screen headings, item names).
pub fn title(content: impl IntoElement) -> Div {
    div()
        .text_size(scale::x2l())
        .font_weight(FontWeight::SEMIBOLD)
        .child(content)
}

/// h4: section heading inside a page.
pub fn heading(content: impl IntoElement) -> Div {
    div()
        .text_size(scale::xl())
        .font_weight(FontWeight::SEMIBOLD)
        .child(content)
}

/// shadcn `small`: label over a block (stats, panels, form fields).
pub fn subheading(content: impl IntoElement) -> Div {
    div()
        .text_size(scale::sm())
        .font_weight(FontWeight::MEDIUM)
        .child(content)
}

/// Stat value (shadcn card `text-2xl font-semibold`).
pub fn figure(content: impl IntoElement) -> Div {
    div()
        .text_size(scale::x2l())
        .font_weight(FontWeight::SEMIBOLD)
        .child(content)
}

/// Semibold single line, truncated.
pub fn strong(content: impl IntoElement) -> Div {
    div()
        .font_weight(FontWeight::SEMIBOLD)
        .truncate()
        .child(content)
}

/// Small muted secondary text.
pub fn caption(content: impl IntoElement, cx: &App) -> Div {
    div()
        .text_size(scale::sm())
        .text_color(color::muted_foreground(cx))
        .child(content)
}

/// Small error message in the theme's danger color.
pub fn error_text(content: impl IntoElement, cx: &App) -> Div {
    div()
        .text_size(scale::sm())
        .text_color(color::danger(cx))
        .child(content)
}

/// Body paragraph clamped to `lines`.
pub fn paragraph(content: impl Into<SharedString>, lines: usize) -> Div {
    div()
        .text_size(scale::sm())
        .line_clamp(lines)
        .text_ellipsis()
        .child(content.into())
}

/// Label and muted value on one line, split to the edges.
pub fn key_value(key: impl IntoElement, value: impl IntoElement, cx: &App) -> Div {
    h_flex()
        .justify_between()
        .text_size(scale::sm())
        .child(div().truncate().child(key))
        .child(
            div()
                .flex_shrink_0()
                .text_color(color::muted_foreground(cx))
                .child(value),
        )
}
