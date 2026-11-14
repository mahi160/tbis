use std::ops::Range;
use std::time::{Duration, Instant};

use gpui_kit::component::v_flex;
use gpui_kit::*;

use crate::components::OnClick;
use crate::components::button::menu_button;
use crate::components::card::poster_card;
use crate::components::image_cache::lru_images;
use crate::components::layout::{counted_title, grid_row, grow, header_row, poster_columns};
use crate::components::menu::choice_menu;
use crate::components::status::{Placeholder, Status, error_banner, full_status};
use crate::components::tabs::scrolling_tabs;
use crate::config::LibraryGrouping;
use crate::design::{gap, page, size};
use crate::jellyfin::{Api, Item, Kind, Library, Sort};
use crate::nav::Nav;

/// Decoded posters kept per grid: a few full screens' worth (each ~0.6 MB), so
/// scrolling back is instant while memory stays bounded.
const POSTERS_KEPT: usize = 200;

pub struct SortChanged(pub Sort);

/// Movies or Series page: every item of one kind from all Libraries, or, with
/// `Separate` grouping, from the one Library picked in its tabs.
pub struct LibraryView {
    kind: Kind,
    api: Api,
    sort: Sort,
    grouping: LibraryGrouping,
    /// This kind's Libraries, in server order; loaded only with `Separate` grouping.
    libraries: Vec<Library>,
    /// The Library tab highlighted; `None` is All.
    picked: Option<Library>,
    /// The Library `items` came from; a failed load puts `picked` back to it.
    shown: Option<Library>,
    items: Vec<Item>,
    /// `items` names lowercased once per load, for Search's per-keystroke matching.
    names: Vec<String>,
    loading: bool,
    error: Option<SharedString>,
    scroll: UniformListScrollHandle,
    columns: usize,
    card_width: Pixels,
    /// When the items last loaded; `None` means stale.
    loaded_at: Option<Instant>,
    _load: Task<()>,
}

impl EventEmitter<SortChanged> for LibraryView {}
impl EventEmitter<Nav> for LibraryView {}

impl LibraryView {
    pub fn new(kind: Kind, api: Api, sort: Sort, grouping: LibraryGrouping) -> Self {
        Self {
            kind,
            api,
            sort,
            grouping,
            libraries: Vec::new(),
            picked: None,
            shown: None,
            items: Vec::new(),
            names: Vec::new(),
            loading: false,
            error: None,
            scroll: UniformListScrollHandle::new(),
            columns: 1,
            card_width: size::poster_min_w(),
            loaded_at: None,
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

    /// Marks the data out of date, so the next tab visit reloads it.
    pub fn mark_stale(&mut self) {
        self.loaded_at = None;
        // a load already in flight may predate the change; drop it so it can't mark itself fresh
        self._load = Task::ready(());
        self.loading = false;
    }

    /// Reloads unless the data is under a minute old.
    pub fn refresh_if_stale(&mut self, cx: &mut Context<Self>) {
        const FRESH_FOR: Duration = Duration::from_secs(60);
        if self.loaded_at.is_some_and(|at| at.elapsed() < FRESH_FOR) {
            return;
        }
        self.refresh(cx);
    }

    /// Merged drops the picked Library; either way the next visit reloads.
    pub fn set_grouping(&mut self, grouping: LibraryGrouping, cx: &mut Context<Self>) {
        self.grouping = grouping;
        if grouping == LibraryGrouping::Merged {
            self.libraries.clear();
            self.picked = None;
            self.shown = None;
        }
        self.mark_stale();
        cx.notify();
    }

    fn pick(&mut self, library: Option<Library>, cx: &mut Context<Self>) {
        if library == self.picked {
            return;
        }
        self.picked = library;
        self.scroll.scroll_to_item(0, ScrollStrategy::Top);
        self.refresh(cx);
    }

    /// Keeps showing the current list until fresh data arrives.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let (api, kind, sort) = (self.api.clone(), self.kind, self.sort);
        let separate = self.grouping == LibraryGrouping::Separate;
        let picked = self.picked.clone();
        self.loading = true;
        cx.notify();
        // replacing task cancels older in-flight load
        self._load = cx.spawn(async move |this, cx| {
            // whole-Library JSON is large; decode it off the UI thread
            let result = cx
                .background_spawn(async move {
                    // the grid can still show All without the tabs
                    let libraries = if separate {
                        let mut libraries = api.libraries().await.unwrap_or_default();
                        libraries.retain(|library| library.kind == kind);
                        libraries
                    } else {
                        Vec::new()
                    };
                    // a picked Library that's gone (or the only one left) falls back to All
                    let picked = picked.filter(|p| libraries.len() > 1 && libraries.contains(p));
                    let items = api.library(kind, sort, picked.as_ref()).await?;
                    let names = items.iter().map(|item| item.name.to_lowercase()).collect();
                    anyhow::Ok((libraries, picked, items, names))
                })
                .await;
            this.update(cx, |this, cx| {
                this.loading = false;
                match result {
                    Ok((libraries, picked, items, names)) => {
                        if picked != this.picked {
                            this.scroll.scroll_to_item(0, ScrollStrategy::Top);
                        }
                        this.libraries = libraries;
                        this.picked = picked.clone();
                        this.shown = picked;
                        this.items = items;
                        this.names = names;
                        this.error = None;
                        this.loaded_at = Some(Instant::now());
                    }
                    Err(err) => {
                        this.picked = this.shown.clone();
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

    /// All, then one tab per Library; none unless there are two or more to pick from.
    fn render_library_tabs(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        if self.libraries.len() < 2 {
            return None;
        }
        let selected = self.picked.as_ref().map_or(0, |picked| {
            self.libraries
                .iter()
                .position(|library| library == picked)
                .map_or(0, |ix| ix + 1)
        });
        let labels = std::iter::once(SharedString::from("All"))
            .chain(self.libraries.iter().map(|l| l.name.clone().into()));
        let tabs = scrolling_tabs(
            "library-tabs",
            labels,
            selected,
            cx.listener(|this, ix: &usize, _, cx| {
                let library = ix.checked_sub(1).and_then(|ix| this.libraries.get(ix));
                this.pick(library.cloned(), cx)
            }),
        );
        Some(div().px(page()).pb(gap::item()).child(tabs))
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
            image_cache(lru_images(
                SharedString::from(format!("posters-{}", self.title())),
                POSTERS_KEPT,
            ))
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
            .children(self.render_library_tabs(cx))
            .children(banner)
            .child(body)
    }
}
