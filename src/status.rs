//! Loading/empty/error placeholder shared by every list and detail screen
//! (Home, Movies/Series library, Search, Movie/Series detail).

use gpui_kit::component::button::Button;
use gpui_kit::component::{ActiveTheme as _, v_flex};
use gpui_kit::*;

use crate::card::OnClick;

pub enum Status {
    Loading,
    /// No error and no items.
    Empty(SharedString),
    Error(SharedString),
}

/// Full-screen centered placeholder, replacing a list's content while it's empty.
/// Error shows a Retry button; `loading` puts it in its spinner state while refreshing.
pub fn full_status(status: Status, retry: OnClick, loading: bool, cx: &App) -> AnyElement {
    let theme = cx.theme();
    let body = match status {
        Status::Loading => div()
            .text_color(theme.muted_foreground)
            .child("Loading…")
            .into_any_element(),
        Status::Empty(message) => div()
            .text_color(theme.muted_foreground)
            .child(message)
            .into_any_element(),
        Status::Error(message) => v_flex()
            .gap_3()
            .items_center()
            .child(div().text_color(theme.danger).child(message))
            .child(
                Button::new("status-retry")
                    .label("Retry")
                    .loading(loading)
                    .on_click(retry),
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
