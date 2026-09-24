use std::time::Duration;

use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::*;

use crate::card;
use crate::jellyfin::{Api, Item, Kind};
use crate::nav::Nav;
use crate::status::{Status, inline_status};

const PAD: f32 = 24.;
const CARD_WIDTH: f32 = 150.;
const MIN_CHARS: usize = 2;
const DEBOUNCE: Duration = Duration::from_millis(250);
const PER_KIND: usize = 24;

#[derive(Default)]
struct Results {
    movies: Vec<Item>,
    series: Vec<Item>,
    episodes: Vec<Item>,
}

impl Results {
    fn is_empty(&self) -> bool {
        self.movies.is_empty() && self.series.is_empty() && self.episodes.is_empty()
    }
}

pub struct SearchView {
    api: Api,
    query: String,
    results: Results,
    loading: bool,
    error: Option<SharedString>,
    _search: Task<()>,
}

impl EventEmitter<Nav> for SearchView {}

impl SearchView {
    pub fn new(api: Api) -> Self {
        Self {
            api,
            query: String::new(),
            results: Results::default(),
            loading: false,
            error: None,
            _search: Task::ready(()),
        }
    }

    /// Debounced; a newer query cancels the pending or in-flight one, so stale results never land.
    pub fn set_query(&mut self, query: &str, cx: &mut Context<Self>) {
        let query = query.trim().to_string();
        if query == self.query {
            return;
        }
        self.query = query.clone();
        self.error = None;
        if query.chars().count() < MIN_CHARS {
            self.results = Results::default();
            self.loading = false;
            self._search = Task::ready(());
            cx.notify();
            return;
        }
        self.loading = true;
        let api = self.api.clone();
        self._search = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(DEBOUNCE).await;
            let (movies, series, episodes) = futures::join!(
                api.search(&query, Kind::Movie, PER_KIND),
                api.search(&query, Kind::Series, PER_KIND),
                api.search(&query, Kind::Episode, PER_KIND),
            );
            this.update(cx, |this, cx| {
                this.loading = false;
                match (movies, series, episodes) {
                    (Ok(movies), Ok(series), Ok(episodes)) => {
                        this.results = Results {
                            movies,
                            series,
                            episodes,
                        };
                    }
                    (Err(err), _, _) | (_, Err(err), _) | (_, _, Err(err)) => {
                        this.error = Some(format!("Search failed: {err}").into());
                    }
                }
                cx.notify();
            })
            .ok();
        });
        cx.notify();
    }

    fn poster_section(
        &self,
        title: &'static str,
        items: &[Item],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut cards = Vec::with_capacity(items.len());
        for item in items {
            let item_ = item.clone();
            let on_click: card::OnClick = Box::new(
                cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(Nav::Open(item_.clone()))),
            );
            cards.push(card::poster_card(
                &self.api,
                item,
                px(CARD_WIDTH),
                on_click,
                cx,
            ));
        }
        section(
            title,
            h_flex().flex_wrap().gap_4().items_start().children(cards),
        )
    }

    fn episode_section(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut rows = Vec::with_capacity(self.results.episodes.len());
        for item in &self.results.episodes {
            let play = {
                let item = item.clone();
                cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(Nav::Play(item.clone())))
            };
            let muted_fg = cx.theme().muted_foreground;
            rows.push(
                h_flex()
                    .id(SharedString::from(item.id.clone()))
                    .gap_3()
                    .p_1()
                    .rounded_md()
                    .cursor_pointer()
                    .hover(|this| this.bg(cx.theme().muted))
                    .on_click(play)
                    .child(
                        div()
                            .w(px(160.))
                            .h(px(90.))
                            .flex_shrink_0()
                            .rounded_md()
                            .overflow_hidden()
                            .child(card::image(self.api.poster_url(item), "".into(), cx)),
                    )
                    .child(
                        v_flex()
                            .min_w_0()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(muted_fg)
                                    .truncate()
                                    .child(item.series_name.clone().unwrap_or_default()),
                            )
                            .child(div().truncate().child(item.episode_label())),
                    ),
            );
        }
        section("Episodes", v_flex().gap_2().children(rows))
    }
}

fn section(title: &'static str, body: impl IntoElement) -> AnyElement {
    v_flex()
        .gap_3()
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(body)
        .into_any_element()
}

impl Render for SearchView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let status = if let Some(error) = self.error.clone() {
            Some(Status::Error(error))
        } else if self.query.chars().count() < MIN_CHARS {
            Some(Status::Empty("Type at least 2 characters".into()))
        } else if self.results.is_empty() {
            Some(Status::Empty(
                if self.loading {
                    "Searching…"
                } else {
                    "No results"
                }
                .into(),
            ))
        } else {
            None
        };

        let mut sections = Vec::new();
        if !self.results.movies.is_empty() {
            sections.push(self.poster_section("Movies", &self.results.movies, cx));
        }
        if !self.results.series.is_empty() {
            sections.push(self.poster_section("Series", &self.results.series, cx));
        }
        if !self.results.episodes.is_empty() {
            sections.push(self.episode_section(cx));
        }

        v_flex()
            .id("search")
            .size_full()
            .overflow_y_scroll()
            .p(px(PAD))
            .gap_8()
            .children(inline_status(status, cx))
            .children(sections)
    }
}
