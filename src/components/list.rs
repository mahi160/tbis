//! Clickable list rows (shadcn Item): `rounded-md text-sm`, `hover:bg-muted`, gap and
//! padding from the theme's `design.item` recipe (vega: sm `gap-2.5 px-3 py-2.5`,
//! xs `gap-2 px-2.5 py-2`).

use gpui_kit::component::h_flex;
use gpui_kit::*;

use crate::design::{color, radius, sp, text, tokens};

pub enum ItemSize {
    Sm,
    Xs,
}

/// Hoverable item row; callers add content and click handling.
pub fn item(id: impl Into<ElementId>, size: ItemSize, cx: &App) -> Stateful<Div> {
    let [gap, px, py] = match size {
        ItemSize::Sm => tokens().item.sm,
        ItemSize::Xs => tokens().item.xs,
    };
    let muted = color::muted(cx);
    h_flex()
        .id(id)
        .gap(sp(gap))
        .px(sp(px))
        .py(sp(py))
        .rounded(radius::md())
        .text_size(text::sm())
        .cursor_pointer()
        .hover(move |this| this.bg(muted))
}
