use std::ops::Range;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::card::{OnClick, poster_card};
use crate::jellyfin::{Api, Item, Kind, Sort};
use crate::detail::PAD;
use crate::nav::Nav;
use crate::status::{Status, full_status, inline_status};

pub struct SortChanged(pub Sort);

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
impl EventEmitter<Nav> for LibraryView {}

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

    /// Whatever has loaded so far (empty until the tab is first opened).
    pub fn items(&self) -> &[Item] {
        &self.items
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
            // this view only ever holds Movie or Series; Episode/Other can't reach here
            Kind::Movie | Kind::Episode | Kind::Other => "Movies",
        }
    }

    fn render_card(&self, item: &Item, cx: &mut Context<Self>) -> AnyElement {
        let item_ = item.clone();
        let open: OnClick = Box::new(
            cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(Nav::Open(item_.clone()))),
        );
        poster_card(&self.api, item, self.card_width, open, cx)
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
                            .font_weight(FontWeight::BOLD)
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

        let body = if self.items.is_empty() {
            let status = if let Some(error) = self.error.clone() {
                Status::Error(error)
            } else if self.loading {
                Status::Loading
            } else {
                Status::Empty(format!("No {}", self.title().to_lowercase()).into())
            };
            let retry: OnClick = Box::new(cx.listener(|this, _, _, cx| this.refresh(cx)));
            div()
                .flex_1()
                .child(full_status(status, retry, self.loading, cx))
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

        let banner = self
            .error
            .clone()
            .filter(|_| !self.items.is_empty())
            .map(|error| {
                div()
                    .px(px(PAD))
                    .pb_2()
                    .children(inline_status(Some(Status::Error(error)), cx))
            });

        v_flex()
            .size_full()
            .child(header)
            .children(banner)
            .child(body)
    }
}
