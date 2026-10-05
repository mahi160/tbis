//! App title bar (gpui-kit TitleBar): brand on the left, `center` truly centered
//! across the window, `trailing` on the right.

use gpui_kit::component::{TitleBar, h_flex};
use gpui_kit::*;

use super::logo::brand;
use crate::design::{color, size, sp};

pub fn app_title_bar(
    center: impl IntoElement,
    trailing: impl IntoIterator<Item = AnyElement>,
    cx: &App,
) -> TitleBar {
    TitleBar::new()
        .h(sp(10.))
        .border_b_0()
        .bg(color::background(cx))
        .child(brand(cx))
        // spans whole window width so center sits at true center; offset cancels TitleBar left padding
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(-size::TITLE_BAR_INSET)
                .right_0()
                .flex()
                .items_center()
                .justify_center()
                .child(center),
        )
        .child(h_flex().pr(sp(2.)).gap(sp(1.)).children(trailing))
}
