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
use crate::design::{color, gap, opacity, overlay, page, radius, size, sp, text};

const SKELETON_COUNT: usize = 12;
const ROW_SKELETON_COUNT: usize = 8;
const EPISODE_SKELETON_COUNT: usize = 3;

/// Which loaded layout a loading `full_status` stands in for.
#[derive(Clone, Copy)]
pub enum Placeholder {
    /// Wrapping poster grid (Library).
    Grid,
    /// Wide-card row, then poster row, each under a heading (Home).
    Rows,
}

/// Art block with a title bar and a meta bar under it, like a loaded card.
/// Vega Skeleton is `bg-muted`, here at the one radius.
fn card_skeleton(width: Pixels, art_height: Pixels) -> Div {
    let radius = radius::base();
    v_flex()
        .w(width)
        .flex_shrink_0()
        .gap(gap::item())
        .child(Skeleton::new().w(width).h(art_height).rounded(radius))
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
}

/// Poster-shaped placeholders standing in for a grid that hasn't loaded yet, so the
/// page doesn't jump when real cards replace them.
fn skeleton_grid() -> AnyElement {
    let width = size::poster_min_w();
    div()
        .size_full()
        .flex()
        .flex_wrap()
        .gap(gap::group())
        .p(page())
        .children((0..SKELETON_COUNT).map(|_| card_skeleton(width, width * 1.5)))
        .into_any_element()
}

/// Home-shaped placeholders: same page padding and gaps as `rows_page`/`scroll_row`.
fn skeleton_rows() -> AnyElement {
    let block = |width: Pixels, art_height: Pixels| {
        v_flex()
            .gap(gap::item())
            .child(
                Skeleton::new()
                    .h(text::title())
                    .w(size::wide_w())
                    .rounded(radius::base())
                    .ml(page()),
            )
            .child(
                h_flex()
                    .px(page())
                    .gap(gap::group())
                    .items_start()
                    .children((0..ROW_SKELETON_COUNT).map(|_| card_skeleton(width, art_height))),
            )
    };
    v_flex()
        .size_full()
        .overflow_hidden()
        .py(page())
        .gap(page())
        .child(block(size::wide_w(), size::wide_w() * (9. / 16.)))
        .child(block(size::poster_w(), size::poster_w() * 1.5))
        .into_any_element()
}

/// Episode-row-shaped placeholders for a season whose Episodes are still loading.
pub fn episode_skeletons() -> Div {
    let radius = radius::base();
    v_flex()
        .gap(gap::tight())
        .children((0..EPISODE_SKELETON_COUNT).map(|_| {
            h_flex()
                .gap(gap::item())
                .px(gap::item())
                .py(gap::item())
                .child(Skeleton::new().w(sp(48.)).h(sp(27.)).rounded(radius))
                .child(
                    v_flex()
                        .gap(gap::tight())
                        .child(
                            Skeleton::new()
                                .h(text::body())
                                .w(size::wide_w())
                                .rounded(radius),
                        )
                        .child(
                            Skeleton::new()
                                .h(text::meta())
                                .w(sp(30.))
                                .rounded(radius),
                        ),
                )
        }))
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

/// Vega `empty-description`: relaxed, in `fill`.
fn description(message: SharedString, fill: Hsla) -> EmptyDescription {
    EmptyDescription::new()
        .text_size(text::body())
        .line_height(relative(1.625))
        .text_color(fill)
        .child(message)
}

/// Where an `error_panel` sits, so its text stays readable on that background.
#[derive(Clone, Copy)]
pub enum Tone {
    /// On the app background: theme colors.
    Theme,
    /// Over video: fixed overlay tints.
    Video,
}

/// Centered error block: alert icon in a muted tile, medium-weight `title`, muted
/// `detail`, then `actions`.
/// Shared by list screens (`full_status`, `Tone::Theme`) and the Player's error overlay
/// (`Tone::Video`).
pub fn error_panel(
    title: impl Into<SharedString>,
    detail: SharedString,
    actions: impl IntoIterator<Item = AnyElement>,
    tone: Tone,
    cx: &App,
) -> Empty {
    let (empty, detail_fill) = match tone {
        Tone::Theme => (empty(), color::muted_foreground(cx)),
        Tone::Video => (empty().text_color(overlay::WHITE), overlay::TEXT_MUTED),
    };
    empty
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
                .description(description(detail, detail_fill)),
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
            .description(description(message, color::muted_foreground(cx))),
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
/// Loading shows `placeholder`-shaped skeletons. Error shows a Retry button; `loading`
/// puts it in its spinner state while refreshing.
pub fn full_status(
    status: Status,
    placeholder: Placeholder,
    retry: OnClick,
    loading: bool,
    cx: &App,
) -> AnyElement {
    let body = match status {
        Status::Loading => {
            return match placeholder {
                Placeholder::Grid => skeleton_grid(),
                Placeholder::Rows => skeleton_rows(),
            };
        }
        Status::Empty(message) => empty_panel(message, cx).into_any_element(),
        Status::Error(message) => error_panel(
            "Something went wrong",
            message,
            [button("status-retry", "Retry")
                .loading(loading)
                .on_click(retry)
                .into_any_element()],
            Tone::Theme,
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
