//! Art building blocks: the rounded tile, image with placeholder, and overlays
//! (play scrim, watched check, progress) shared by cards and episode rows.

use gpui_kit::component::progress::Progress;
use gpui_kit::component::{Icon, IconName, Sizable as _};
use gpui_kit::*;

use super::logo::logo_mark;
use crate::design::overlay::black;
use crate::design::{color, gap, icon, opacity, radius, shadow, sp};

/// Poster/thumbnail base: relative, rounded, clipped, lifts on hover.
/// `group` scopes the hover to this one card, not every card sharing the name.
pub fn art_tile(width: Pixels, height: Pixels, cx: &App) -> Div {
    div()
        .group("card")
        .relative()
        .w(width)
        .h(height)
        .rounded(radius::base())
        .overflow_hidden()
        .bg(color::muted(cx))
        .shadow(shadow::sm())
        .group_hover("card", |this| this.shadow(shadow::lg()))
}

/// Centered play button that fades in over the art while `group` is hovered
/// (Photon's play scrim).
pub fn play_scrim(group: &'static str, cx: &App) -> AnyElement {
    div()
        .absolute()
        .inset_0()
        .rounded(radius::base())
        .flex()
        .items_center()
        .justify_center()
        .bg(black(0.35))
        .opacity(0.)
        .group_hover(group, |this| this.opacity(1.))
        .child(
            div()
                .size(sp(11.))
                .rounded(radius::base())
                .flex()
                .items_center()
                .justify_center()
                .bg(color::primary(cx))
                .text_color(color::primary_foreground(cx))
                .child(Icon::new(IconName::Play)),
        )
        .into_any_element()
}

/// Round check in the top-right corner of a relative image box.
pub fn check_badge(cx: &App) -> Div {
    div()
        .absolute()
        .top(gap::item())
        .right(gap::item())
        .size(sp(6.))
        .rounded(radius::base())
        .flex()
        .items_center()
        .justify_center()
        .bg(color::primary(cx))
        .text_color(color::primary_foreground(cx))
        .child(Icon::new(IconName::Check).size(icon::sm()))
}

/// gpui-kit Progress inset from the bottom of a relative image box; `percent` is 0..100.
/// Inset, not edge-to-edge: flush it would need the box's corner radius, which
/// `overflow_hidden` can't clip to. Vega Progress: `h-1.5 rounded-full`.
pub fn progress_bar(id: impl Into<ElementId>, percent: f64, cx: &App) -> Progress {
    Progress::new(id)
        .with_size(gpui_kit::component::Size::Size(sp(
            crate::design::tokens().progress_height
        )))
        .value(percent as f32)
        .color(color::primary(cx))
        .absolute()
        .bottom(gap::item())
        .left(gap::item())
        .right(gap::item())
        .w_auto()
}

/// Image filling its parent; the tbis mark on a muted tile when missing.
/// Corner radius has to sit on the image itself: a wrapping div's
/// `overflow_hidden` clips to a plain rectangle, not to its rounded shape.
pub fn image(url: Option<String>, cx: &App) -> AnyElement {
    let (muted, muted_fg, radius) = (
        color::muted(cx),
        color::muted_foreground(cx),
        radius::base(),
    );
    let placeholder = move || {
        div()
            .size_full()
            .rounded(radius)
            .bg(muted)
            .flex()
            .items_center()
            .justify_center()
            .child(logo_mark(sp(8.)).text_color(muted_fg.opacity(opacity::HALF)))
            .into_any_element()
    };
    match url {
        Some(url) => img(url)
            .size_full()
            .rounded(radius)
            .object_fit(ObjectFit::Cover)
            .with_fallback(placeholder)
            .into_any_element(),
        None => placeholder(),
    }
}

/// Fixed-size rounded image box, e.g. a poster beside a header or a result row.
pub fn thumb(url: Option<String>, width: Pixels, height: Pixels, cx: &App) -> Div {
    div()
        .w(width)
        .h(height)
        .flex_shrink_0()
        .rounded(radius::base())
        .overflow_hidden()
        .child(image(url, cx))
}
