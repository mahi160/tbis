//! Outline badge (gpui-kit Tag) for ratings and media formats: border, meta text,
//! medium weight, the one radius, recipe height, item-gap padding.

use gpui_kit::component::tag::Tag;
use gpui_kit::component::{Icon, h_flex};
use gpui_kit::*;

use crate::design::{color, gap, icon, radius, text, tokens};

pub fn pill(label: impl Into<SharedString>, cx: &App) -> Tag {
    Styled::h(
        Tag::secondary().outline().rounded(radius::base()),
        text::fit(tokens().badge.height, text::meta(), px(0.)),
    )
    .px(gap::item())
    .py(px(0.))
    .text_size(text::meta())
    .font_weight(FontWeight::MEDIUM)
    .border_color(color::border(cx))
    .text_color(color::foreground(cx))
    .child(label.into())
}

/// Warning-colored star with a one-decimal score.
pub fn star_rating(score: f32, cx: &App) -> Div {
    h_flex()
        .gap(gap::tight())
        .child(
            Icon::new(assets::IconName::Star)
                .size(icon::sm())
                .text_color(color::warning(cx)),
        )
        .child(format!("{score:.1}"))
}
