//! The tbis mark and wordmark. Callers set the mark's color with `text_color`.

use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::*;

use super::typography::{caption, display};
use crate::design::{color, gap, sp, text};

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
        .gap(gap::tight())
        // mark tracks wordmark size, not spacing
        .child(logo_mark(text::body()).text_color(color::primary(cx)))
        .child(
            div()
                .text_size(text::body())
                .font_weight(FontWeight::BOLD)
                .child("tbis"),
        )
}

/// Large stacked mark, name and `tagline`, for full-screen entry pages.
pub fn brand_hero(tagline: &'static str, cx: &App) -> Div {
    v_flex()
        .items_center()
        .gap(gap::item())
        .child(logo_mark(sp(12.)).text_color(color::primary(cx)))
        .child(display("tbis"))
        .child(caption(tagline, cx))
}
