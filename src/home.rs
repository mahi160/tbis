use gpui_kit::component::button::Button;
use gpui_kit::component::{IconName, h_flex, v_flex};
use gpui_kit::*;
use std::time::{Duration, Instant};

use crate::components::button::{GhostState as _, VegaSize as _, icon_button};
use crate::components::layout::{header_row, rows_page, scroll_row};
use crate::components::status::{Placeholder, Status, error_banner, full_status};
use crate::components::{OnClick, card, typography};
use crate::config::HideSpoilers;
use crate::design::{gap, page, size};
use crate::jellyfin::{Api, Item, Kind, blurred};
use crate::nav::Nav;

const PER_ROW: usize = 24;
const PAGE_ANIMATION: Duration = Duration::from_millis(350);

#[derive(Default)]
struct Rows {
    continue_watching: Vec<Item>,
    next_up: Vec<Item>,
    movies: Vec<Item>,
    series: Vec<Item>,
}

impl Rows {
    fn is_empty(&self) -> bool {
        self.continue_watching.is_empty()
            && self.next_up.is_empty()
            && self.movies.is_empty()
            && self.series.is_empty()
    }
}

/// Continue Watching, Next Up, Movies row, Series row; all Libraries.
pub struct HomeView {
    api: Api,
    rows: Rows,
    loading: bool,
    error: Option<SharedString>,
    /// Per row, same order as `render`'s rows; drives the heading arrows.
    scroll: [ScrollHandle; 4],
    /// Arrow-driven slide in progress; advanced each frame in `render`.
    paging: Option<Paging>,
    _load: Task<()>,
}

struct Paging {
    row: usize,
    from: Pixels,
    to: Pixels,
    start: Instant,
}

impl EventEmitter<Nav> for HomeView {}

impl HomeView {
    pub fn new(api: Api) -> Self {
        Self {
            api,
            rows: Rows::default(),
            loading: false,
            error: None,
            scroll: Default::default(),
            paging: None,
            _load: Task::ready(()),
        }
    }

    /// Keeps showing the current rows until fresh data arrives.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let api = self.api.clone();
        self.loading = true;
        cx.notify();
        self._load = cx.spawn(async move |this, cx| {
            // over-fetch: dedupe below drops items already in Continue Watching
            let (resume, next_up, movies, series) = futures::join!(
                api.resume(PER_ROW),
                api.next_up_all(PER_ROW * 2),
                api.latest_unplayed(Kind::Movie, PER_ROW * 2),
                api.latest_unplayed(Kind::Series, PER_ROW),
            );
            this.update(cx, |this, cx| {
                this.loading = false;
                match (resume, next_up, movies, series) {
                    (Ok(continue_watching), Ok(next_up), Ok(movies), Ok(series)) => {
                        let resumed =
                            |item: &Item| continue_watching.iter().any(|c| c.id == item.id);
                        let next_up = next_up
                            .into_iter()
                            .filter(|i| !resumed(i))
                            .take(PER_ROW)
                            .collect();
                        // Movies row: unstarted only
                        let movies = movies
                            .into_iter()
                            .filter(|i| i.user_data.playback_position_ticks == 0 && !resumed(i))
                            .take(PER_ROW)
                            .collect();
                        this.rows = Rows {
                            continue_watching,
                            next_up,
                            movies,
                            series,
                        };
                        this.error = None;
                    }
                    (Err(err), ..) | (_, Err(err), ..) | (.., Err(err), _) | (.., Err(err)) => {
                        this.error = Some(format!("Could not load Home: {err}").into());
                    }
                }
                cx.notify();
            })
            .ok();
        });
    }

    fn play_on_click(item: &Item, cx: &mut Context<Self>) -> OnClick {
        let item = item.clone();
        Box::new(cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(Nav::Play(item.clone()))))
    }

    fn open_on_click(item: &Item, cx: &mut Context<Self>) -> OnClick {
        let item = item.clone();
        Box::new(cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(Nav::Open(item.clone()))))
    }

    /// Slides row `row` about one viewport forward (`next`) or back, snapping so a card
    /// starts at the left gutter: forward brings the first cropped card there.
    fn page(&mut self, row: usize, next: bool, window: &mut Window) {
        let gutter = page();
        let scroll = &self.scroll[row];
        let viewport = scroll.bounds();
        // card lefts/rights in content coords (child bounds are unscrolled)
        let cards: Vec<(Pixels, Pixels)> = (0..)
            .map_while(|ix| scroll.bounds_for_item(ix))
            .map(|b| (b.left() - viewport.left(), b.right() - viewport.left()))
            .collect();
        let from = scroll.offset().x;
        // content x at viewport's left edge; offset.x runs 0 (start) down to -max (end)
        let view = -from;
        let slack = px(1.);
        let target = if next {
            cards
                .iter()
                .find(|(_, right)| *right > view + viewport.size.width - gutter + slack)
                .map_or(scroll.max_offset().x, |(left, _)| *left - gutter)
        } else {
            let first = cards
                .iter()
                .find(|(left, _)| *left >= view + gutter - slack)
                .map_or(px(0.), |(left, _)| *left);
            let earliest = first - (viewport.size.width - gutter * 2.);
            cards
                .iter()
                .find(|(left, _)| *left >= earliest - slack)
                .map_or(px(0.), |(left, _)| *left - gutter)
        };
        let to = -target.clamp(px(0.), scroll.max_offset().x);
        self.paging = Some(Paging {
            row,
            from,
            to,
            start: Instant::now(),
        });
        window.refresh();
    }

    fn page_button(
        id: String,
        icon: IconName,
        row: usize,
        next: bool,
        disabled: bool,
        cx: &mut Context<Self>,
    ) -> Button {
        icon_button(ElementId::Name(id.into()), icon, cx)
            .icon_xs()
            .ghost_disabled(disabled, cx)
            .on_click(cx.listener(move |this, _, window, _| this.page(row, next, window)))
    }

    /// Moves the sliding row one frame along; requests the next frame until done.
    fn advance_paging(&mut self, window: &mut Window, cx: &App) {
        let Some(paging) = &self.paging else {
            return;
        };
        let t = if cx.reduce_motion() {
            1.
        } else {
            (paging.start.elapsed().as_secs_f32() / PAGE_ANIMATION.as_secs_f32()).min(1.)
        };
        let scroll = &self.scroll[paging.row];
        let mut offset = scroll.offset();
        offset.x = paging.from + (paging.to - paging.from) * ease_out_quint()(t);
        scroll.set_offset(offset);
        if t < 1. {
            window.request_animation_frame();
        } else {
            self.paging = None;
        }
    }

    /// Row heading: bold title, plus page arrows.
    fn row(
        title: &'static str,
        id: &'static str,
        cards: Vec<AnyElement>,
        row: usize,
        scroll: ScrollHandle,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if cards.is_empty() {
            return None;
        }
        let (offset, max) = (scroll.offset().x, scroll.max_offset().x);
        // max is 0 until first layout; keep right arrow live then (click is a clamped no-op)
        let at_end = max > px(0.) && offset <= -max;
        Some(
            v_flex()
                .gap(gap::item())
                .child(
                    header_row(
                        typography::title(title),
                        h_flex()
                            .gap(gap::tight())
                            .child(Self::page_button(
                                format!("{id}-prev"),
                                IconName::ChevronLeft,
                                row,
                                false,
                                offset >= px(0.),
                                cx,
                            ))
                            .child(Self::page_button(
                                format!("{id}-next"),
                                IconName::ChevronRight,
                                row,
                                true,
                                at_end,
                                cx,
                            )),
                    )
                    .px(page()),
                )
                .child(scroll_row(id, &scroll, cards))
                .into_any_element(),
        )
    }
}

