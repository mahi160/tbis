//! Loading/empty/error placeholder shared by every list and detail screen
//! (Home, Movies/Series library, Search, Movie/Series detail).

use gpui_kit::component::empty::{
    Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant, EmptyTitle,
};
use gpui_kit::component::skeleton::Skeleton;
use gpui_kit::component::{Icon, h_flex, v_flex};
use gpui_kit::*;

use super::OnClick;
use super::button::button;
use super::logo::logo_mark;
use crate::design::{color, opacity, radius, size, sp, text};

const SKELETON_COUNT: usize = 12;

/// Poster-shaped placeholders standing in for a grid that hasn't loaded yet, so the
/// page doesn't jump when real cards replace them.
/// Vega Skeleton is `bg-muted rounded-md`; poster stand-ins round like images (`lg`).
fn skeleton_grid(radius: Pixels, line: Pixels) -> AnyElement {
    let width = size::poster_min_w();
    div()
        .size_full()
        .flex()
        .flex_wrap()
        .gap(sp(4.))
        .p(sp(6.))
        .children((0..SKELETON_COUNT).map(|_| {
            v_flex()
                .w(width)
                .gap(sp(2.))
                .child(Skeleton::new().w(width).h(width * 1.5).rounded(radius))
                .child(Skeleton::new().h(text::sm()).w(relative(0.75)).rounded(line))
                .child(Skeleton::new().h(text::xs()).w(relative(0.4)).rounded(line))
        }))
        .into_any_element()
}

/// gpui-kit Empty overridden to vega: `gap-4 rounded-lg p-12`, no dashed border.
fn empty() -> Empty {
    Empty::new()
        .max_w(sp(110.))
        .gap(sp(4.))
        .p(sp(12.))
        .rounded(radius::lg())
        .border_0()
}

/// Vega `empty-description`: `text-sm/relaxed` muted.
fn description(message: SharedString, cx: &App) -> EmptyDescription {
    EmptyDescription::new()
        .text_size(text::sm())
        .line_height(relative(1.625))
        .text_color(color::muted_foreground(cx))
        .child(message)
}

/// Centered error block: alert icon in a muted `size-10 rounded-lg` tile, `title`
/// (`text-lg font-medium`), muted `detail`, then `actions`.
/// Shared by list screens (`full_status`) and the Player's error overlay.
pub fn error_panel(
    title: impl Into<SharedString>,
    detail: SharedString,
    actions: impl IntoIterator<Item = AnyElement>,
    cx: &App,
) -> Empty {
    empty()
        .header(
            EmptyHeader::new()
                .gap(sp(2.))
                .media(
                    EmptyMedia::new()
                        .with_variant(EmptyMediaVariant::Icon)
                        .size(sp(10.))
                        .rounded(radius::lg())
                        .child(
                            Icon::new(assets::IconName::CircleAlert)
                                .size(sp(6.))
                                .text_color(color::danger(cx)),
                        ),
                )
                .title(
                    EmptyTitle::new()
                        .text_size(text::lg())
                        .font_weight(FontWeight::MEDIUM)
                        .child(title.into()),
                )
                .description(description(detail, cx)),
        )
        .content(EmptyContent::new().child(h_flex().gap(sp(2.)).children(actions)))
}

/// Centered empty block: the faded tbis mark over a muted `message`.
pub fn empty_panel(message: SharedString, cx: &App) -> Empty {
    empty().header(
        EmptyHeader::new()
            .gap(sp(2.))
            .media(
                EmptyMedia::new().child(
                    logo_mark(sp(12.))
                        .text_color(color::muted_foreground(cx))
                        .opacity(opacity::HALF),
                ),
            )
            .description(description(message, cx)),
    )
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
        return skeleton_grid(radius::lg(), radius::md());
    }
    let body = match status {
        Status::Loading => unreachable!(),
        Status::Empty(message) => empty_panel(message, cx).into_any_element(),
        Status::Error(message) => error_panel(
            "Something went wrong",
            message,
            [button("status-retry", "Retry")
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

/// Error line kept above content that's still showing older data.
pub fn error_banner(error: SharedString, cx: &App) -> Div {
    div().children(inline_status(Some(Status::Error(error)), cx))
}

/// Inline status line for a screen that keeps its other content up while this shows:
/// the error in `danger`, else a muted message (e.g. "Loading…", "No episodes").
pub fn inline_status(status: Option<Status>, cx: &App) -> Option<AnyElement> {
    let (muted_fg, danger) = (color::muted_foreground(cx), color::danger(cx));
    status.map(|status| match status {
        Status::Loading => div()
            .text_color(muted_fg)
            .child("Loading…")
            .into_any_element(),
        Status::Empty(message) => div()
            .text_color(muted_fg)
            .child(message)
            .into_any_element(),
        Status::Error(message) => div()
            .text_size(text::sm())
            .text_color(danger)
            .child(message)
            .into_any_element(),
    })
}
