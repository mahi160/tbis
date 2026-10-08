//! Detail-page building blocks shared by Movie and Series pages: header, meta line,
//! genres, media tags, backdrop, cast.

use gpui_kit::component::{IconName, h_flex, v_flex};
use gpui_kit::*;

use super::avatar::avatar;
use super::button::icon_button;
use super::typography::caption;
use super::{OnClick, art, tag, typography};
use crate::design::{color, gap, sp, text};
use crate::jellyfin::{Api, Item};

/// Back button in its own row, then `image` (URL, width, height), then `body` (title/meta/overview/actions).
pub fn detail_header(
    image: (Option<String>, Pixels, Pixels),
    back: OnClick,
    body: impl IntoElement,
    cx: &App,
) -> impl IntoElement {
    let (url, width, height) = image;
    v_flex()
        .gap(gap::item())
        .items_start()
        .child(icon_button("detail-back", IconName::ArrowLeft, cx).on_click(back))
        .child(
            h_flex()
                .w_full()
                .gap(gap::group())
                .items_start()
                .child(art::thumb(url, width, height, cx))
                .child(body),
        )
}

/// Column beside the header image holding title, meta and actions.
pub fn header_body() -> Div {
    v_flex().flex_1().min_w_0()
}

/// Year · runtime-or-counts · age rating · ★ community · critics %, skipping missing parts.
pub fn meta_line(item: &Item, length: Option<String>, cx: &App) -> Option<AnyElement> {
    let muted_fg = color::muted_foreground(cx);
    let mut parts: Vec<AnyElement> = Vec::new();
    if let Some(year) = item.production_year {
        parts.push(year.to_string().into_any_element());
    }
    if let Some(length) = length {
        parts.push(length.into_any_element());
    }
    if let Some(rating) = item.official_rating.clone() {
        parts.push(tag::pill(rating, cx).into_any_element());
    }
    if let Some(score) = item.community_rating {
        parts.push(tag::star_rating(score, cx).into_any_element());
    }
    if let Some(score) = item.critic_rating {
        parts.push(format!("Critics {}%", score.round()).into_any_element());
    }
    (!parts.is_empty()).then(|| {
        h_flex()
            .gap(gap::item())
            .items_center()
            .text_size(text::body())
            .text_color(muted_fg)
            .children(parts)
            .into_any_element()
    })
}

/// Resolution/HDR/audio format badges; `None` when the item has no stream info.
pub fn media_tags(item: &Item, cx: &App) -> Option<AnyElement> {
    let tags = item.media_tags();
    (!tags.is_empty()).then(|| {
        h_flex()
            .gap(gap::tight())
            .flex_wrap()
            .children(tags.into_iter().map(|t| tag::pill(t, cx)))
            .into_any_element()
    })
}

pub fn genres_line(item: &Item, cx: &App) -> Option<AnyElement> {
    (!item.genres.is_empty()).then(|| caption(item.genres.join(", "), cx).into_any_element())
}

const CAST_LIMIT: usize = 12;

/// Top billed actors as a wrapping grid of compact tiles: round portrait (initials
/// when none), name, character.
pub fn cast_row(api: &Api, item: &Item, cx: &App) -> Option<AnyElement> {
    let muted_fg = color::muted_foreground(cx);
    let actors: Vec<AnyElement> = item
        .people
        .iter()
        .filter(|p| p.kind.as_deref() == Some("Actor"))
        .take(CAST_LIMIT)
        .map(|person| {
            h_flex()
                .w(sp(54.))
                .gap(gap::item())
                .child(avatar(
                    api.person_image_url(person),
                    person.name.clone(),
                    sp(crate::design::tokens().avatar.lg),
                    cx,
                ))
                .child(
                    v_flex()
                        .min_w_0()
                        .child(
                            div()
                                .text_size(text::body())
                                .font_weight(FontWeight::MEDIUM)
                                .truncate()
                                .child(person.name.clone()),
                        )
                        .children(person.role.clone().filter(|r| !r.is_empty()).map(|role| {
                            div()
                                .text_size(text::meta())
                                .truncate()
                                .text_color(muted_fg)
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
            .gap(gap::group())
            .child(typography::section_heading("Cast"))
            .child(div().flex().flex_wrap().gap(gap::group()).children(actors))
            .into_any_element(),
    )
}
