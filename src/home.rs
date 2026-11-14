use gpui_kit::component::IconName;
use gpui_kit::component::button::Button;
use gpui_kit::*;
use std::time::{Duration, Instant};

use crate::components::button::{GhostState as _, VegaSize as _, icon_button};
use crate::components::detail::tag_labels;
use crate::components::hero::{Hero, HeroCarousel, Peek};
use crate::components::image_cache::lru_images;
use crate::components::layout::rows_page;
use crate::components::row::paged_row;
use crate::components::status::{Placeholder, Status, error_banner, full_status};
use crate::components::{OnClick, card, typography};
use crate::config::{HideSpoilers, HomeLayout};
use crate::design::{page, size};
use crate::detail::runtime_label;
use crate::jellyfin::{Api, Item, Kind, Library, blurred};
use crate::nav::Nav;

const PER_ROW: usize = 24;
// four rows of up to 24 cards, with room to scroll back
const IMAGES_KEPT: usize = 120;
const PAGE_ANIMATION: Duration = Duration::from_millis(350);
/// Films in the Modern hero carousel.
const HERO_SLIDES: usize = 5;
/// How long a hero slide shows before the next, unless the pointer is on it.
const HERO_DWELL: Duration = Duration::from_secs(8);

#[derive(Default)]
struct Rows {
    continue_watching: Vec<Item>,
    next_up: Vec<Item>,
    movies: Vec<Item>,
    series: Vec<Item>,
    /// Modern layout only: each Movie and Series Library with its newest items.
    libraries: Vec<(Library, Vec<Item>)>,
    /// Modern layout only: the hero's films, in full.
    featured: Vec<Item>,
}

impl Rows {
    /// Whether `layout` has nothing to show; rows loaded for the other layout don't count.
    fn is_empty(&self, layout: HomeLayout) -> bool {
        self.continue_watching.is_empty()
            && self.next_up.is_empty()
            && match layout {
                HomeLayout::Default => self.movies.is_empty() && self.series.is_empty(),
                HomeLayout::Modern => {
                    self.featured.is_empty()
                        && self.libraries.iter().all(|(_, items)| items.is_empty())
                }
            }
    }
}

/// Which hero slide shows, and the one fading out under it.
#[derive(Default)]
struct Carousel {
    active: usize,
    previous: Option<usize>,
    /// Keys the fade; bumped per slide change.
    generation: usize,
    /// Keys the active dot's fill; bumped per countdown restart.
    cycle: usize,
    /// Pointer on the hero; the countdown waits.
    hovered: bool,
}

/// Default: Continue Watching, Next Up, Movies row, Series row, all Libraries merged.
/// Modern: hero carousel of the newest unwatched films, Continue Watching, Next Up,
/// then one row per Library.
pub struct HomeView {
    api: Api,
    layout: HomeLayout,
    rows: Rows,
    loading: bool,
    error: Option<SharedString>,
    /// Per row, same order as `render`'s rows, keyed by row id so a row keeps its
    /// scroll when rows come or go; drives the heading arrows.
    scroll: Vec<(SharedString, ScrollHandle)>,
    /// Arrow-driven slide in progress; advanced each frame in `render`.
    paging: Option<Paging>,
    /// When the rows last loaded; `None` means stale.
    loaded_at: Option<Instant>,
    hero: Carousel,
    _load: Task<()>,
    /// Hero auto-advance; replaced to restart the wait after a dot click.
    _advance: Task<()>,
}

struct Paging {
    row: usize,
    from: Pixels,
    to: Pixels,
    start: Instant,
}

impl EventEmitter<Nav> for HomeView {}

impl HomeView {
    pub fn new(api: Api, layout: HomeLayout) -> Self {
        Self {
            api,
            layout,
            rows: Rows::default(),
            loading: false,
            error: None,
            scroll: Default::default(),
            paging: None,
            loaded_at: None,
            hero: Carousel::default(),
            _load: Task::ready(()),
            _advance: Task::ready(()),
        }
    }

    /// Fades to hero slide `ix`.
    fn show_slide(&mut self, ix: usize, cx: &mut Context<Self>) {
        if ix == self.hero.active || ix >= self.rows.featured.len() {
            return;
        }
        self.hero.previous = Some(self.hero.active);
        self.hero.active = ix;
        self.hero.generation += 1;
        cx.notify();
    }

