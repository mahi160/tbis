use std::ops::Range;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::card::{OnClick, OpenMovie, OpenSeries, poster_card};
use crate::jellyfin::{Api, Item, Kind, Sort};

pub struct SortChanged(pub Sort);

const PAD: f32 = 24.;
const GAP: f32 = 16.;
const MIN_CARD_WIDTH: f32 = 150.;

/// Movies or Series page: every item of one kind from all Libraries.
pub struct LibraryView {
    kind: Kind,
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

impl EventEmitter<SortChanged> for LibraryView {}
impl EventEmitter<OpenMovie> for LibraryView {}
impl EventEmitter<OpenSeries> for LibraryView {}

impl LibraryView {
    pub fn new(kind: Kind, api: Api, sort: Sort) -> Self {
        Self {
            kind,
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
        let (api, kind, sort) = (self.api.clone(), self.kind, self.sort);
        self.loading = true;
        cx.notify();
        // replacing task cancels older in-flight load
        self._load = cx.spawn(async move |this, cx| {
            let result = api.library(kind, sort).await;
            this.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok(items) => {
                        this.items = items;
                        this.error = None;
                    }
                    Err(err) => {
                        this.error = Some(format!("Could not load {}: {err}", this.title()).into())
                    }
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

    fn title(&self) -> &'static str {
        match self.kind {
            Kind::Series => "Series",
            _ => "Movies",
        }
    }

    fn render_card(&self, item: &Item, cx: &mut Context<Self>) -> AnyElement {
        let item_ = item.clone();
        let open: OnClick = match self.kind {
            Kind::Series => Box::new(
                cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(OpenSeries(item_.clone()))),
            ),
            _ => Box::new(
                cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(OpenMovie(item_.clone()))),
            ),
        };
        poster_card(&self.api, item, self.card_width, Some(open), cx)
    }

    fn render_sort_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        let current = self.sort;
        Button::new("library-sort")
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

impl Render for LibraryView {
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
                            .child(self.title()),
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
                    Button::new("library-retry")
                        .label("Retry")
                        .loading(self.loading)
                        .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
                )
                .into_any_element()
        } else if self.items.is_empty() {
            let message = if self.loading {
                "Loading…".to_string()
            } else {
                format!("No {}", self.title().to_lowercase())
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
            image_cache(retain_all(SharedString::from(format!(
                "posters-{}",
                self.title()
            ))))
            .flex_1()
            .min_h_0()
            .child(
                uniform_list(
                    "library-grid",
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
