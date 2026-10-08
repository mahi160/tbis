//! Art building blocks: the rounded tile, image with placeholder, and overlays
//! (play scrim, watched check, progress) shared by cards and episode rows.

use std::time::{Duration, Instant};

use gpui_kit::component::progress::Progress;
use gpui_kit::component::{Icon, IconName, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::OnClick;
use super::logo::logo_mark;
use crate::design::overlay::{self, black};
use crate::design::{color, gap, icon, opacity, radius, shadow, sp};

/// Poster/thumbnail base: relative, rounded, clipped, lifts while `group` is hovered.
/// gpui resolves `group` to the nearest ancestor of that name, so each tile hovers alone;
/// pass the same name to its `play_scrim`.
pub fn art_tile(group: &'static str, width: Pixels, height: Pixels, cx: &App) -> Div {
    div()
        .group(group)
        .relative()
        .w(width)
        .h(height)
        .rounded(radius::base())
        .overflow_hidden()
        .bg(color::muted(cx))
        .shadow(shadow::sm())
        .group_hover(group, |this| this.shadow(shadow::lg()))
}

const FADE_IN: Duration = Duration::from_millis(150);
const FADE_OUT: Duration = Duration::from_millis(120);

/// Per-tile scrim fade. `generation` 0 means never hovered: static, no animation.
struct Fade {
    generation: u64,
    visible: bool,
    from: f32,
    since: Instant,
}

impl Fade {
    /// Opacity right now, mid-fade included, so a reversal starts where the eye is.
    fn current(&self) -> f32 {
        let (target, dur) = if self.visible {
            (1., FADE_IN)
        } else {
            (0., FADE_OUT)
        };
        let t = (self.since.elapsed().as_secs_f32() / dur.as_secs_f32()).min(1.);
        self.from + (target - self.from) * ease_out_quint()(t)
    }
}

/// Centered play button over the art that fades in on hover and out on leave
/// (Photon's play scrim). gpui has no hover transitions, so each hover change
/// restarts an animation under a new element id. With `on_play` the button
/// itself is clickable and swallows the click so a parent's click (open) doesn't
/// also fire. `group` only namespaces element ids.
#[derive(IntoElement)]
pub struct PlayScrim {
    group: &'static str,
    on_play: Option<OnClick>,
}

pub fn play_scrim(group: &'static str, on_play: Option<OnClick>) -> PlayScrim {
    PlayScrim { group, on_play }
}

impl RenderOnce for PlayScrim {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let group = self.group;
        let fade = window.use_keyed_state(
            ElementId::Name(format!("{group}-fade").into()),
            cx,
            |_, _| Fade {
                generation: 0,
                visible: false,
                from: 0.,
                since: Instant::now(),
            },
        );
        let (generation, visible, from) = {
            let f = fade.read(cx);
            (f.generation, f.visible, f.from)
        };
        let target = if visible { 1. } else { 0. };

        let scrim = div()
            .id(ElementId::Name(format!("{group}-scrim").into()))
            .absolute()
            .inset_0()
            .rounded(radius::base())
            .flex()
            .items_center()
            .justify_center()
            .bg(black(0.35))
            .on_hover(move |hovered, _, cx| {
                fade.update(cx, |f, cx| {
                    if f.visible != *hovered {
                        (f.from, f.since) = (f.current(), Instant::now());
                        f.visible = *hovered;
                        f.generation += 1;
                        cx.notify();
                    }
                })
            })
            .child(
                div()
                    .id(ElementId::Name(format!("{group}-play").into()))
                    .size(sp(11.))
                    .rounded(radius::base())
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(color::primary(cx))
                    .text_color(color::primary_foreground(cx))
                    .when_some(self.on_play, |this, on_play| {
                        this.cursor_pointer().on_click(move |event, window, cx| {
                            cx.stop_propagation();
                            on_play(event, window, cx)
                        })
                    })
                    .child(Icon::new(IconName::Play)),
            );

        if generation == 0 {
            return scrim.opacity(0.).into_any_element();
        }
        let animation =
            Animation::new(if visible { FADE_IN } else { FADE_OUT }).with_easing(ease_out_quint());
        scrim
            .with_animation(
                ElementId::NamedInteger(format!("{group}-fade").into(), generation),
                animation,
                move |this, delta| this.opacity(from + (target - from) * delta),
            )
            .into_any_element()
    }
}

/// Dark tinted check in the top-right corner of a relative image box.
pub fn check_badge() -> Div {
    div()
        .absolute()
        .top(gap::item())
        .right(gap::item())
        .size(sp(6.))
        .rounded(radius::base())
        .flex()
        .items_center()
        .justify_center()
        .bg(black(0.6))
        .text_color(overlay::WHITE)
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
