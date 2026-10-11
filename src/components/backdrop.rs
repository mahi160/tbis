//! Dimmed hero art behind the top of a page, fading into the background.

use gpui_kit::*;

use crate::design::{color, opacity, sp};

/// Place first inside a `relative` page so content paints over it.
pub fn backdrop(url: String, cx: &App) -> AnyElement {
    let bg = color::background(cx);
    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .h(sp(115.))
        // Cover scales past the box; only this clips it
        .overflow_hidden()
        .child(
            img(url)
                .size_full()
                .object_fit(ObjectFit::Cover)
                .opacity(opacity::FAINT),
        )
        // left scrim: header text sits there
        .child(div().absolute().inset_0().bg(linear_gradient(
            90.,
            linear_color_stop(bg.opacity(0.7), 0.),
            linear_color_stop(bg.opacity(0.), 0.75),
        )))
        .child(div().absolute().inset_0().bg(linear_gradient(
            180.,
            linear_color_stop(bg.opacity(0.), 0.35),
            linear_color_stop(bg, 1.),
        )))
        .into_any_element()
}