    /// Restarts the wait before the next hero slide; each advance starts the next wait.
    fn countdown(&mut self, cx: &mut Context<Self>) {
        self.hero.cycle += 1;
        cx.notify();
        self._advance = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(HERO_DWELL).await;
            this.update(cx, |this, cx| {
                let count = this.rows.featured.len();
                if count > 1 && !this.hero.hovered {
                    this.show_slide((this.hero.active + 1) % count, cx);
                    this.countdown(cx);
                }
            })
            .ok();
        });
    }

    /// Pointer on the hero pauses the countdown; leaving restarts it in full.
    fn hero_hovered(&mut self, hovered: bool, cx: &mut Context<Self>) {
        self.hero.hovered = hovered;
        if hovered {
            self._advance = Task::ready(());
            cx.notify();
        } else {
            self.countdown(cx);
        }
    }

    /// Marks the data out of date, so the next tab visit reloads it.
    pub fn mark_stale(&mut self) {
        self.loaded_at = None;
        // a load already in flight may predate the change; drop it so it can't mark itself fresh
        self._load = Task::ready(());
        self.loading = false;
    }

    /// Switches layout; Modern's Library rows load on the next visit.
    pub fn set_layout(&mut self, layout: HomeLayout, cx: &mut Context<Self>) {
        self.layout = layout;
        self.mark_stale();
        cx.notify();
    }

    /// Reloads unless the data is under a minute old.
    pub fn refresh_if_stale(&mut self, cx: &mut Context<Self>) {
        const FRESH_FOR: Duration = Duration::from_secs(60);
        if self.loaded_at.is_some_and(|at| at.elapsed() < FRESH_FOR) {
            return;
        }
        self.refresh(cx);
    }

    /// Keeps showing the current rows until fresh data arrives.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let api = self.api.clone();
        let modern = self.layout == HomeLayout::Modern;
        self.loading = true;
        cx.notify();
        self._load = cx.spawn(async move |this, cx| {
            // over-fetch: dedupe below drops items already in Continue Watching
            let (resume, next_up, movies, series, libraries, featured) = cx
                .background_spawn(async move {
                    // each layout fetches only the rows it shows
                    let (resume, next_up, movies, series, libraries) = futures::join!(
                        api.resume(PER_ROW),
                        api.next_up_all(PER_ROW * 2),
                        // Default's Movies row; Modern's hero picks from the same films
                        api.latest_unplayed(
                            Kind::Movie,
                            if modern { HERO_SLIDES * 2 } else { PER_ROW * 2 }
                        ),
                        async {
                            if modern {
                                Ok(Vec::new())
                            } else {
                                api.latest_unplayed(Kind::Series, PER_ROW).await
                            }
                        },
                        async {
                            if modern {
                                library_rows(&api).await
                            } else {
                                Ok(Vec::new())
                            }
                        },
                    );
                    let featured = match &movies {
                        Ok(movies) if modern => featured(&api, movies).await,
                        _ => Vec::new(),
                    };
                    (resume, next_up, movies, series, libraries, featured)
                })
                .await;
            this.update(cx, |this, cx| {
                this.loading = false;
                match (resume, next_up, movies, series, libraries) {
                    (Ok(continue_watching), Ok(next_up), Ok(movies), Ok(series), Ok(libraries)) => {
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
                            libraries,
                            featured,
                        };
                        if this.hero.active >= this.rows.featured.len() {
                            this.hero.active = 0;
                        }
                        this.hero.previous = None;
                        this.countdown(cx);
                        this.error = None;
                        this.loaded_at = Some(Instant::now());
                    }
                    (Err(err), ..)
                    | (_, Err(err), ..)
                    | (_, _, Err(err), ..)
                    | (.., Err(err), _)
                    | (.., Err(err)) => {
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
        let scroll = &self.scroll[row].1;
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
        let tooltip = if next { "Next" } else { "Previous" };
        icon_button(ElementId::Name(id.into()), icon, tooltip, cx)
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
        let scroll = &self.scroll[paging.row].1;
        let mut offset = scroll.offset();
        offset.x = paging.from + (paging.to - paging.from) * ease_out_quint()(t);
        scroll.set_offset(offset);
        if t < 1. {
            window.request_animation_frame();
        } else {
            self.paging = None;
        }
    }

    /// The Modern hero: the active film's slide over the one fading out, with dots.
    fn hero_carousel(&self, cx: &mut Context<Self>) -> Option<HeroCarousel> {
        let films = &self.rows.featured;
        let slide = |film: &Item, cx: &mut Context<Self>| Hero {
            backdrop: self.api.backdrop_url(film),
            logo: self.api.logo_url(film),
            title: film.name.clone().into(),
            facts: [
                film.production_year.map(|year| year.to_string()),
                film.run_time_ticks.map(runtime_label),
            ]
            .into_iter()
            .flatten()
            .map(Into::into)
            .collect(),
            rating: film.community_rating,
            tags: tag_labels(film).into_iter().map(Into::into).collect(),
            overview: film.overview.clone().map(Into::into),
            on_play: Self::play_on_click(film, cx),
            on_more: Self::open_on_click(film, cx),
        };
        let active = films.get(self.hero.active)?;
        let previous = self
            .hero
            .previous
            .and_then(|ix| films.get(ix))
            .map(|film| slide(film, cx));
        let dots = films
            .iter()
            .enumerate()
            .map(|(ix, film)| -> (SharedString, OnClick) {
                let on_click = cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.show_slide(ix, cx);
                    this.countdown(cx);
                });
                (film.name.clone().into(), Box::new(on_click))
            })
            .collect();
        let count = films.len();
        let peek = |ix: usize, cx: &mut Context<Self>| Peek {
            backdrop: self.api.backdrop_url(&films[ix]),
            title: films[ix].name.clone().into(),
            on_click: Box::new(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.show_slide(ix, cx);
                this.countdown(cx);
            })),
        };
        let peeks = (count > 1).then(|| {
            let ix = self.hero.active;
            [
                peek((ix + count - 1) % count, cx),
                peek((ix + 1) % count, cx),
            ]
        });
        let on_hover = cx.listener(|this, hovered: &bool, _, cx| this.hero_hovered(*hovered, cx));
        Some(HeroCarousel {
            slide: slide(active, cx),
            previous,
            peeks,
            active: self.hero.active,
            generation: self.hero.generation,
            cycle: self.hero.cycle,
            dwell: (!self.hero.hovered).then_some(HERO_DWELL),
            dots,
            on_hover: Box::new(on_hover),
        })
    }

    /// One Home row: its cards plus page arrows wired to this row's paging.
    fn row(
        title: SharedString,
        id: SharedString,
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
        let previous = Self::page_button(
            format!("{id}-prev"),
            IconName::ChevronLeft,
            row,
            false,
            offset >= px(0.),
            cx,
        );
        let next = Self::page_button(
            format!("{id}-next"),
            IconName::ChevronRight,
            row,
            true,
            at_end,
            cx,
        );
        Some(
            paged_row(
                id,
                typography::title(title),
                &scroll,
                cards,
                previous.into_any_element(),
                next.into_any_element(),
            )
            .into_any_element(),
        )
    }
}

