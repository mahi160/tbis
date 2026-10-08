//! App title bar (gpui-kit TitleBar): brand on the left, `center` truly centered
//! across the window, `trailing` on the right.

use gpui_kit::component::{TitleBar, h_flex};
use gpui_kit::*;

use super::logo::brand;
use crate::design::{color, gap, size, sp, tokens, window};

pub fn app_title_bar(
    center: impl IntoElement,
    trailing: impl IntoIterator<Item = AnyElement>,
    cx: &App,
) -> TitleBar {
    // trailing inset = avatar's vertical inset, so its box sits concentric with window corner
    let inset = ((window::TITLE_BAR_H - sp(tokens().avatar.default)) / 2.).max(sp(2.));
    TitleBar::new()
        // grows with tall content (big type scales)
        .h_auto()
        .min_h(window::TITLE_BAR_H)
        .border_b_0()
        .bg(color::background(cx))
        .child(brand(cx).pl(window::BRAND_LEAD))
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
        .child(h_flex().pr(inset).gap(gap::tight()).children(trailing))
}
