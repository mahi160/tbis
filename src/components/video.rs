//! Chrome drawn over live video: scrims, gradient bars, banners, info panel, cards.
//! Colors are the fixed `design::overlay` tints, not theme colors: they sit over
//! mpv's frame, not the app's own background.

use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::design::overlay::{self, black, white};
use crate::design::{color, radius, shadow, size, sp, text};

/// Top chrome: fades from black down to clear.
pub fn top_bar() -> Div {
    h_flex()
        .pt(sp(9.))
        .px(sp(6.))
        .pb(sp(5.))
        .gap(sp(3.))
        .bg(linear_gradient(
            180.,
            linear_color_stop(black(0.65), 0.),
            linear_color_stop(overlay::CLEAR, 1.),
        ))
}

/// Bottom chrome: fades from black up to clear; content centered at max width.
pub fn bottom_bar(content: impl IntoElement) -> Div {
    div()
        .pt(sp(14.))
        .px(sp(6.))
        .pb(sp(5.))
        .bg(linear_gradient(
            0.,
            linear_color_stop(black(0.78), 0.),
            linear_color_stop(overlay::CLEAR, 1.),
        ))
        .child(
            v_flex()
                .w_full()
                .max_w(sp(272.))
                .mx_auto()
                .child(content),
        )
}

/// Title over an optional mono subtitle.
pub fn video_title(title: SharedString, subtitle: Option<SharedString>, mono: SharedString) -> Div {
    v_flex()
        .flex_1()
        .min_w_0()
        .child(
            div()
                .text_size(text::lg())
                .font_weight(FontWeight::SEMIBOLD)
                .truncate()
                .child(title),
        )
        .children(subtitle.map(|subtitle| {
            div()
                .font_family(mono)
                .text_size(text::xs())
                .text_color(white(0.55))
                .truncate()
                .child(subtitle)
        }))
}

/// Outlined mono pill (resolution, HDR, audio format).
pub fn video_tag(label: impl Into<SharedString>, mono: SharedString) -> Div {
    div()
        .px(sp(2.))
        .rounded_full()
        .border_1()
        .border_color(white(0.35))
        .font_family(mono)
        .text_size(text::xs())
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
                .p(sp(4.))
                .rounded(radius::lg())
                .bg(black(0.6))
                .child(content),
        )
        .into_any_element()
}

/// Anchors `content` bottom-right, above the timeline.
pub fn overlay_corner(content: impl IntoElement) -> Div {
    div()
        .absolute()
        .right(sp(6.))
        .bottom(sp(28.))
        .child(content)
}

/// Dark rounded floating card in `fill`.
pub fn overlay_card(fill: Hsla) -> Div {
    v_flex()
        .p(sp(4.))
        .rounded(radius::xl())
        .bg(fill)
        .border_1()
        .border_color(white(0.08))
}

/// "Playback info" card: titled groups of label/value rows.
pub fn info_panel(groups: &[(&'static str, Vec<(&'static str, String)>)], cx: &App) -> Div {
    overlay_card(black(0.72))
        .absolute()
        .top(sp(24.))
        .left(sp(6.))
        .w(sp(70.))
        .gap(sp(4.))
        .text_size(text::xs())
        .child(
            div()
                .text_size(text::sm())
                .font_weight(FontWeight::BOLD)
                .child("Playback info"),
        )
        .children(groups.iter().map(|(heading, rows)| {
            v_flex()
                .gap(sp(1.5))
                .child(
                    div()
                        .pb(sp(1.))
                        .border_b_1()
                        .border_color(white(0.1))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(color::primary(cx))
                        .child(*heading),
                )
                .children(rows.iter().map(|(label, value)| {
                    h_flex()
                        .gap(sp(3.))
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

/// Elapsed time, `slider` with `preview` floating above it at `fraction`, total time.
pub fn timeline(
    elapsed: impl IntoElement,
    slider: impl IntoElement,
    preview: Option<(f32, AnyElement)>,
    total: impl IntoElement,
    mono: SharedString,
) -> Div {
    h_flex()
        .gap(sp(3.))
        .font_family(mono)
        .text_size(text::sm())
        .text_color(white(0.85))
        .child(elapsed)
        .child(
            div()
                .relative()
                .flex_1()
                .child(slider)
                .children(preview.map(|(fraction, preview)| {
                    div()
                        .absolute()
                        .bottom(sp(7.))
                        .left(relative(fraction))
                        .ml(px(-size::trickplay_w() / 2.))
                        .child(preview)
                })),
        )
        .child(total)
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
            .child(div().w(sp(24.)).px(sp(2.)).child(slider)),
    )
}

/// Scrub-preview tile cropped from a sprite sheet at `offset` px, already scaled.
pub fn sprite_thumb(
    url: String,
    tile: Size<f32>,
    sprite: Size<f32>,
    offset: Point<f32>,
) -> Div {
    div()
        .w(px(tile.width))
        .h(px(tile.height))
        .rounded(radius::md())
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