impl Render for HomeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.rows.is_empty()
            && let Some(status) = Status::pick(
                self.error.clone(),
                self.loading,
                Some("No movies or series on this server yet".into()),
            )
        {
            let retry: OnClick = Box::new(cx.listener(|this, _, _, cx| this.refresh(cx)));
            return full_status(status, Placeholder::Rows, retry, self.loading, cx);
        }

        let api = self.api.clone();
        let hide_spoilers = cx.global::<HideSpoilers>().0;
        let wide = |items: &[Item], cx: &mut Context<Self>| -> Vec<AnyElement> {
            items
                .iter()
                .map(|item| {
                    let on_play = Self::play_on_click(item, cx);
                    let on_open = Self::open_on_click(&item.detail_target(), cx);
                    // Continue Watching/Next Up stills give away unwatched Episodes
                    let spoiler =
                        hide_spoilers && item.kind == Kind::Episode && !item.user_data.played;
                    let still = api
                        .wide_image_url(item)
                        .map(|url| if spoiler { blurred(&url) } else { url });
                    card::wide_card(item, still, size::wide_w(), on_play, on_open, cx)
                })
                .collect()
        };
        let posters = |items: &[Item], cx: &mut Context<Self>| -> Vec<AnyElement> {
            items
                .iter()
                .map(|item| {
                    let on_play = Self::play_on_click(item, cx);
                    let on_open = Self::open_on_click(item, cx);
                    card::poster_card(&api, item, size::poster_w(), on_play, on_open, cx)
                })
                .collect()
        };
        let continue_watching = wide(&self.rows.continue_watching, cx);
        let next_up = wide(&self.rows.next_up, cx);
        let movies = posters(&self.rows.movies, cx);
        let series = posters(&self.rows.series, cx);

        self.advance_paging(window, cx);
        let row = |title, id, cards, ix: usize, cx: &mut Context<Self>| {
            let scroll = self.scroll[ix].clone();
            Self::row(title, id, cards, ix, scroll, cx)
        };
        // shown while a background refresh fails but the rows still have the old data
        let banner = self
            .error
            .clone()
            .map(|error| error_banner(error, cx).px(page()));
        rows_page("home")
            .children(banner)
            .children(row(
                "Continue Watching",
                "row-continue",
                continue_watching,
                0,
                cx,
            ))
            .children(row("Next Up", "row-next-up", next_up, 1, cx))
            .children(row("Movies", "row-movies", movies, 2, cx))
            .children(row("Series", "row-series", series, 3, cx))
            .into_any_element()
    }
}
