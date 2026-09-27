use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Sizable as _, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::jellyfin::{Api, Item, Kind};

const TITLE_HEIGHT: f32 = 20.;
const META_HEIGHT: f32 = 16.;

/// Art tile corner radius (Photon's `--radius-m`, 0.875rem).
pub(crate) const RADIUS: f32 = 14.;

pub type OnClick = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// Resting elevation for art tiles (Photon's `--shadow-1`).
fn shadow_rest() -> Vec<BoxShadow> {
    vec![BoxShadow::new(px(0.), px(1.), hsla(0., 0., 0., 0.3)).blur_radius(px(2.))]
}

/// Hover elevation, the lift a tile gets under the pointer (Photon's `--shadow-2`).
fn shadow_hover() -> Vec<BoxShadow> {
    vec![
        BoxShadow::new(px(0.), px(10.), hsla(0., 0., 0., 0.55))
            .blur_radius(px(30.))
            .spread_radius(px(-10.)),
        BoxShadow::new(px(0.), px(2.), hsla(0., 0., 0., 0.15)).blur_radius(px(4.)),
    ]
}

/// Poster/thumbnail base: relative, rounded, clipped, lifts on hover.
/// `group` scopes the hover to this one card, not every card sharing the name.
fn art_tile(width: Pixels, height: Pixels, muted: Hsla) -> Div {
    div()
        .group("card")
        .relative()
        .w(width)
        .h(height)
        .rounded(px(RADIUS))
        .overflow_hidden()
        .bg(muted)
        .shadow(shadow_rest())
        .group_hover("card", |this| this.shadow(shadow_hover()))
}

/// Centered play button that fades in over the art while `group` is hovered
/// (Photon's play scrim).
pub fn play_scrim(group: &'static str, cx: &App) -> AnyElement {
    div()
        .absolute()
        .inset_0()
        .rounded(px(RADIUS))
        .flex()
        .items_center()
        .justify_center()
        .bg(hsla(0., 0., 0., 0.35))
        .opacity(0.)
        .group_hover(group, |this| this.opacity(1.))
        .child(
            div()
                .size_11()
                .rounded_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(cx.theme().primary)
                .text_color(cx.theme().primary_foreground)
                .child(Icon::new(IconName::Play)),
        )
        .into_any_element()
}

/// 2:3 poster, title, year; check badge when played. Fixed height so grid rows stay uniform.
pub fn poster_card(
    api: &Api,
    item: &Item,
    width: Pixels,
    on_click: OnClick,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let name: SharedString = item.name.clone().into();
    v_flex()
        .id(SharedString::from(item.id.clone()))
        .w(width)
        .gap_2()
        .cursor_pointer()
        .on_click(on_click)
        .child(
            art_tile(width, width * 1.5, theme.muted)
                .child(image(api.poster_url(item), name.clone(), cx))
                .when(item.user_data.played, |this| this.child(check_badge(cx))),
        )
        .child(
            div()
                .h(px(TITLE_HEIGHT))
                .text_sm()
                .truncate()
                .group_hover("card", |this| this.text_color(theme.primary))
                .child(name),
        )
        .child(
            div()
                .h(px(META_HEIGHT))
                .text_xs()
                .font_family(theme.mono_font_family.clone())
                .text_color(theme.muted_foreground)
                .child(
                    item.production_year
                        .map(|y| y.to_string())
                        .unwrap_or_default(),
                ),
        )
        .into_any_element()
}

/// 16:9 art, title, `S2E3 · Name` or year, progress bar. For Continue Watching and Next Up.
/// Art plays (`on_play`); the title/subtitle open the detail page (`on_open`).
pub fn wide_card(
    api: &Api,
    item: &Item,
    width: Pixels,
    on_play: OnClick,
    on_open: OnClick,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let (title, subtitle) = match item.kind {
        Kind::Episode => (
            item.series_name.clone().unwrap_or_default(),
            item.episode_label(),
        ),
        _ => (
            item.name.clone(),
            item.production_year
                .map(|y| y.to_string())
                .unwrap_or_default(),
        ),
    };
    v_flex()
        .w(width)
        .flex_shrink_0()
        .gap_2()
        .child(
            art_tile(width, width * (9. / 16.), theme.muted)
                .id(ElementId::Name(format!("wide-art-{}", item.id).into()))
                .cursor_pointer()
                .on_click(on_play)
                .child(image(api.wide_image_url(item), title.clone().into(), cx))
                .child(play_scrim("card", cx))
                .when_some(
                    item.user_data.played_percentage.filter(|p| *p > 0.),
                    |this, p| this.child(progress_bar(p, cx)),
                ),
        )
        .child(
            v_flex()
                .id(ElementId::Name(format!("wide-title-{}", item.id).into()))
                .group("card-title")
                .gap_2()
                .cursor_pointer()
                .on_click(on_open)
                .child(
                    div()
                        .text_sm()
                        .truncate()
                        .group_hover("card-title", |this| this.text_color(theme.primary))
                        .child(title),
                )
                .child(
                    div()
                        .text_xs()
                        .font_family(theme.mono_font_family.clone())
                        .text_color(theme.muted_foreground)
                        .truncate()
                        .child(subtitle),
                ),
        )
        .into_any_element()
}

/// Round check in the top-right corner of a relative image box.
pub fn check_badge(cx: &App) -> Div {
    div()
        .absolute()
        .top_2()
        .right_2()
        .size_6()
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(cx.theme().primary)
        .text_color(cx.theme().primary_foreground)
        .child(Icon::new(IconName::Check).small())
}

/// Pill bar inset from the bottom of a relative image box; `percent` is 0..100.
/// Inset, not edge-to-edge: flush it would need the box's corner radius, which
/// `overflow_hidden` can't clip to. Fill: accent → info beam with a soft glow.
pub fn progress_bar(percent: f64, cx: &App) -> Div {
    let theme = cx.theme();
    let beam = linear_gradient(
        90.,
        linear_color_stop(theme.primary, 0.),
        linear_color_stop(theme.info, 1.),
    );
    let glow = vec![
        BoxShadow::new(px(0.), px(0.), theme.primary.opacity(0.65)).blur_radius(px(4.)),
        BoxShadow::new(px(0.), px(0.), theme.primary.opacity(0.25)).blur_radius(px(12.)),
    ];
    div()
        .absolute()
        .bottom_2()
        .left_2()
        .right_2()
        .h(px(4.))
        .rounded_full()
        .bg(hsla(0., 0., 0., 0.55))
        .child(
            div()
                .h_full()
                .rounded_full()
                .w(relative((percent / 100.) as f32))
                .bg(beam)
                .shadow(glow),
        )
}

/// Image filling its parent; `fallback_text` on a muted tile when missing.
/// Corner radius has to sit on the image itself: a wrapping div's
/// `overflow_hidden` clips to a plain rectangle, not to its rounded shape.
pub fn image(url: Option<String>, fallback_text: SharedString, cx: &App) -> AnyElement {
    let (muted, muted_fg) = (cx.theme().muted, cx.theme().muted_foreground);
    let placeholder = move |text: SharedString| {
        div()
            .size_full()
            .rounded(px(RADIUS))
            .bg(muted)
            .flex()
            .items_center()
            .justify_center()
            .p_2()
            .text_sm()
            .text_center()
            .text_color(muted_fg)
            .child(text)
            .into_any_element()
    };
    match url {
        Some(url) => img(url)
            .size_full()
            .rounded(px(RADIUS))
            .object_fit(ObjectFit::Cover)
            .with_fallback(move || placeholder(fallback_text.clone()))
            .into_any_element(),
        None => placeholder(fallback_text),
    }
}
