//! A titled card row that scrolls sideways and pages with arrow buttons.

use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::*;

use super::layout::{header_row, scroll_row};
use crate::design::{gap, page};

/// A titled row of cards that scrolls sideways, with previous and next page buttons.
pub fn paged_row(
    id: impl Into<ElementId>,
    heading: impl IntoElement,
    scroll: &ScrollHandle,
    cards: Vec<AnyElement>,
    previous: AnyElement,
    next: AnyElement,
) -> Div {
    v_flex()
        .gap(gap::item())
        .child(
            header_row(
                heading,
                h_flex().gap(gap::tight()).child(previous).child(next),
            )
            .px(page()),
        )
        .child(scroll_row(id, scroll, cards))
}
