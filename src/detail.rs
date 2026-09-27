//! Shared layout for Movie and Series detail pages (`movie.rs`, `series.rs`): both are
//! a Back button, poster, and a header body, over a scrollable page with the same padding.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Selectable as _, Sizable as _, h_flex, v_flex,
};
use gpui_kit::*;

use crate::card::{self, OnClick};
use crate::jellyfin::{Api, Item, UserData};

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

/// Back button, `image` (URL, width, height), then `body` (title/meta/overview/actions).
pub fn detail_header(
    item: &Item,
    image: (Option<String>, Pixels, Pixels),
    back: OnClick,
    body: impl IntoElement,
    cx: &App,
) -> impl IntoElement {
    let (url, width, height) = image;
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
                .child(card::image(url, item.name.clone().into(), cx)),
        )
        .child(body)
}

/// Watched/Favorite flag a detail page can flip on one of its items.
#[derive(Clone, Copy)]
pub enum Toggle {
    Played,
    Favorite,
}

/// Detail page holding items whose `UserData` a `Toggle` flips.
pub trait UserDataView: Sized + 'static {
    fn user_data(&mut self, id: &str) -> Option<&mut UserData>;
    fn set_error(&mut self, error: SharedString);
    /// Server accepted the change; e.g. Series detail reloads its Episodes.
    fn saved(&mut self, _id: &str, _toggle: Toggle, _cx: &mut Context<Self>) {}
}

/// Flips `toggle` on item `id` now, then tells the server; on failure restores the
/// previous state and reports the error.
pub fn toggle<V: UserDataView>(
    view: &mut V,
    api: &Api,
    id: &str,
    toggle: Toggle,
    cx: &mut Context<V>,
) {
    let Some(data) = view.user_data(id) else {
        return;
    };
    let before = data.clone();
    let value = match toggle {
        Toggle::Played => {
            data.played = !data.played;
            // server resets the resume position both ways
            data.playback_position_ticks = 0;
            data.played_percentage = None;
            data.played
        }
        Toggle::Favorite => {
            data.is_favorite = !data.is_favorite;
            data.is_favorite
        }
    };
    cx.notify();
    let (api, id) = (api.clone(), id.to_string());
    cx.spawn(async move |this, cx| {
        let result = match toggle {
            Toggle::Played => api.set_played(&id, value).await,
            Toggle::Favorite => api.set_favorite(&id, value).await,
        };
        this.update(cx, |view, cx| {
            match result {
                Ok(()) => view.saved(&id, toggle, cx),
                Err(err) => {
                    if let Some(data) = view.user_data(&id) {
                        *data = before;
                    }
                    let what = match toggle {
                        Toggle::Played => "watched state",
                        Toggle::Favorite => "favorite",
                    };
                    view.set_error(format!("Could not update {what}: {err}").into());
                }
            }
            cx.notify();
        })
        .ok();
    })
    .detach();
}

/// Icon button for one `Toggle`; `selected` while the flag is set.
pub fn toggle_button(id: impl Into<ElementId>, toggle: Toggle, data: &UserData) -> Button {
    let (icon, on, label_on, label_off) = match toggle {
        Toggle::Played => (
            assets::IconName::CircleCheck,
            data.played,
            "Mark unwatched",
            "Mark watched",
        ),
        Toggle::Favorite => (
            assets::IconName::Heart,
            data.is_favorite,
            "Remove from favorites",
            "Add to favorites",
        ),
    };
    Button::new(id)
        .ghost()
        .icon(Icon::new(icon))
        .selected(on)
        .tooltip(if on { label_on } else { label_off })
}

/// Year · runtime-or-counts · age rating · ★ community · critics %, skipping missing parts.
pub fn meta_line(item: &Item, length: Option<String>, cx: &App) -> Option<AnyElement> {
    let theme = cx.theme();
    let mut parts: Vec<AnyElement> = Vec::new();
    if let Some(year) = item.production_year {
        parts.push(year.to_string().into_any_element());
    }
    if let Some(length) = length {
        parts.push(length.into_any_element());
    }
    if let Some(rating) = item.official_rating.clone() {
        parts.push(
            div()
                .px_1p5()
                .rounded_sm()
                .border_1()
                .border_color(theme.border)
                .text_xs()
                .child(rating)
                .into_any_element(),
        );
    }
    if let Some(score) = item.community_rating {
        parts.push(
            h_flex()
                .gap_1()
                .child(
                    Icon::new(assets::IconName::Star)
                        .small()
                        .text_color(theme.warning),
                )
                .child(format!("{score:.1}"))
                .into_any_element(),
        );
    }
    if let Some(score) = item.critic_rating {
        parts.push(format!("Critics {}%", score.round()).into_any_element());
    }
    (!parts.is_empty()).then(|| {
        h_flex()
            .gap_3()
            .items_center()
            .text_sm()
            .text_color(theme.muted_foreground)
            .children(parts)
            .into_any_element()
    })
}

