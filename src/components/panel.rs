//! Card: the one radius, `bg-card` body text, `shadow-xs ring-1 ring-foreground/10`;
//! padding and gap are the group gap, or the item gap when small.

use gpui_kit::component::v_flex;
use gpui_kit::*;

use super::typography::{caption, title};
use crate::design::{color, gap, radius, shadow, sp, text};

/// Card shell with padding and gap of `spacing`.
fn card(spacing: Pixels, cx: &App) -> Div {
    v_flex()
        .p(spacing)
        .gap(spacing)
        .rounded(radius::base())
        .bg(color::surface(cx))
        .text_size(text::body())
        .border_1()
        .border_color(color::border(cx))
        .shadow(shadow::xs())
}

/// Card holding a form.
pub fn panel(cx: &App) -> Div {
    card(gap::group(), cx)
}

/// Small card: description over a large value (shadcn dashboard stat card).
pub fn stat_tile(name: &'static str, value: String, cx: &App) -> Div {
    card(gap::item(), cx)
        .flex_1()
        .min_w(sp(30.))
        .gap(gap::tight())
        .child(caption(name, cx))
        .child(title(value))
}
