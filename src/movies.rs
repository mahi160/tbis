use std::ops::Range;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::jellyfin::{Api, Item, Sort};

pub struct SortChanged(pub Sort);
pub struct Play(pub Item);

const PAD: f32 = 24.;
const GAP: f32 = 16.;
const MIN_CARD_WIDTH: f32 = 150.;
const TITLE_HEIGHT: f32 = 20.;
const META_HEIGHT: f32 = 16.;

pub struct MoviesView {
    api: Api,
    sort: Sort,
    items: Vec<Item>,
    loading: bool,
    error: Option<SharedString>,
    scroll: UniformListScrollHandle,
    columns: usize,
    card_width: Pixels,
    _load: Task<()>,
}

impl EventEmitter<SortChanged> for MoviesView {}
impl EventEmitter<Play> for MoviesView {}

impl MoviesView {
    pub fn new(api: Api, sort: Sort) -> Self {
        Self {
            api,
            sort,
            items: Vec::new(),
            loading: false,
            error: None,
            scroll: UniformListScrollHandle::new(),
            columns: 1,
            card_width: px(MIN_CARD_WIDTH),
            _load: Task::ready(()),
        }
    }

    /// Keeps showing the current list until fresh data arrives.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let api = self.api.clone();
        let sort = self.sort;
        self.loading = true;
        cx.notify();
        // replacing task cancels older in-flight load
        self._load = cx.spawn(async move |this, cx| {
            let result = api.movies(sort).await;
            this.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(items) => {
                        this.items = items;
                        this.error = None;
                    }
                    Err(err) => this.error = Some(format!("Could not load movies: {err}").into()),
                }
                cx.notify();
            })
            .ok();
        });
    }

    fn set_sort(&mut self, sort: Sort, cx: &mut Context<Self>) {
        if sort == self.sort {
            return;
        }
        self.sort = sort;
        self.scroll.scroll_to_item(0, ScrollStrategy::Top);
        cx.emit(SortChanged(sort));
        self.refresh(cx);
    }

    fn render_rows(&self, rows: Range<usize>, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let start = row * self.columns;
            let end = (start + self.columns).min(self.items.len());
            let mut cards = Vec::with_capacity(end - start);
            for item in &self.items[start..end] {
                cards.push(self.render_card(item, cx));
            }
            out.push(
                h_flex()
                    .px(px(PAD))
                    .pb(px(GAP))
                    .gap(px(GAP))
                    .items_start()
                    .children(cards)
                    .into_any_element(),
            );
        }
        out
    }

    fn render_card(&self, item: &Item, cx: &mut Context<Self>) -> AnyElement {
        let play = {
            let item = item.clone();
            cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(Play(item.clone())))
        };
        let theme = cx.theme();
        let (muted, muted_fg) = (theme.muted, theme.muted_foreground);
        let name: SharedString = item.name.clone().into();
        let placeholder = move |name: SharedString| {
            div()
                .size_full()
                .bg(muted)
                .flex()
                .items_center()
                .justify_center()
                .p_2()
                .text_sm()
                .text_center()
                .text_color(muted_fg)
                .child(name)
                .into_any_element()
        };
        let poster = match self.api.poster_url(item) {
            Some(url) => {
                let name = name.clone();
                img(url)
                    .size_full()
                    .object_fit(ObjectFit::Cover)
                    .with_fallback(move || placeholder(name.clone()))
                    .into_any_element()
            }
            None => placeholder(name.clone()),
        };

        v_flex()
            .id(SharedString::from(item.id.clone()))
            .w(self.card_width)
            .gap_1()
            .cursor_pointer()
            .on_click(play)
            .child(
                div()
                    .relative()
                    .w(self.card_width)
                    .h(self.card_width * 1.5)
                    .rounded_md()
                    .overflow_hidden()
                    .bg(muted)
                    .child(poster)
                    .when(item.user_data.played, |this| {
                        this.child(
                            div()
                                .absolute()
                                .top_2()
                                .right_2()
                                .size_6()
                                .rounded_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(theme.primary)
                                .text_color(theme.primary_foreground)
                                .child(Icon::new(IconName::Check).small()),
                        )
                    }),
            )
            .child(div().h(px(TITLE_HEIGHT)).text_sm().truncate().child(name))
            .child(
                div()
                    .h(px(META_HEIGHT))
                    .text_xs()
                    .text_color(muted_fg)
                    .child(
                        item.production_year
                            .map(|y| y.to_string())
                            .unwrap_or_default(),
                    ),
            )
            .into_any_element()
    }

    fn render_sort_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        let current = self.sort;
        Button::new("movies-sort")
            .ghost()
            .small()
            .label(format!("Sort: {}", current.label()))
            .dropdown_caret(true)
            .dropdown_menu(move |mut menu, _, _| {
                for sort in Sort::ALL {
                    let this = this.clone();
                    menu = menu.item(
                        PopupMenuItem::new(sort.label())
                            .checked(sort == current)
                            .on_click(move |_, _, cx| {
                                this.update(cx, |this, cx| this.set_sort(sort, cx)).ok();
                            }),
                    );
                }
                menu
            })
    }
}

impl Render for MoviesView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let available = window.viewport_size().width.as_f32() - PAD * 2.;
        self.columns = (((available + GAP) / (MIN_CARD_WIDTH + GAP)).floor() as usize).max(1);
        self.card_width = px((available - GAP * (self.columns - 1) as f32) / self.columns as f32);

        let muted_fg = cx.theme().muted_foreground;
        let header = h_flex()
            .px(px(PAD))
            .py_4()
            .justify_between()
            .child(
                h_flex()
                    .gap_2()
                    .items_baseline()
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Movies"),
                    )
                    .when(!self.items.is_empty(), |this| {
                        this.child(
                            div()
                                .text_sm()
                                .text_color(muted_fg)
                                .child(self.items.len().to_string()),
                        )
                    }),
            )
            .child(self.render_sort_menu(cx));

        let body = if let Some(error) = self.error.clone().filter(|_| self.items.is_empty()) {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .gap_3()
                .child(div().text_color(cx.theme().danger).child(error))
                .child(
                    Button::new("movies-retry")
                        .label("Retry")
                        .loading(self.loading)
                        .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
                )
                .into_any_element()
        } else if self.items.is_empty() {
            let message = if self.loading {
                "Loading…"
            } else {
                "No movies"
            };
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(muted_fg)
                .child(message)
                .into_any_element()
        } else {
            let rows = self.items.len().div_ceil(self.columns);
            // posters freed when page stops rendering
            // ponytail: retains every poster scrolled past; LRU cache if memory hurts
            image_cache(retain_all("movie-posters"))
                .flex_1()
                .min_h_0()
                .child(
                    uniform_list(
                        "movies-grid",
                        rows,
                        cx.processor(|this, range, _, cx| this.render_rows(range, cx)),
                    )
                    .size_full()
                    .track_scroll(&self.scroll),
                )
                .into_any_element()
        };

        v_flex()
            .size_full()
            .child(header)
            .children(
                self.error
                    .clone()
                    .filter(|_| !self.items.is_empty())
                    .map(|error| {
                        div()
                            .px(px(PAD))
                            .pb_2()
                            .text_sm()
                            .text_color(cx.theme().danger)
                            .child(error)
                    }),
            )
            .child(body)
    }
}
