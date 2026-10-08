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
use crate::design::{color, gap, opacity, page, radius, size, sp, text};

const SKELETON_COUNT: usize = 12;

/// Poster-shaped placeholders standing in for a grid that hasn't loaded yet, so the
/// page doesn't jump when real cards replace them.
/// Vega Skeleton is `bg-muted`, here at the one radius.
fn skeleton_grid() -> AnyElement {
    let (width, radius) = (size::poster_min_w(), radius::base());
    div()
        .size_full()
        .flex()
        .flex_wrap()
        .gap(gap::group())
        .p(page())
        .children((0..SKELETON_COUNT).map(|_| {
            v_flex()
                .w(width)
                .gap(gap::item())
                .child(Skeleton::new().w(width).h(width * 1.5).rounded(radius))
                .child(
                    Skeleton::new()
                        .h(text::body())
                        .w(relative(0.75))
                        .rounded(radius),
                )
                .child(
                    Skeleton::new()
                        .h(text::meta())
                        .w(relative(0.4))
                        .rounded(radius),
                )
        }))
        .into_any_element()
}

/// gpui-kit Empty overridden: group gap and padding, the one radius, no dashed border.
fn empty() -> Empty {
    Empty::new()
        .max_w(sp(110.))
        .gap(gap::group())
        .p(gap::group())
        .rounded(radius::base())
        .border_0()
}

/// Vega `empty-description`: relaxed, muted.
fn description(message: SharedString, cx: &App) -> EmptyDescription {
    EmptyDescription::new()
        .text_size(text::body())
        .line_height(relative(1.625))
        .text_color(color::muted_foreground(cx))
        .child(message)
}

/// Centered error block: alert icon in a muted tile, medium-weight `title`, muted
/// `detail`, then `actions`.
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
                .gap(gap::item())
                .media(
                    EmptyMedia::new()
                        .with_variant(EmptyMediaVariant::Icon)
                        .size(sp(10.))
                        .rounded(radius::base())
                        .child(
                            Icon::new(assets::IconName::CircleAlert)
                                .size(sp(6.))
                                .text_color(color::danger(cx)),
                        ),
                )
                .title(
                    EmptyTitle::new()
                        .text_size(text::body())
                        .font_weight(FontWeight::MEDIUM)
                        .child(title.into()),
                )
                .description(description(detail, cx)),
        )
        .content(EmptyContent::new().child(h_flex().gap(gap::item()).children(actions)))
}

/// Centered empty block: the faded tbis mark over a muted `message`.
pub fn empty_panel(message: SharedString, cx: &App) -> Empty {
    empty().header(
        EmptyHeader::new()
            .gap(gap::item())
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

impl Status {
    /// The status to show, if any: `error` wins, then `loading`, then the `empty`
    /// message (`Some` only when there is nothing to show).
    pub fn pick(
        error: Option<SharedString>,
        loading: bool,
        empty: Option<SharedString>,
    ) -> Option<Self> {
        match (error, loading, empty) {
            (Some(error), _, _) => Some(Self::Error(error)),
            (None, true, _) => Some(Self::Loading),
            (None, false, empty) => empty.map(Self::Empty),
        }
    }
}

/// Full-screen centered placeholder, replacing a list's content while it's empty.
/// Error shows a Retry button; `loading` puts it in its spinner state while refreshing.
pub fn full_status(status: Status, retry: OnClick, loading: bool, cx: &App) -> AnyElement {
    let body = match status {
        Status::Loading => return skeleton_grid(),
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
    div()
        .text_size(text::body())
        .text_color(color::danger(cx))
        .child(error)
}

/// Inline status line for a screen that keeps its other content up while this shows:
/// the error in `danger`, else a muted message (e.g. "Loading…", "No episodes").
pub fn inline_status(status: Option<Status>, cx: &App) -> Option<AnyElement> {
    let status = status?;
    let (fg, message) = match status {
        Status::Loading => (color::muted_foreground(cx), "Loading…".into()),
        Status::Empty(message) => (color::muted_foreground(cx), message),
        Status::Error(message) => (color::danger(cx), message),
    };
    Some(
        div()
            .text_size(text::body())
            .text_color(fg)
            .child(message)
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    use super::Status;

    #[test]
    fn pick_prefers_error_then_loading_then_empty() {
        let pick = |e: Option<&str>, l, m: Option<&str>| {
            Status::pick(e.map(Into::into), l, m.map(Into::into))
        };
        assert!(matches!(
            pick(Some("x"), true, Some("m")),
            Some(Status::Error(_))
        ));
        assert!(matches!(pick(None, true, Some("m")), Some(Status::Loading)));
        assert!(matches!(
            pick(None, false, Some("m")),
            Some(Status::Empty(_))
        ));
        assert!(pick(None, false, None).is_none());
    }
}
