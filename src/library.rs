use std::ops::Range;

use gpui_kit::component::v_flex;
use gpui_kit::*;

use crate::components::OnClick;
use crate::components::button::menu_button;
use crate::components::card::poster_card;
use crate::components::layout::{counted_title, grid_row, grow, header_row, poster_columns};
use crate::components::menu::choice_menu;
use crate::components::status::{Placeholder, Status, error_banner, full_status};
use crate::design::{gap, page, size};
use crate::jellyfin::{Api, Item, Kind, Sort};
use crate::nav::Nav;

pub struct SortChanged(pub Sort);

/// Movies or Series page: every item of one kind from all Libraries.
pub struct LibraryView {
    kind: Kind,
    api: Api,
    sort: Sort,
    items: Vec<Item>,
    /// `items` names lowercased once per load, for Search's per-keystroke matching.
    names: Vec<String>,
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
            names: Vec::new(),
            loading: false,
            error: None,
            scroll: UniformListScrollHandle::new(),
            columns: 1,
            card_width: size::poster_min_w(),
            _load: Task::ready(()),
        }
    }

    /// Loaded items whose name contains `needle` (already lowercase); empty until the
    /// tab is first opened.
    pub fn name_matches(&self, needle: &str) -> impl Iterator<Item = &Item> {
        self.items
            .iter()
            .zip(&self.names)
            .filter(move |(_, name)| name.contains(needle))
            .map(|(item, _)| item)
    }

    /// Keeps showing the current list until fresh data arrives.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let (api, kind, sort) = (self.api.clone(), self.kind, self.sort);
        self.loading = true;
        cx.notify();
        // replacing task cancels older in-flight load
        self._load = cx.spawn(async move |this, cx| {
            // whole-Library JSON is large; decode it off the UI thread
            let result = cx
                .background_spawn(async move {
                    let items = api.library(kind, sort).await?;
                    let names = items.iter().map(|item| item.name.to_lowercase()).collect();
                    anyhow::Ok((items, names))
                })
                .await;
            this.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok((items, names)) => {
                        this.items = items;
                        this.names = names;
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
            out.push(grid_row(cards).into_any_element());
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
        let item_ = item.clone();
        let play: OnClick = Box::new(
            cx.listener(move |_, _: &ClickEvent, _, cx| cx.emit(Nav::Play(item_.clone()))),
        );
        poster_card(&self.api, item, self.card_width, play, open, cx)
    }

    fn render_sort_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        let current = self.sort;
        choice_menu(
            menu_button("library-sort", cx).label(format!("Sort: {}", current.label())),
            Sort::ALL
                .iter()
                .map(|sort| (sort.label().into(), *sort == current))
                .collect(),
            false,
            move |ix, cx| {
                this.update(cx, |this, cx| this.set_sort(Sort::ALL[ix], cx))
                    .ok();
            },
            |_, _, _| {},
        )
    }
}

impl Render for LibraryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        (self.columns, self.card_width) = poster_columns(window);

        let count = (!self.items.is_empty()).then_some(self.items.len());
        let header = header_row(
            counted_title(self.title(), count, cx),
            self.render_sort_menu(cx),
        )
        .px(page())
        .py(gap::group());

        let body = if self.items.is_empty()
            && let Some(status) = Status::pick(
                self.error.clone(),
                self.loading,
                Some(format!("No {}", self.title().to_lowercase()).into()),
            ) {
            let retry: OnClick = Box::new(cx.listener(|this, _, _, cx| this.refresh(cx)));
            grow()
                .child(full_status(
                    status,
                    Placeholder::Grid,
                    retry,
                    self.loading,
                    cx,
                ))
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

        let retry: OnClick = Box::new(cx.listener(|this, _, _, cx| this.refresh(cx)));
        let banner = self
            .error
            .clone()
            .filter(|_| !self.items.is_empty())
            .map(|error| {
                error_banner(error, retry, self.loading, cx)
                    .px(page())
                    .pb(gap::item())
            });

        v_flex()
            .size_full()
            .child(header)
            .children(banner)
            .child(body)
    }
}
