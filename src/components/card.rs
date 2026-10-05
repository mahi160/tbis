//! Poster and wide cards for grids and rows, built on the art blocks in `art`.

use gpui_kit::component::v_flex;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::OnClick;
use super::art::{art_tile, check_badge, image, play_scrim, progress_bar};
use crate::config::HideSpoilers;
use crate::design::{color, sp, text};
use crate::jellyfin::{Api, Item, Kind};

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
        .h(sp(4.))
        .text_size(text::xs())
        .text_color(color::muted_foreground(cx))
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
    let primary = color::primary(cx);
    let name: SharedString = item.name.clone().into();
    v_flex()
        .id(SharedString::from(item.id.clone()))
        .w(width)
        .gap(sp(2.))
        .cursor_pointer()
        .on_click(on_open)
        .child(poster_art(api, item, width, None, cx))
        .child(
            div()
                .h(sp(5.))
                .text_size(text::sm())
                .truncate()
                .group_hover("card", |this| this.text_color(primary))
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
    let primary = color::primary(cx);
    let name: SharedString = item.name.clone().into();
    v_flex()
        .id(SharedString::from(item.id.clone()))
        .w(width)
        .gap(sp(2.))
        .cursor_pointer()
        .child(poster_art(api, item, width, Some(on_play), cx))
        .child(
            div()
                .id(ElementId::Name(format!("poster-title-{}", item.id).into()))
                .h(sp(5.))
                .text_size(text::sm())
                .truncate()
                .hover(|this| this.text_color(primary))
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
    let (primary, muted_fg) = (color::primary(cx), color::muted_foreground(cx));
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
    // Continue Watching/Next Up double as spoilers: the still and the episode name
    // both give away that an unwatched Episode exists/what happens in it.
    let spoiler =
        item.kind == Kind::Episode && cx.global::<HideSpoilers>().0 && !item.user_data.played;
    // server-side blur; gpui can't blur an image
    let still = api.wide_image_url(item).map(|url| {
        if spoiler {
            format!("{url}&blur=30")
        } else {
            url
        }
    });
    v_flex()
        .w(width)
        .flex_shrink_0()
        .gap(sp(2.))
        .child(
            art_tile(width, width * (9. / 16.), cx)
                .id(ElementId::Name(format!("wide-art-{}", item.id).into()))
                .cursor_pointer()
                .on_click(on_play)
                .child(image(still, cx))
                .child(play_scrim("card", cx))
                .when_some(
                    item.user_data.played_percentage.filter(|p| *p > 0.),
                    |this, p| {
                        this.child(progress_bar(
                            ElementId::Name(format!("wide-progress-{}", item.id).into()),
                            p,
                            cx,
                        ))
                    },
                ),
        )
        .child(
            v_flex()
                .id(ElementId::Name(format!("wide-title-{}", item.id).into()))
                .group("card-title")
                .gap(sp(2.))
                .cursor_pointer()
                .on_click(on_open)
                .child(
                    div()
                        .text_size(text::sm())
                        .truncate()
                        .group_hover("card-title", |this| this.text_color(primary))
                        .child(title),
                )
                .child(
                    div()
                        .text_size(text::xs())
                        .text_color(muted_fg)
                        .truncate()
                        .child(subtitle),
                ),
        )
        .into_any_element()
}
