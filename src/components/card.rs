//! Poster and wide cards for grids and rows, built on the art blocks in `art`.

use gpui_kit::component::v_flex;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::OnClick;
use super::art::{art_tile, check_badge, image, play_scrim, progress_bar};
use crate::design::{color, gap, text};
use crate::jellyfin::{Api, Item, Kind};

/// `poster_card` art; `on_play` wires the art's own
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

/// `poster_card` year row.
fn poster_meta(item: &Item, cx: &App) -> AnyElement {
    div()
        .h(text::line(text::meta()))
        .line_height(text::line(text::meta()))
        .text_size(text::meta())
        .text_color(color::muted_foreground(cx))
        .child(
            item.production_year
                .map(|y| y.to_string())
                .unwrap_or_default(),
        )
        .into_any_element()
}

/// Title over its muted meta line, set tight.
fn caption_stack(title: impl IntoElement, meta: impl IntoElement) -> Div {
    v_flex().gap(gap::tight()).child(title).child(meta)
}

/// 2:3 poster, title, year; check badge when played. Fixed height so grid rows stay uniform.
/// With `on_play` the art plays and only the title opens (Home's Movies/Series rows);
/// without it the whole card opens (`on_open`).
pub fn poster_card(
    api: &Api,
    item: &Item,
    width: Pixels,
    on_play: Option<OnClick>,
    on_open: OnClick,
    cx: &App,
) -> AnyElement {
    let primary = color::primary(cx);
    let name: SharedString = item.name.clone().into();
    let (title_click, card_click) = match on_play {
        Some(_) => (Some(on_open), None),
        None => (None, Some(on_open)),
    };
    let title = div()
        .id(ElementId::Name(format!("poster-title-{}", item.id).into()))
        .h(text::line(text::body()))
        .line_height(text::line(text::body()))
        .text_size(text::body())
        .truncate()
        .child(name);
    let title = match title_click {
        Some(on_open) => title
            .hover(|this| this.text_color(primary))
            .on_click(on_open),
        None => title.group_hover("card", |this| this.text_color(primary)),
    };
    v_flex()
        .id(SharedString::from(item.id.clone()))
        .w(width)
        .gap(gap::item())
        .cursor_pointer()
        .when_some(card_click, |this, on_open| this.on_click(on_open))
        .child(poster_art(api, item, width, on_play, cx))
        .child(caption_stack(title, poster_meta(item, cx)))
        .into_any_element()
}

/// 16:9 art, title, `S2E3 · Name` or year, progress bar. For Continue Watching and Next Up.
/// Art plays (`on_play`); the title/subtitle open the detail page (`on_open`).
pub fn wide_card(
    item: &Item,
    still: Option<String>,
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
    v_flex()
        .w(width)
        .flex_shrink_0()
        .gap(gap::item())
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
                .gap(gap::tight())
                .cursor_pointer()
                .on_click(on_open)
                .child(
                    div()
                        .text_size(text::body())
                        .line_height(text::line(text::body()))
                        .truncate()
                        .group_hover("card-title", |this| this.text_color(primary))
                        .child(title),
                )
                .child(
                    div()
                        .text_size(text::meta())
                        .line_height(text::line(text::meta()))
                        .text_color(muted_fg)
                        .truncate()
                        .child(subtitle),
                ),
        )
        .into_any_element()
}
