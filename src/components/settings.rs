//! Settings layout built from the app's own components, so it shares their sizes,
//! radius and gaps (gpui-kit's Settings fixes its own rem-based row heights).
//! Same builder shape as gpui-kit's: `SettingPage` > `SettingGroup` > `SettingItem`
//! holding a `SettingField`. Left: search and a page/group list. Right: the selected
//! page (or one group), each group a bordered list of rows: label and description
//! left, control right. A search query shows matching rows from every page instead.

use std::rc::Rc;

use gpui_kit::component::input::InputState;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::button::select_button;
use super::input::search_box;
use super::list::{ItemSize, item};
use super::menu::choice_menu;
use super::typography::{caption, subheading};
use crate::design::{color, gap, page, radius, sp, text};

type Get<T> = Rc<dyn Fn(&App) -> T>;
type Set<T> = Rc<dyn Fn(T, &mut App)>;

/// The control on a setting's row.
pub enum SettingField {
    /// `(value, label)` options; `get` is the current value.
    Dropdown(Vec<(SharedString, SharedString)>, Get<SharedString>, Set<SharedString>),
    Switch(Get<bool>, Set<bool>),
}

impl SettingField {
    pub fn dropdown(
        options: Vec<(SharedString, SharedString)>,
        get: impl Fn(&App) -> SharedString + 'static,
        set: impl Fn(SharedString, &mut App) + 'static,
    ) -> Self {
        Self::Dropdown(options, Rc::new(get), Rc::new(set))
    }

    /// `dropdown` for long option lists; the menu scrolls past 8 options either way.
    pub fn scrollable_dropdown(
        options: Vec<(SharedString, SharedString)>,
        get: impl Fn(&App) -> SharedString + 'static,
        set: impl Fn(SharedString, &mut App) + 'static,
    ) -> Self {
        Self::dropdown(options, get, set)
    }

    pub fn switch(
        get: impl Fn(&App) -> bool + 'static,
        set: impl Fn(bool, &mut App) + 'static,
    ) -> Self {
        Self::Switch(Rc::new(get), Rc::new(set))
    }

    fn render(&self, id: SharedString, cx: &App) -> AnyElement {
        match self {
            Self::Dropdown(options, get, set) => {
                let current = get(cx);
                let label = options
                    .iter()
                    .find(|(value, _)| *value == current)
                    .map_or(current.clone(), |(_, label)| label.clone());
                let values: Vec<SharedString> = options.iter().map(|(v, _)| v.clone()).collect();
                let set = set.clone();
                choice_menu(
                    select_button(id, label, cx),
                    options
                        .iter()
                        .map(|(value, label)| (label.clone(), *value == current))
                        .collect(),
                    options.len() > 8,
                    move |ix, cx| set(values[ix].clone(), cx),
                    |_, _, _| {},
                )
            }
            Self::Switch(get, set) => {
                let set = set.clone();
                Switch::new(id)
                    .checked(get(cx))
                    .on_click(move |on, _, cx| set(*on, cx))
                    .into_any_element()
            }
        }
    }
}

enum Body {
    Field(SettingField),
    View(AnyView),
}

/// One row: a labelled control, or a whole custom view.
pub struct SettingItem {
    title: SharedString,
    description: Option<SharedString>,
    body: Body,
}

impl SettingItem {
    pub fn new(title: impl Into<SharedString>, field: SettingField) -> Self {
        Self {
            title: title.into(),
            description: None,
            body: Body::Field(field),
        }
    }

    /// A custom view spanning the row (e.g. charts).
    pub fn render(view: impl Into<AnyView>) -> Self {
        Self {
            title: SharedString::default(),
            description: None,
            body: Body::View(view.into()),
        }
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    fn matches(&self, query: &str) -> bool {
        self.title.to_lowercase().contains(query)
            || self
                .description
                .as_ref()
                .is_some_and(|d| d.to_lowercase().contains(query))
    }

    fn row(&self, id: SharedString, cx: &App) -> AnyElement {
        match &self.body {
            Body::View(view) => view.clone().into_any_element(),
            Body::Field(field) => h_flex()
                .justify_between()
                .gap(gap::group())
                .child(
                    v_flex()
                        .min_w_0()
                        .gap(gap::tight())
                        .child(div().text_size(text::body()).child(self.title.clone()))
                        .children(self.description.clone().map(|d| caption(d, cx))),
                )
                .child(div().flex_shrink_0().child(field.render(id, cx)))
                .into_any_element(),
        }
    }
}

#[derive(Default)]
pub struct SettingGroup {
    title: Option<SharedString>,
    description: Option<SharedString>,
    items: Vec<SettingItem>,
}

impl SettingGroup {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn item(mut self, item: SettingItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = SettingItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Heading and description over the bordered rows; only rows passing `keep`.
    fn render(
        &self,
        id: &str,
        keep: impl Fn(&SettingItem) -> bool,
        cx: &App,
    ) -> Option<AnyElement> {
        let border = color::border(cx);
        let rows: Vec<AnyElement> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| keep(item))
            .map(|(ix, item)| {
                div()
                    .p(gap::group())
                    .when(ix > 0, |row| row.border_t_1().border_color(border))
                    .child(item.row(format!("{id}-{ix}").into(), cx))
                    .into_any_element()
            })
            .collect();
        (!rows.is_empty()).then(|| {
            v_flex()
                .gap(gap::item())
                .children(self.title.clone().map(subheading))
                .children(self.description.clone().map(|d| caption(d, cx)))
                .child(
                    v_flex()
                        .bg(color::surface(cx))
                        .border_1()
                        .border_color(border)
                        .rounded(radius::base())
                        .children(rows),
                )
                .into_any_element()
        })
    }
}

pub struct SettingPage {
    title: SharedString,
    groups: Vec<SettingGroup>,
}

impl SettingPage {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            groups: Vec::new(),
        }
    }

    pub fn group(mut self, group: SettingGroup) -> Self {
        self.groups.push(group);
        self
    }

    pub fn groups(mut self, groups: impl IntoIterator<Item = SettingGroup>) -> Self {
        self.groups.extend(groups);
        self
    }
}

