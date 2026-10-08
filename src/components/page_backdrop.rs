//! Dimmed full-page art behind a browse screen (Settings > Appearance > Backdrops).

use std::time::Duration;

use gpui_kit::*;

use crate::design::{backdrop, color};

const FADE: Duration = Duration::from_millis(1200);

fn fill(url: String) -> Img {
    img(url).absolute().size_full().object_fit(ObjectFit::Cover)
}

/// Full-size layer; place first inside a `relative` container. `current` fades in over
/// `previous` once per `generation`; `next` loads unseen so the next swap doesn't pop in.
pub fn page_backdrop(
    previous: Option<String>,
    current: Option<String>,
    next: Option<String>,
    generation: u64,
    cx: &App,
) -> Div {
    let bg = color::background(cx);
    let current = current.map(|url| {
        if cx.reduce_motion() {
            return fill(url).into_any_element();
        }
        fill(url)
            .with_animation(
                ElementId::NamedInteger("page-backdrop".into(), generation),
                Animation::new(FADE).with_easing(ease_in_out),
                |img, delta| img.opacity(delta),
            )
            .into_any_element()
    });
    div()
        .absolute()
        .inset_0()
        .overflow_hidden()
        .bg(bg)
        .children(next.map(|url| fill(url).opacity(0.)))
        .children(previous.map(fill))
        .children(current)
        .child(div().absolute().inset_0().bg(bg.opacity(backdrop::TINT)))
}