impl Render for HomeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.rows.is_empty(self.layout)
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
        // (title, element id, cards), in screen order; each row's index is its scroll handle
        let mut rows: Vec<(SharedString, SharedString, Vec<AnyElement>)> = vec![
            (
                "Continue Watching".into(),
                "row-continue".into(),
                wide(&self.rows.continue_watching, cx),
            ),
            (
                "Next Up".into(),
                "row-next-up".into(),
                wide(&self.rows.next_up, cx),
            ),
        ];
        let hero = match self.layout {
            HomeLayout::Default => {
                rows.push((
                    "Movies".into(),
                    "row-movies".into(),
                    posters(&self.rows.movies, cx),
                ));
                rows.push((
                    "Series".into(),
                    "row-series".into(),
                    posters(&self.rows.series, cx),
                ));
                None
            }
            HomeLayout::Modern => {
                for (library, items) in &self.rows.libraries {
                    rows.push((
                        library.name.clone().into(),
                        format!("row-library-{}", library.id).into(),
                        posters(items, cx),
                    ));
                }
                self.hero_carousel(cx)
            }
        };

        if !self
            .scroll
            .iter()
            .map(|(id, _)| id)
            .eq(rows.iter().map(|(_, id, _)| id))
        {
            // a running slide targets a row by index, which may now be another row
            self.paging = None;
            let mut old = std::mem::take(&mut self.scroll);
            self.scroll = rows
                .iter()
                .map(|(_, id, _)| {
                    let kept = old
                        .iter()
                        .position(|(o, _)| o == id)
                        .map(|ix| old.remove(ix).1);
                    (id.clone(), kept.unwrap_or_default())
                })
                .collect();
        }
        self.advance_paging(window, cx);
        let rows: Vec<AnyElement> = rows
            .into_iter()
            .enumerate()
            .filter_map(|(ix, (title, id, cards))| {
                Self::row(title, id, cards, ix, self.scroll[ix].1.clone(), cx)
            })
            .collect();
        // shown while a background refresh fails but the rows still have the old data
        let retry: OnClick = Box::new(cx.listener(|this, _, _, cx| this.refresh(cx)));
        let banner = self
            .error
            .clone()
            .map(|error| error_banner(error, retry, self.loading, cx).px(page()));
        // rows' images freed when page stops rendering
        image_cache(lru_images("home-images", IMAGES_KEPT))
            .size_full()
            .child(
                rows_page("home")
                    .children(banner)
                    .children(hero)
                    .children(rows),
            )
            .into_any_element()
    }
}

/// Each Movie and Series Library with its newest items, in server order.
async fn library_rows(api: &Api) -> anyhow::Result<Vec<(Library, Vec<Item>)>> {
    let libraries = api.libraries().await?;
    let rows =
        futures::future::try_join_all(libraries.iter().map(|l| api.latest_in(l, PER_ROW))).await?;
    Ok(libraries.into_iter().zip(rows).collect())
}

/// The hero's films: the newest unstarted ones, each fetched in full, since list items
/// carry no logo tag, overview, age rating or streams. A failed lookup keeps the list item.
async fn featured(api: &Api, newest: &[Item]) -> Vec<Item> {
    let films = newest
        .iter()
        .filter(|film| film.user_data.playback_position_ticks == 0)
        .take(HERO_SLIDES)
        .map(|film| async move { api.item(&film.id).await.unwrap_or_else(|_| film.clone()) });
    futures::future::join_all(films).await
}
