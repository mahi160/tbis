use gpui_kit::component::button::Button;
use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::*;

use crate::card::{self, OnClick, OpenSeries, Play};
use crate::jellyfin::{Api, Item, Kind};

const PER_ROW: usize = 24;
const POSTER_WIDTH: f32 = 150.;
const WIDE_WIDTH: f32 = 256.;

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
    _load: Task<()>,
}

impl EventEmitter<Play> for HomeView {}
impl EventEmitter<OpenSeries> for HomeView {}

impl HomeView {
    pub fn new(api: Api) -> Self {
        Self {
            api,
            rows: Rows::default(),
            loading: false,
            error: None,
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
        Box::new(cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(Play(item.clone()))))
    }

    fn row(title: &'static str, id: &'static str, cards: Vec<AnyElement>) -> Option<AnyElement> {
        if cards.is_empty() {
            return None;
        }
        Some(
            v_flex()
                .gap_3()
                .child(
                    div()
                        .px_6()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(title),
                )
                .child(
                    h_flex()
                        .id(id)
                        .overflow_x_scroll()
                        .px_6()
                        .gap_4()
                        .items_start()
                        .children(cards),
                )
                .into_any_element(),
        )
    }
}

impl Render for HomeView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted_fg = cx.theme().muted_foreground;
        if self.rows.is_empty() {
            let status = match self.error.clone() {
                Some(error) => v_flex()
                    .gap_3()
                    .items_center()
                    .child(div().text_color(cx.theme().danger).child(error))
                    .child(
                        Button::new("home-retry")
                            .label("Retry")
                            .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
                    )
                    .into_any_element(),
                None => div()
                    .text_color(muted_fg)
                    .child(if self.loading {
                        "Loading…"
                    } else {
                        "Nothing here yet"
                    })
                    .into_any_element(),
            };
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(status)
                .into_any_element();
        }

        let api = self.api.clone();
        let wide = |items: &[Item], cx: &mut Context<Self>| -> Vec<AnyElement> {
            items
                .iter()
                .map(|item| {
                    let on_click = Self::play_on_click(item, cx);
                    card::wide_card(&api, item, px(WIDE_WIDTH), on_click, cx)
                })
                .collect()
        };
        let continue_watching = wide(&self.rows.continue_watching, cx);
        let next_up = wide(&self.rows.next_up, cx);
        let movies = self
            .rows
            .movies
            .iter()
            .map(|item| {
                let on_click = Self::play_on_click(item, cx);
                card::poster_card(&self.api, item, px(POSTER_WIDTH), Some(on_click), cx)
            })
            .collect();
        let series = self
            .rows
            .series
            .iter()
            .map(|item| {
                let item_ = item.clone();
                let on_click: OnClick =
                    Box::new(cx.listener(move |_, _: &ClickEvent, _, cx| {
                        cx.emit(OpenSeries(item_.clone()))
                    }));
                card::poster_card(&self.api, item, px(POSTER_WIDTH), Some(on_click), cx)
            })
            .collect();

        v_flex()
            .id("home")
            .size_full()
            .overflow_y_scroll()
            .py_6()
            .gap_8()
            .children(Self::row(
                "Continue Watching",
                "row-continue",
                continue_watching,
            ))
            .children(Self::row("Next Up", "row-next-up", next_up))
            .children(Self::row("Movies", "row-movies", movies))
            .children(Self::row("Series", "row-series", series))
            .into_any_element()
    }
}