/// Resolution/HDR/audio format badges; `None` when the item has no stream info.
pub fn media_tags(item: &Item) -> Option<AnyElement> {
    let tags = item.media_tags();
    (!tags.is_empty()).then(|| {
        h_flex()
            .gap_1p5()
            .flex_wrap()
            .children(
                tags.into_iter()
                    .map(|tag| Tag::secondary().outline().small().child(tag)),
            )
            .into_any_element()
    })
}

/// Dimmed backdrop art behind the top of a detail page, fading into the background.
/// Place first inside a `relative` page so content paints over it.
pub fn backdrop(api: &Api, item: &Item, cx: &App) -> Option<AnyElement> {
    let url = api.backdrop_url(item)?;
    let bg = cx.theme().background;
    Some(
        div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .h(px(BACKDROP_HEIGHT))
            // Cover scales past the box; only this clips it
            .overflow_hidden()
            .child(
                img(url)
                    .size_full()
                    .object_fit(ObjectFit::Cover)
                    .opacity(BACKDROP_OPACITY),
            )
            // left scrim: header text sits there
            .child(div().absolute().inset_0().bg(linear_gradient(
                90.,
                linear_color_stop(bg.opacity(0.7), 0.),
                linear_color_stop(bg.opacity(0.), 0.75),
            )))
            .child(div().absolute().inset_0().bg(linear_gradient(
                180.,
                linear_color_stop(bg.opacity(0.), 0.35),
                linear_color_stop(bg, 1.),
            )))
            .into_any_element(),
    )
}

const BACKDROP_HEIGHT: f32 = 460.;
const BACKDROP_OPACITY: f32 = 0.3;

pub fn genres_line(item: &Item, cx: &App) -> Option<AnyElement> {
    (!item.genres.is_empty()).then(|| {
        div()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child(item.genres.join(", "))
            .into_any_element()
    })
}

const CAST_LIMIT: usize = 12;
const CAST_TILE_WIDTH: f32 = 216.;
const PORTRAIT_SIZE: f32 = 44.;

/// Top billed actors as a wrapping grid of compact tiles: round portrait (initials
/// when none), name, character.
pub fn cast_row(api: &Api, item: &Item, cx: &App) -> Option<AnyElement> {
    let theme = cx.theme();
    let actors: Vec<AnyElement> = item
        .people
        .iter()
        .filter(|p| p.kind.as_deref() == Some("Actor"))
        .take(CAST_LIMIT)
        .map(|person| {
            h_flex()
                .w(px(CAST_TILE_WIDTH))
                .gap_3()
                .child(portrait(api.person_image_url(person), &person.name, cx))
                .child(
                    v_flex()
                        .min_w_0()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .truncate()
                                .child(person.name.clone()),
                        )
                        .children(person.role.clone().filter(|r| !r.is_empty()).map(|role| {
                            div()
                                .text_xs()
                                .truncate()
                                .text_color(theme.muted_foreground)
                                .child(role)
                        })),
                )
                .into_any_element()
        })
        .collect();
    if actors.is_empty() {
        return None;
    }
    Some(
        v_flex()
            .gap_4()
            .child(
                div()
                    .text_xs()
                    .font_family(theme.mono_font_family.clone())
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.muted_foreground)
                    .child("CAST"),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_x_6()
                    .gap_y_4()
                    .children(actors),
            )
            .into_any_element(),
    )
}

/// Round person photo; initials on a muted circle when missing or failing to load.
/// Rounded on the `img` itself: a wrapper's `overflow_hidden` clips only to a rectangle.
fn portrait(url: Option<String>, name: &str, cx: &App) -> AnyElement {
    let (muted, muted_fg) = (cx.theme().muted, cx.theme().muted_foreground);
    let initials: SharedString = name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
        .into();
    let size = px(PORTRAIT_SIZE);
    let placeholder = move || {
        div()
            .size(size)
            .flex_shrink_0()
            .rounded_full()
            .bg(muted)
            .flex()
            .items_center()
            .justify_center()
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .text_color(muted_fg)
            .child(initials.clone())
            .into_any_element()
    };
    match url {
        Some(url) => img(url)
            .size(size)
            .flex_shrink_0()
            .rounded_full()
            .object_fit(ObjectFit::Cover)
            .with_fallback(placeholder)
            .into_any_element(),
        None => placeholder(),
    }
}
