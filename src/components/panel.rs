//! Card: `rounded-xl bg-card text-sm shadow-xs ring-1 ring-foreground/10`, padding and
//! gap from the theme's `design.card` recipe (vega: 6 default, 4 small).

use gpui_kit::component::v_flex;
use gpui_kit::*;

use super::typography::{caption, figure};
use crate::design::{color, radius, shadow, sp, text, tokens};

/// Card shell with padding and gap of `spacing` units.
fn card(spacing: f32, cx: &App) -> Div {
    v_flex()
        .p(sp(spacing))
        .gap(sp(spacing))
        .rounded(radius::xl())
        .bg(color::popover(cx))
        .text_size(text::sm())
        .border_1()
        .border_color(color::foreground(cx).opacity(0.1))
        .shadow(shadow::xs())
}

/// Card holding a form.
pub fn panel(cx: &App) -> Div {
    card(tokens().card.spacing, cx)
}

/// Small card: description over a large value (shadcn dashboard stat card).
pub fn stat_tile(name: &'static str, value: String, cx: &App) -> Div {
    card(tokens().card.spacing_sm, cx)
        .flex_1()
        .min_w(sp(30.))
        .gap(sp(1.))
        .child(caption(name, cx))
        .child(figure(value))
}
