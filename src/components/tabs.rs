//! Tab strip (gpui-kit TabBar, segmented) in shadcn vega's Tabs look: a muted
//! `rounded-lg p-[3px]` list whose active trigger is a raised background chip, sized by
//! the theme's `design.tabs` step.

use gpui_kit::component::Sizable as _;
use gpui_kit::component::tab::TabBar;
use gpui_kit::*;

/// `labels` as segments, `selected` raised (none when out of range).
pub fn tabs(
    id: impl Into<ElementId>,
    labels: impl IntoIterator<Item = impl Into<SharedString>>,
    selected: usize,
    on_click: impl Fn(&usize, &mut Window, &mut App) + 'static,
) -> TabBar {
    TabBar::new(id)
        .segmented()
        .with_size(crate::design::tokens().tabs)
        .children(labels.into_iter().map(|label| label.into()))
        .selected_index(selected)
        .on_click(on_click)
}

/// `tabs` that scroll sideways when they outgrow the row.
pub fn scrolling_tabs(
    id: &'static str,
    labels: impl IntoIterator<Item = impl Into<SharedString>>,
    selected: usize,
    on_click: impl Fn(&usize, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(SharedString::from(format!("{id}-scroll")))
        .overflow_x_scroll()
        .child(tabs(id, labels, selected, on_click))
}
