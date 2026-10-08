//! Frosted detail pages: the item's art, server-blurred and tinted with the theme,
//! fills the page behind the content; content sits in translucent cards.

use gpui_kit::component::v_flex;
use gpui_kit::*;

use crate::design::{color, frost, gap, radius, sp};

/// Full-size layer; place first inside a `relative` container, with the scrolling
/// page above it so the art stays put. `blurred` fills it, `sharp` shows faintly at the top.
pub fn frost_backdrop(blurred: Option<String>, sharp: Option<String>, cx: &App) -> Div {
    let bg = color::background(cx);
    let sharp_h = sp(115.);
    let steps = frost::SHARP_STEPS;
    // stacked copies compose to SHARP_TOP at the top
    let step_opacity = 1. - (1. - frost::SHARP_TOP).powf(1. / steps as f32);
    div()
        .absolute()
        .inset_0()
        .overflow_hidden()
        .bg(bg)
        .children(blurred.map(|url| img(url).absolute().size_full().object_fit(ObjectFit::Cover)))
        // gpui has no gradient masks: each shorter copy of the sharp art drops one
        // step of opacity, so it fades out with no hard edge
        .children(sharp.into_iter().flat_map(|url| {
            (0..steps).map(move |i| {
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .h(sharp_h * ((steps - i) as f32 / steps as f32))
                    .overflow_hidden()
                    .child(
                        img(url.clone())
                            .absolute()
                            .top_0()
                            .left_0()
                            .w_full()
                            .h(sharp_h)
                            .object_fit(ObjectFit::Cover)
                            .opacity(step_opacity),
                    )
            })
        }))
        .child(div().absolute().inset_0().bg(bg.opacity(frost::PAGE_TINT)))
}

/// Translucent theme-surface card over the frosted backdrop.
pub fn frost_card(cx: &App) -> Div {
    v_flex()
        .p(gap::group())
        .gap(gap::group())
        .rounded(radius::base())
        .bg(color::surface(cx).opacity(frost::CARD))
        .border_1()
        .border_color(color::border(cx))
}
