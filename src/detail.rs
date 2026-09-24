//! Shared layout for Movie and Series detail pages (`movie.rs`, `series.rs`): both are
//! a Back button, poster, and a header body, over a scrollable page with the same padding.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{IconName, h_flex};
use gpui_kit::*;

use crate::card::{self, OnClick};
use crate::jellyfin::{Api, Item};

pub const PAD: f32 = 24.;
const TICKS_PER_MINUTE: i64 = 600_000_000;

/// Ticks rounded to the nearest minute; shared rounding for every runtime display.
pub fn minutes(ticks: i64) -> i64 {
    (ticks + TICKS_PER_MINUTE / 2) / TICKS_PER_MINUTE
}

/// `1h 23m`, rounded to the nearest minute.
pub fn runtime_label(ticks: i64) -> String {
    let minutes = minutes(ticks);
    format!("{}h {:02}m", minutes / 60, minutes % 60)
}

/// Back button, poster at `poster` size, then `body` (title/meta/overview/actions).
pub fn detail_header(
    api: &Api,
    item: &Item,
    poster: (Pixels, Pixels),
    back: OnClick,
    body: impl IntoElement,
    cx: &App,
) -> impl IntoElement {
    let (width, height) = poster;
    h_flex()
        .gap_5()
        .items_start()
        .child(
            Button::new("detail-back")
                .ghost()
                .icon(IconName::ArrowLeft)
                .on_click(back),
        )
        .child(
            div()
                .w(width)
                .h(height)
                .flex_shrink_0()
                .rounded_md()
                .overflow_hidden()
                .child(card::image(
                    api.poster_url(item),
                    item.name.clone().into(),
                    cx,
                )),
        )
        .child(body)
}