/// Selected page, optionally narrowed to one group; the search field.
struct State {
    page: usize,
    group: Option<usize>,
    search: Entity<InputState>,
}

#[derive(IntoElement)]
pub struct Settings {
    id: ElementId,
    pages: Vec<SettingPage>,
}

pub fn settings(id: impl Into<ElementId>) -> Settings {
    Settings {
        id: id.into(),
        pages: Vec::new(),
    }
}

impl Settings {
    pub fn pages(mut self, pages: impl IntoIterator<Item = SettingPage>) -> Self {
        self.pages.extend(pages);
        self
    }
}

/// Sidebar row; `active` gets the hover fill and medium weight.
fn nav_item(
    id: SharedString,
    label: SharedString,
    active: bool,
    indent: bool,
    on_click: impl Fn(&mut App) + 'static,
    cx: &App,
) -> AnyElement {
    item(id, ItemSize::Sm, cx)
        .when(indent, |row| row.pl(gap::group() + gap::item()))
        .when(active, |row| {
            row.bg(color::active(cx)).font_weight(FontWeight::MEDIUM)
        })
        .on_click(move |_, _, cx| on_click(cx))
        .child(label)
        .into_any_element()
}

impl RenderOnce for Settings {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |window, cx| State {
            page: 0,
            group: None,
            search: cx.new(|cx| InputState::new(window, cx).placeholder("Search")),
        });
        let (selected, selected_group, search) = {
            let s = state.read(cx);
            (s.page.min(self.pages.len().saturating_sub(1)), s.group, s.search.clone())
        };
        let query = search.read(cx).value().to_lowercase();

        let mut nav: Vec<AnyElement> = Vec::new();
        for (page_ix, page) in self.pages.iter().enumerate() {
            let select = |group: Option<usize>| {
                let state = state.clone();
                move |cx: &mut App| {
                    state.update(cx, |s, cx| {
                        (s.page, s.group) = (page_ix, group);
                        cx.notify();
                    })
                }
            };
            let on_page = page_ix == selected;
            nav.push(nav_item(
                format!("page-{page_ix}").into(),
                page.title.clone(),
                on_page && selected_group.is_none(),
                false,
                select(None),
                cx,
            ));
            let titled: Vec<_> = page
                .groups
                .iter()
                .enumerate()
                .filter_map(|(ix, g)| g.title.clone().map(|t| (ix, t)))
                .collect();
            if titled.len() > 1 {
                for (group_ix, title) in titled {
                    nav.push(nav_item(
                        format!("group-{page_ix}-{group_ix}").into(),
                        title,
                        on_page && selected_group == Some(group_ix),
                        true,
                        select(Some(group_ix)),
                        cx,
                    ));
                }
            }
        }

        let groups: Vec<AnyElement> = if query.is_empty() {
            let page = &self.pages[selected];
            page.groups
                .iter()
                .enumerate()
                .filter(|(ix, _)| selected_group.is_none_or(|g| g == *ix))
                .filter_map(|(ix, g)| g.render(&format!("s-{selected}-{ix}"), |_| true, cx))
                .collect()
        } else {
            self.pages
                .iter()
                .enumerate()
                .flat_map(|(p, page)| {
                    page.groups.iter().enumerate().map(move |(g, group)| (p, g, group))
                })
                .filter_map(|(p, g, group)| {
                    let group_hit = group
                        .title
                        .as_ref()
                        .is_some_and(|t| t.to_lowercase().contains(&query));
                    group.render(&format!("s-{p}-{g}"), |item| group_hit || item.matches(&query), cx)
                })
                .collect()
        };

        h_flex()
            .size_full()
            .items_start()
            .child(
                v_flex()
                    .w(sp(56.))
                    .h_full()
                    .flex_shrink_0()
                    .px(page())
                    .gap(gap::tight())
                    .child(div().pb(gap::item()).child(search_box(&search, cx)))
                    .children(nav),
            )
            .child(
                v_flex()
                    .id("settings-content")
                    .flex_1()
                    .h_full()
                    .min_w_0()
                    .overflow_y_scroll()
                    .px(page())
                    .pb(page())
                    .gap(gap::group())
                    .children(groups),
            )
    }
}
