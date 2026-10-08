//! Clickable list rows (shadcn Item): the one radius, body text, `hover:bg-muted`;
//! item-gap spacing, Xs rows tight vertically.

use gpui_kit::component::h_flex;
use gpui_kit::*;

use crate::design::{color, gap, radius, text};

pub enum ItemSize {
    Sm,
    Xs,
}

/// Hoverable item row; callers add content and click handling.
pub fn item(id: impl Into<ElementId>, size: ItemSize, cx: &App) -> Stateful<Div> {
    let py = match size {
        ItemSize::Sm => gap::item(),
        ItemSize::Xs => gap::tight(),
    };
    let hover = color::hover(cx);
    h_flex()
        .id(id)
        .gap(gap::item())
        .px(gap::item())
        .py(py)
        .rounded(radius::base())
        .text_size(text::body())
        .cursor_pointer()
        .hover(move |this| this.bg(hover))
}
