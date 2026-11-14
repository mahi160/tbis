//! Chrome drawn over live video: scrims, gradient bars, banners, info panel, cards.
//! Colors are the fixed `design::overlay` tints, not theme colors: they sit over
//! mpv's frame, not the app's own background.

use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::base::ElementExt as _;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::design::overlay::{self, black, white};
use crate::design::{gap, radius, shadow, size, sp, text};

/// Bare layer filling the video, for mouse input on the video itself. Chrome built
/// here blocks the mouse, so the layer only sees clicks that reach the picture.
pub fn video_surface(id: &'static str) -> Stateful<Div> {
    div().id(id).absolute().inset_0()
}

/// Top chrome: fades from black down to clear.
pub fn top_bar() -> Div {
    h_flex()
        .block_mouse_except_scroll()
        // fixed: clears the traffic lights
        .pt(sp(9.))
        .px(gap::group())
        .pb(gap::group())
        .gap(gap::item())
        .bg(linear_gradient(
            180.,
            linear_color_stop(black(0.65), 0.),
            linear_color_stop(overlay::CLEAR, 1.),
        ))
}

/// Bottom chrome: fades from black up to clear; content centered at max width.
pub fn bottom_bar(content: impl IntoElement) -> Div {
    div()
        .block_mouse_except_scroll()
        // fixed: room for the gradient to fade
        .pt(sp(14.))
        .px(gap::group())
        .pb(gap::group())
        .bg(linear_gradient(
            0.,
            linear_color_stop(black(0.78), 0.),
            linear_color_stop(overlay::CLEAR, 1.),
        ))
        .child(v_flex().w_full().max_w(sp(272.)).mx_auto().child(content))
}

/// Title over an optional mono subtitle.
pub fn video_title(title: SharedString, subtitle: Option<SharedString>, mono: SharedString) -> Div {
    v_flex()
        .flex_1()
        .min_w_0()
        .child(
            div()
                .text_size(text::body())
                .font_weight(FontWeight::SEMIBOLD)
                .truncate()
                .child(title),
        )
        .children(subtitle.map(|subtitle| {
            div()
                .font_family(mono)
                .text_size(text::meta())
                .text_color(white(0.55))
                .truncate()
                .child(subtitle)
        }))
}

/// Outlined mono pill (resolution, HDR, audio format).
pub fn video_tag(label: impl Into<SharedString>, mono: SharedString) -> Div {
    div()
        .px(gap::item())
        .rounded(radius::base())
        .border_1()
        .border_color(white(0.35))
        .font_family(mono)
        .text_size(text::meta())
        .text_color(overlay::TEXT_MUTED)
        .child(label.into())
}

/// Mono text in an overlay tint, e.g. the wall clock or "ends at".
pub fn video_meta(content: impl IntoElement, mono: SharedString, color: Hsla) -> Div {
    div()
        .font_family(mono)
        .text_color(color)
        .whitespace_nowrap()
        .child(content)
}

/// Content centered on the video in a dark scrim box.
pub fn overlay_banner(content: impl IntoElement) -> AnyElement {
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .p(gap::group())
                .rounded(radius::base())
                .bg(black(0.6))
                .child(content),
        )
        .into_any_element()
}

/// Spinner centered on the video, for buffering.
pub fn overlay_spinner() -> AnyElement {
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .child(Spinner::new().large().color(overlay::WHITE))
        .into_any_element()
}

/// Picture-in-Picture window chrome: `buttons` centered in a scrim and `seek_bar`
/// along the bottom, shown on hover. Dragging anywhere else moves the window.
pub fn pip_frame(
    buttons: impl IntoIterator<Item = impl IntoElement>,
    seek_bar: impl IntoElement,
) -> Stateful<Div> {
    div()
        .id("pip")
        .group("pip")
        .relative()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
        .child(
            h_flex()
                .gap(gap::tight())
                .p(gap::tight())
                .rounded(radius::base())
                .bg(black(0.6))
                .opacity(0.)
                .group_hover("pip", |s| s.opacity(1.))
                // clicks on the buttons must not start a window drag
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .children(buttons),
        )
        .child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .bottom_0()
                .p(gap::item())
                .bg(linear_gradient(
                    0.,
                    linear_color_stop(black(0.6), 0.),
                    linear_color_stop(overlay::CLEAR, 1.),
                ))
                .opacity(0.)
                .group_hover("pip", |s| s.opacity(1.))
                // scrubbing must not start a window drag
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(seek_bar),
        )
}

/// Anchors `content` bottom-right, above the timeline.
pub fn overlay_corner(content: impl IntoElement) -> Div {
    div()
        .block_mouse_except_scroll()
        .absolute()
        .right(gap::group())
        // fixed: clears the bottom bar
        .bottom(sp(28.))
        .child(content)
}

/// Dark rounded floating card in `fill`.
pub fn overlay_card(fill: Hsla) -> Div {
    v_flex()
        .block_mouse_except_scroll()
        .p(gap::group())
        .rounded(radius::base())
        .text_color(overlay::WHITE)
        .bg(fill)
        .border_1()
        .border_color(white(0.08))
}

