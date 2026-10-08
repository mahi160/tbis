//! Episode rows: the Series page's full row and a compact search result.

use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::OnClick;
use super::art;
use super::list::{ItemSize, item};
use super::tag::star_rating;
use super::typography::paragraph;
use crate::design::{color, gap, icon, sp, text};

/// What an Episode row shows; the page decides spoilers and formatting.
pub struct EpisodeRow {
    pub id: SharedString,
    pub still: Option<String>,
    pub title: String,
    pub runtime: Option<String>,
    pub rating: Option<f32>,
    pub overview: Option<String>,
    pub progress: Option<f64>,
    pub played: bool,
}

/// Still (plays, with scrim/progress/check), title, runtime · rating, overview;
/// the row opens the Episode and `trailing` sits on the right.
pub fn episode_row(
    row: EpisodeRow,
    on_open: OnClick,
    on_play: OnClick,
    trailing: impl IntoElement,
    cx: &App,
) -> AnyElement {
    let muted_fg = color::muted_foreground(cx);
    item(row.id.clone(), ItemSize::Sm, cx)
        .on_click(on_open)
        .child(
            art::art_tile("episode-still", sp(48.), sp(27.), cx)
                .id(ElementId::Name(format!("episode-play-{}", row.id).into()))
                .flex_shrink_0()
                .on_click(on_play)
                .child(art::image(row.still, cx))
                .child(art::play_scrim("episode-still", cx))
                .when_some(row.progress, |this, percent| {
                    this.child(art::progress_bar(
                        ElementId::Name(format!("episode-progress-{}", row.id).into()),
                        percent,
                        cx,
                    ))
                })
                .when(row.played, |this| this.child(art::check_badge(cx))),
        )
        .child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap(gap::tight())
                .child(div().truncate().child(row.title))
                .child(
                    h_flex()
                        .gap(gap::item())
                        .text_size(text::meta())
                        .text_color(muted_fg)
                        .children(row.runtime)
                        .children(row.rating.map(|score| star_rating(score, cx))),
                )
                .children(row.overview.map(|o| paragraph(o, 2).text_color(muted_fg))),
        )
        .child(trailing)
        .into_any_element()
}

/// Search result: 16:9 still, series name over the episode label.
pub fn episode_result(
    id: impl Into<SharedString>,
    still: Option<String>,
    series: String,
    label: String,
    on_open: OnClick,
    cx: &App,
) -> AnyElement {
    let muted_fg = color::muted_foreground(cx);
    item(id.into(), ItemSize::Sm, cx)
        .on_click(on_open)
        .child(art::thumb(still, sp(40.), sp(22.5), cx))
        .child(
            v_flex()
                .min_w_0()
                .gap(gap::tight())
                .child(
                    div()
                        .text_size(text::meta())
                        .text_color(muted_fg)
                        .truncate()
                        .child(series),
                )
                .child(div().truncate().child(label)),
        )
        .into_any_element()
}

/// Recent-search row: muted icon, the query.
pub fn recent_query(id: impl Into<ElementId>, query: SharedString, cx: &App) -> Stateful<Div> {
    use gpui_kit::component::{Icon, IconName};
    item(id, ItemSize::Xs, cx)
        .child(
            Icon::new(IconName::Search)
                .size(icon::default())
                .text_color(color::muted_foreground(cx)),
        )
        .child(query)
}
