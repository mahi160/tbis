//! Loading/empty/error placeholder shared by every list and detail screen
//! (Home, Movies/Series library, Search, Movie/Series detail).

use gpui_kit::component::button::Button;
use gpui_kit::component::skeleton::Skeleton;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex, v_flex};
use gpui_kit::*;

use crate::card::OnClick;

const SKELETON_CARD_WIDTH: f32 = 150.;
const SKELETON_COUNT: usize = 12;

/// Poster-shaped placeholders standing in for a grid that hasn't loaded yet, so the
/// page doesn't jump when real cards replace them.
fn skeleton_grid(radius: Pixels) -> AnyElement {
    div()
        .size_full()
        .flex()
        .flex_wrap()
        .gap_4()
        .p_6()
        .children((0..SKELETON_COUNT).map(|_| {
            v_flex()
                .w(px(SKELETON_CARD_WIDTH))
                .gap_2()
                .child(
                    Skeleton::new()
                        .w(px(SKELETON_CARD_WIDTH))
                        .h(px(SKELETON_CARD_WIDTH * 1.5))
                        .rounded(radius),
                )
                .child(Skeleton::new().h(px(14.)).w(relative(0.75)))
                .child(Skeleton::new().secondary().h(px(12.)).w(relative(0.4)))
        }))
        .into_any_element()
}

/// Centered error block: alert icon, `title`, wrapped muted `detail`, then `actions`.
/// Shared by list screens (`full_status`) and the Player's error overlay.
pub fn error_panel(
    title: impl Into<SharedString>,
    detail: SharedString,
    actions: impl IntoIterator<Item = AnyElement>,
    cx: &App,
) -> Div {
    let theme = cx.theme();
    v_flex()
        .max_w(px(440.))
        .items_center()
        .gap_2()
        .text_center()
        .child(
            Icon::new(assets::IconName::CircleAlert)
                .large()
                .text_color(theme.danger),
        )
        .child(
            div()
                .mt_1()
                .font_weight(FontWeight::SEMIBOLD)
                .child(title.into()),
        )
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(detail),
        )
        .child(h_flex().mt_3().gap_2().children(actions))
}

pub enum Status {
    Loading,
    /// No error and no items.
    Empty(SharedString),
    Error(SharedString),
}

/// Full-screen centered placeholder, replacing a list's content while it's empty.
/// Error shows a Retry button; `loading` puts it in its spinner state while refreshing.
pub fn full_status(status: Status, retry: OnClick, loading: bool, cx: &App) -> AnyElement {
    if let Status::Loading = status {
        return skeleton_grid(cx.theme().radius_lg);
    }
    let theme = cx.theme();
    let body = match status {
        Status::Loading => unreachable!(),
        Status::Empty(message) => div()
            .text_color(theme.muted_foreground)
            .child(message)
            .into_any_element(),
        Status::Error(message) => error_panel(
            "Something went wrong",
            message,
            [Button::new("status-retry")
                .label("Retry")
                .loading(loading)
                .on_click(retry)
                .into_any_element()],
            cx,
        )
        .into_any_element(),
    };
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(body)
        .into_any_element()
}

/// Inline status line for a screen that keeps its other content up while this shows:
/// the error in `danger`, else a muted message (e.g. "Loading…", "No episodes").
pub fn inline_status(status: Option<Status>, cx: &App) -> Option<AnyElement> {
    let theme = cx.theme();
    status.map(|status| match status {
        Status::Loading => div()
            .text_color(theme.muted_foreground)
            .child("Loading…")
            .into_any_element(),
        Status::Empty(message) => div()
            .text_color(theme.muted_foreground)
            .child(message)
            .into_any_element(),
        Status::Error(message) => div()
            .text_sm()
            .text_color(theme.danger)
            .child(message)
            .into_any_element(),
    })
}