/// One titled group of label/value rows in the `info_panel`.
pub type InfoGroup = (&'static str, Vec<(&'static str, String)>);

/// "Playback info" card: titled groups of label/value rows.
/// `close` sits beside the heading; the panel stays up until closed, not with the controls.
pub fn info_panel(groups: &[InfoGroup], close: impl IntoElement) -> Div {
    overlay_card(black(0.72))
        .absolute()
        // fixed: clears the top bar
        .top(sp(24.))
        .left(gap::group())
        .w(sp(70.))
        .gap(gap::group())
        .text_size(text::meta())
        .child(
            h_flex()
                .justify_between()
                .child(
                    div()
                        .text_size(text::body())
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Playback info"),
                )
                .child(close),
        )
        .children(groups.iter().map(|(heading, rows)| {
            v_flex()
                .gap(gap::tight())
                .child(
                    div()
                        .pb(gap::tight())
                        .border_b_1()
                        .border_color(white(0.1))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(overlay::WHITE)
                        .child(*heading),
                )
                .children(rows.iter().map(|(label, value)| {
                    h_flex()
                        .gap(gap::item())
                        .justify_between()
                        .child(div().flex_none().text_color(white(0.6)).child(*label))
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .font_weight(FontWeight::MEDIUM)
                                .child(value.clone()),
                        )
                }))
        }))
}

/// Elapsed time, `slider` over a `buffered` bar (fraction of the duration) with
/// `preview` floating above it at `fraction`, total time. `on_hover` gets the
/// pointer's fraction along the slider, and `None` when it leaves.
pub fn timeline(
    elapsed: impl IntoElement,
    slider: impl IntoElement,
    buffered: f32,
    preview: Option<(f32, AnyElement)>,
    total: impl IntoElement,
    mono: SharedString,
    on_hover: impl Fn(&Option<f32>, &mut Window, &mut App) + 'static,
) -> Div {
    // the slider fills this box, so its bounds map the pointer the way the slider does
    let bounds = Rc::new(Cell::new(Bounds::<Pixels>::default()));
    let on_hover = Rc::new(on_hover);
    h_flex()
        .gap(gap::item())
        .font_family(mono)
        .text_size(text::body())
        .text_color(white(0.85))
        .child(elapsed)
        .child(
            div()
                .id("timeline-track")
                .relative()
                .flex_1()
                .on_prepaint({
                    let bounds = bounds.clone();
                    move |b, _, _| bounds.set(b)
                })
                .on_mouse_move({
                    let on_hover = on_hover.clone();
                    move |event, window, cx| {
                        let b = bounds.get();
                        if b.size.width > px(0.) {
                            let fraction = (event.position.x - b.left()) / b.size.width;
                            on_hover(&Some(fraction.clamp(0., 1.)), window, cx);
                        }
                    }
                })
                .on_hover(move |hovered, window, cx| {
                    if !*hovered {
                        on_hover(&None, window, cx);
                    }
                })
                // painted first so it sits under the slider's translucent track, which centres it too
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .top(relative(0.5))
                        .h(px(1.))
                        .w(relative(buffered.clamp(0., 1.)))
                        .bg(white(0.35)),
                )
                .child(slider)
                .children(preview.map(|(fraction, preview)| {
                    div()
                        .absolute()
                        .bottom(sp(7.))
                        .left(relative(fraction))
                        .ml(-size::trickplay_w() / 2.)
                        .child(preview)
                })),
        )
        .child(total)
}

/// Scrub preview: the trickplay `thumb`, when there is one, over the `time` it shows.
pub fn scrub_preview(thumb: Option<AnyElement>, time: impl IntoElement) -> Div {
    v_flex()
        // as wide as a thumb, so `timeline` centres it on the pointer either way
        .w(size::trickplay_w())
        .items_center()
        .gap(gap::tight())
        .children(thumb)
        .child(
            div()
                .px(gap::tight())
                .rounded(radius::base())
                .bg(black(0.6))
                .text_size(text::meta())
                .text_color(overlay::WHITE)
                .child(time),
        )
}

/// `button` with `slider` unfolding beside it on hover, or while `open`.
pub fn hover_reveal(
    group: &'static str,
    button: impl IntoElement,
    slider: impl IntoElement,
    open: bool,
) -> Div {
    h_flex().group(group).child(button).child(
        div()
            .w_0()
            .overflow_hidden()
            .group_hover(group, |s| s.w(sp(24.)))
            .when(open, |s| s.w(sp(24.)))
            .child(div().w(sp(24.)).px(gap::item()).child(slider)),
    )
}

/// Scrub-preview tile cropped from a sprite sheet at `offset` px, already scaled.
pub fn sprite_thumb(url: String, tile: Size<f32>, sprite: Size<f32>, offset: Point<f32>) -> Div {
    div()
        .w(px(tile.width))
        .h(px(tile.height))
        .rounded(radius::base())
        .overflow_hidden()
        .relative()
        .shadow(shadow::lg())
        .child(
            img(url)
                .absolute()
                .top(px(-offset.y))
                .left(px(-offset.x))
                .w(px(sprite.width))
                .h(px(sprite.height)),
        )
}
