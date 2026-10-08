//! Page-level layout: padding, scrolling pages, centering, rows and grids.

use gpui_kit::component::{InteractiveElementExt as _, h_flex, v_flex};
use gpui_kit::*;

use super::typography::title;
use crate::design::{color, gap, page, size, text};

/// App root: full size, theme background and text color.
pub fn screen(cx: &App) -> Div {
    v_flex()
        .size_full()
        .bg(color::background(cx))
        .text_color(color::foreground(cx))
}

/// Remaining space in a column, allowed to shrink so inner scrolling works.
pub fn grow() -> Div {
    div().flex_1().min_h_0()
}

/// Flexible gap pushing siblings apart in a row.
pub fn spacer() -> Div {
    div().flex_1()
}

/// Vertically scrolling page with the standard gutter.
pub fn scroll_page(id: impl Into<ElementId>) -> Stateful<Div> {
    v_flex().id(id).size_full().overflow_y_scroll().p(page())
}

/// Vertically scrolling page of full-bleed horizontal rows (rows bring their own gutter).
/// Photon's `.page { padding-block: 1.5rem 3rem }` + `.section` gap.
pub fn rows_page(id: impl Into<ElementId>) -> Stateful<Div> {
    v_flex()
        .id(id)
        .size_full()
        .overflow_y_scroll()
        // else horizontal row swipes leak into page's vertical scroll
        .lock_scroll_axis()
        .py(page())
        .gap(page())
}

/// Centers `child` in all the available space.
pub fn centered(child: impl IntoElement) -> Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(child)
}

/// `centered`, but scrolls when `child` is taller than the window, e.g. a sign-in form
/// under a large-spacing theme.
pub fn centered_scroll(id: impl Into<ElementId>, child: impl IntoElement) -> Stateful<Div> {
    div()
        .id(id)
        .size_full()
        .overflow_y_scroll()
        .child(centered(child).h_auto().min_h_full().py(gap::group()))
}

/// Title on the left, `trailing` on the right.
pub fn header_row(heading: impl IntoElement, trailing: impl IntoElement) -> Div {
    h_flex().justify_between().child(heading).child(trailing)
}

/// Page title with a muted count beside it (`Movies 124`).
pub fn counted_title(label: impl Into<SharedString>, count: Option<usize>, cx: &App) -> Div {
    h_flex()
        .gap(gap::item())
        // gpui text reports no baseline; centering is the honest alignment
        .items_center()
        .child(title(label.into()))
        .children(count.map(|n| {
            div()
                .text_size(text::body())
                .text_color(color::muted_foreground(cx))
                .child(n.to_string())
        }))
}

/// Optional back button, then the page title.
pub fn page_header(back: Option<impl IntoElement>, label: impl Into<SharedString>) -> Div {
    h_flex()
        .p(page())
        .pb(gap::item())
        .gap(gap::item())
        .children(back)
        .child(title(label.into()))
}

/// Horizontally scrolling row of cards, tracked by `scroll` for paging.
pub fn scroll_row(
    id: impl Into<ElementId>,
    scroll: &ScrollHandle,
    cards: Vec<AnyElement>,
) -> Stateful<Div> {
    h_flex()
        .id(id)
        .overflow_x_scroll()
        // else vertical swipes with slight x drift scroll the row
        .lock_scroll_axis()
        .track_scroll(scroll)
        .px(page())
        .gap(gap::group())
        .items_start()
        .children(cards)
}

/// One row of a virtualized card grid.
pub fn grid_row(cards: Vec<AnyElement>) -> Div {
    h_flex()
        .px(page())
        .pb(gap::group())
        .gap(gap::group())
        .items_start()
        .children(cards)
}

/// Poster grid columns and card width for the current window: as many
/// `poster_min_w` cards as fit inside the page gutter, stretched to fill the row.
pub fn poster_columns(window: &Window) -> (usize, Pixels) {
    let (gap, min_card) = (gap::group().as_f32(), size::poster_min_w().as_f32());
    let available = window.viewport_size().width.as_f32() - page().as_f32() * 2.;
    let columns = (((available + gap) / (min_card + gap)).floor() as usize).max(1);
    (
        columns,
        px((available - gap * (columns - 1) as f32) / columns as f32),
    )
}

/// Card grid in rows of `columns`, no gutter of its own.
pub fn card_grid(cards: Vec<AnyElement>, columns: usize) -> Div {
    let mut cards = cards.into_iter().peekable();
    let mut grid = v_flex().gap(gap::group());
    while cards.peek().is_some() {
        let row: Vec<_> = cards.by_ref().take(columns).collect();
        grid = grid.child(h_flex().gap(gap::group()).items_start().children(row));
    }
    grid
}
