use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Sizable as _, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::jellyfin::{Api, Item, Kind};

const TITLE_HEIGHT: f32 = 20.;
const META_HEIGHT: f32 = 16.;

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
fn art_tile(width: Pixels, height: Pixels, cx: &App) -> Div {
    div()
        .group("card")
        .relative()
        .w(width)
        .h(height)
        .rounded(cx.theme().radius_lg)
        .overflow_hidden()
        .bg(cx.theme().muted)
        .shadow(shadow_rest())
        .group_hover("card", |this| this.shadow(shadow_hover()))
}

/// Centered play button that fades in over the art while `group` is hovered
/// (Photon's play scrim).
pub fn play_scrim(group: &'static str, cx: &App) -> AnyElement {
    div()
        .absolute()
        .inset_0()
        .rounded(cx.theme().radius_lg)
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

/// Art tile shared by `poster_card` and `poster_card_playable`; `on_play` wires the art's own
/// click (and play scrim) when the poster itself should start playback.
fn poster_art(
    api: &Api,
    item: &Item,
    width: Pixels,
    on_play: Option<OnClick>,
    cx: &App,
) -> AnyElement {
    art_tile(width, width * 1.5, cx)
        .id(ElementId::Name(format!("poster-art-{}", item.id).into()))
        .child(image(api.poster_url(item), cx))
        .when_some(on_play, |this, on_play| {
            this.on_click(on_play).child(play_scrim("card", cx))
        })
        .when(item.user_data.played, |this| this.child(check_badge(cx)))
        .into_any_element()
}

/// Year row shared by `poster_card` and `poster_card_playable`.
fn poster_meta(item: &Item, cx: &App) -> AnyElement {
    div()
        .h(px(META_HEIGHT))
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .child(
            item.production_year
                .map(|y| y.to_string())
                .unwrap_or_default(),
        )
        .into_any_element()
}

/// 2:3 poster, title, year; check badge when played. Fixed height so grid rows stay uniform.
/// The whole card opens the detail page.
pub fn poster_card(
    api: &Api,
    item: &Item,
    width: Pixels,
    on_open: OnClick,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let name: SharedString = item.name.clone().into();
    v_flex()
        .id(SharedString::from(item.id.clone()))
        .w(width)
        .gap_2()
        .cursor_pointer()
        .on_click(on_open)
        .child(poster_art(api, item, width, None, cx))
        .child(
            div()
                .h(px(TITLE_HEIGHT))
                .text_sm()
                .truncate()
                .group_hover("card", |this| this.text_color(theme.primary))
                .child(name),
        )
        .child(poster_meta(item, cx))
        .into_any_element()
}

/// As `poster_card`, but the art plays (`on_play`) and only the title opens (`on_open`) — for
/// rows where clicking the poster itself should start playback (Home's Movies/Series rows).
pub fn poster_card_playable(
    api: &Api,
    item: &Item,
    width: Pixels,
    on_play: OnClick,
    on_open: OnClick,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let name: SharedString = item.name.clone().into();
    v_flex()
        .id(SharedString::from(item.id.clone()))
        .w(width)
        .gap_2()
        .cursor_pointer()
        .child(poster_art(api, item, width, Some(on_play), cx))
        .child(
            div()
                .id(ElementId::Name(format!("poster-title-{}", item.id).into()))
                .h(px(TITLE_HEIGHT))
                .text_sm()
                .truncate()
                .hover(|this| this.text_color(theme.primary))
                .on_click(on_open)
                .child(name),
        )
        .child(poster_meta(item, cx))
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
            art_tile(width, width * (9. / 16.), cx)
                .id(ElementId::Name(format!("wide-art-{}", item.id).into()))
                .cursor_pointer()
                .on_click(on_play)
                .child(image(api.wide_image_url(item), cx))
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
/// `overflow_hidden` can't clip to. Fill: the theme's primary color, solid.
pub fn progress_bar(percent: f64, cx: &App) -> Div {
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
                .bg(cx.theme().primary),
        )
}

/// Image filling its parent; the tbis mark on a muted tile when missing.
/// Corner radius has to sit on the image itself: a wrapping div's
/// `overflow_hidden` clips to a plain rectangle, not to its rounded shape.
pub fn image(url: Option<String>, cx: &App) -> AnyElement {
    let (muted, muted_fg, radius) = (
        cx.theme().muted,
        cx.theme().muted_foreground,
        cx.theme().radius_lg,
    );
    let placeholder = move || {
        div()
            .size_full()
            .rounded(radius)
            .bg(muted)
            .flex()
            .items_center()
            .justify_center()
            .child(
                // mark's own aspect ratio (1707x1440)
                svg()
                    .path("icons/tbis-mark.svg")
                    .w(px(38.))
                    .h(px(32.))
                    .text_color(muted_fg.opacity(0.5)),
            )
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
