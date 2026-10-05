//! The tbis mark and wordmark. Callers set the mark's color with `text_color`.

use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::*;

use super::typography::{caption, display};
use crate::design::{color, sp, text};

// mark's own aspect ratio, so it isn't squashed
const ASPECT: f32 = 1707. / 1440.;

/// Mark `height` tall, width from its aspect ratio.
pub fn logo_mark(height: Pixels) -> Svg {
    svg()
        .path("icons/tbis-mark.svg")
        .w(height * ASPECT)
        .h(height)
}

/// Compact mark + name, for the title bar.
pub fn brand(cx: &App) -> Div {
    h_flex()
        .gap(sp(1.5))
        .child(logo_mark(sp(4.)).text_color(color::primary(cx)))
        .child(
            div()
                .text_size(text::base())
                .font_weight(FontWeight::BOLD)
                .child("tbis"),
        )
}

/// Large stacked mark, name and `tagline`, for full-screen entry pages.
pub fn brand_hero(tagline: &'static str, cx: &App) -> Div {
    v_flex()
        .items_center()
        .gap(sp(2.))
        .child(logo_mark(sp(12.)).text_color(color::primary(cx)))
        .child(display("tbis"))
        .child(caption(tagline, cx))
}
