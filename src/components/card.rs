//! Poster and wide cards for grids and rows, built on the art blocks in `art`.

use gpui_kit::component::v_flex;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::OnClick;
use super::ambient::follow_hover;
use super::art::{art_tile, check_badge, image, play_scrim, progress_bar};
use crate::design::{color, gap, text};
use crate::jellyfin::{Api, Item, Kind};

/// `poster_card` art; only the hover play button (`on_play`) plays.
fn poster_art(api: &Api, item: &Item, width: Pixels, on_play: OnClick, cx: &App) -> AnyElement {
    art_tile("poster-art", width, width * 1.5, cx)
        .id(ElementId::Name(format!("poster-art-{}", item.id).into()))
        .child(image(api.poster_url(item), cx))
        .child(play_scrim("poster-art", Some(on_play)))
        .when(item.user_data.played, |this| this.child(check_badge()))
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
/// The whole card opens (`on_open`); only the hover play button plays (`on_play`).
pub fn poster_card(
    api: &Api,
    item: &Item,
    width: Pixels,
    on_play: OnClick,
    on_open: OnClick,
    cx: &App,
) -> AnyElement {
    let primary = color::primary(cx);
    let name: SharedString = item.name.clone().into();
    let title = div()
        .id(ElementId::Name(format!("poster-title-{}", item.id).into()))
        .h(text::line(text::body()))
        .line_height(text::line(text::body()))
        .text_size(text::body())
        .truncate()
        .group_hover("poster", |this| this.text_color(primary))
        .child(name);
    v_flex()
        .id(SharedString::from(item.id.clone()))
        .w(width)
        .gap(gap::item())
        .cursor_pointer()
        .group("poster")
        .on_click(on_open)
        .on_hover(follow_hover(api.frost_url(item)))
        .child(poster_art(api, item, width, on_play, cx))
        .child(caption_stack(title, poster_meta(item, cx)))
        .into_any_element()
}

/// 16:9 art, title, `S2E3 · Name` or year, progress bar. For Continue Watching and Next Up.
/// Art plays (`on_play`); the title/subtitle open the detail page (`on_open`).
pub fn wide_card(
    item: &Item,
    still: Option<String>,
    frost: Option<String>,
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
        .id(ElementId::Name(format!("wide-card-{}", item.id).into()))
        .w(width)
        .flex_shrink_0()
        .gap(gap::item())
        .on_hover(follow_hover(frost))
        .child(
            art_tile("wide-art", width, width * (9. / 16.), cx)
                .id(ElementId::Name(format!("wide-art-{}", item.id).into()))
                .cursor_pointer()
                .on_click(on_play)
                .child(image(still, cx))
                .child(play_scrim("wide-art", None))
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
