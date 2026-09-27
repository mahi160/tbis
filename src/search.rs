use std::time::Duration;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Sizable as _, h_flex, v_flex};
use gpui_kit::*;

use crate::card;
use crate::jellyfin::{Api, Item, Kind};
use crate::library::LibraryView;
use crate::detail::PAD;
use crate::nav::Nav;
use crate::status::{Status, inline_status};

const CARD_WIDTH: f32 = 150.;
const MIN_CHARS: usize = 2;
const DEBOUNCE: Duration = Duration::from_millis(250);
const PER_KIND: usize = 24;
const RECENT_MAX: usize = 10;

pub enum SearchEvent {
    /// A recent search was picked; the app puts it in the search field.
    Pick(String),
    /// Recent searches changed; the app saves them.
    RecentChanged(Vec<String>),
}

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
    /// Movies and Series libraries, matched locally while the server answers.
    local: [Entity<LibraryView>; 2],
    /// Newest first.
    recent: Vec<String>,
    query: String,
    results: Results,
    loading: bool,
    error: Option<SharedString>,
    _search: Task<()>,
}

impl EventEmitter<Nav> for SearchView {}
impl EventEmitter<SearchEvent> for SearchView {}

impl SearchView {
    pub fn new(api: Api, local: [Entity<LibraryView>; 2], recent: Vec<String>) -> Self {
        Self {
            api,
            local,
            recent,
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
        self.results = self.local_matches(&query, cx);
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
                        let shown = std::mem::take(&mut this.results);
                        this.results = Results {
                            movies: merge(shown.movies, movies),
                            series: merge(shown.series, series),
                            episodes: merge(shown.episodes, episodes),
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

    pub fn has_recent(&self) -> bool {
        !self.recent.is_empty()
    }

    /// Moves the current query to the front of recent searches.
    pub fn remember(&mut self, cx: &mut Context<Self>) {
        if self.query.chars().count() < MIN_CHARS {
            return;
        }
        let query = self.query.clone();
        let lower = query.to_lowercase();
        self.recent.retain(|r| r.to_lowercase() != lower);
        self.recent.insert(0, query);
        self.recent.truncate(RECENT_MAX);
        cx.emit(SearchEvent::RecentChanged(self.recent.clone()));
    }

    /// Name matches among already-loaded library items; no Episodes (never loaded).
    fn local_matches(&self, query: &str, cx: &App) -> Results {
        let needle = query.to_lowercase();
        let find = |library: &Entity<LibraryView>| {
            library
                .read(cx)
                .items()
                .iter()
                .filter(|item| item.name.to_lowercase().contains(&needle))
                .take(PER_KIND)
                .cloned()
                .collect()
        };
        Results {
            movies: find(&self.local[0]),
            series: find(&self.local[1]),
            episodes: Vec::new(),
        }
    }

    fn recent_section(&self, cx: &mut Context<Self>) -> AnyElement {
        let clear = Button::new("clear-recent")
            .ghost()
            .xsmall()
            .label("Clear")
            .on_click(cx.listener(|this, _, _, cx| {
                this.recent.clear();
                cx.emit(SearchEvent::RecentChanged(Vec::new()));
                cx.notify();
            }));
        let muted_fg = cx.theme().muted_foreground;
        let rows = self.recent.iter().enumerate().map(|(ix, query)| {
            let query = query.clone();
            h_flex()
                .id(("recent", ix))
                .gap_2()
                .px_2()
                .py_1p5()
                .rounded_md()
                .cursor_pointer()
                .hover(|this| this.bg(cx.theme().muted))
                .child(Icon::new(IconName::Search).small().text_color(muted_fg))
                .child(query.clone())
                // mouse down, not click: focus leaving the search field on press
                // hides this list before a click could complete
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |_, _, _, cx| cx.emit(SearchEvent::Pick(query.clone()))),
                )
        });
        v_flex()
            .max_w(px(480.))
            .gap_2()
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::BOLD)
                            .child("Recent searches"),
                    )
                    .child(clear),
            )
            .children(rows)
            .into_any_element()
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
            let open = {
                let item = item.clone();
                cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(Nav::Open(item.clone())))
            };
            let muted_fg = cx.theme().muted_foreground;
            rows.push(
                h_flex()
                    .id(SharedString::from(item.id.clone()))
                    .gap_3()
                    .p_1()
                    .rounded(cx.theme().radius_lg)
                    .cursor_pointer()
                    .hover(|this| this.bg(cx.theme().muted))
                    .on_click(open)
                    .child(
                        div()
                            .w(px(160.))
                            .h(px(90.))
                            .flex_shrink_0()
                            .rounded(cx.theme().radius_lg)
                            .overflow_hidden()
                            .child(card::image(self.api.poster_url(item), cx)),
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

/// `shown` kept in place (no layout jump), then server items not already shown.
fn merge(mut shown: Vec<Item>, server: Vec<Item>) -> Vec<Item> {
    for item in server {
        if !shown.iter().any(|s| s.id == item.id) {
            shown.push(item);
        }
    }
    shown
}

#[cfg(test)]
mod tests {
    use super::{Item, merge};

    fn item(id: &str) -> Item {
        let mut item = Item::default();
        item.id = id.into();
        item
    }

    #[test]
    fn merge_keeps_shown_order_and_skips_duplicates() {
        let merged = merge(
            vec![item("b"), item("a")],
            vec![item("a"), item("c"), item("b")],
        );
        let ids: Vec<&str> = merged.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, ["b", "a", "c"]);
    }
}

fn section(title: &'static str, body: impl IntoElement) -> AnyElement {
    v_flex()
        .gap_3()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child(title),
        )
        .child(body)
        .into_any_element()
}

impl Render for SearchView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let short = self.query.chars().count() < MIN_CHARS;
        let status = if let Some(error) = self.error.clone() {
            Some(Status::Error(error))
        } else if short && self.query.is_empty() && self.has_recent() {
            None
        } else if short {
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
        if self.query.is_empty() && self.has_recent() {
            sections.push(self.recent_section(cx));
        }
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
