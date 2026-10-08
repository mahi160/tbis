//! Outline badge (gpui-kit Tag) for ratings and media formats: `rounded-4xl border
//! text-xs font-medium border-border text-foreground`, height and padding from the
//! theme's `design.badge` recipe (vega: `h-5 px-2 py-0.5`).

use gpui_kit::component::tag::Tag;
use gpui_kit::component::{Icon, h_flex};
use gpui_kit::*;

use crate::design::{color, icon, radius, sp, text, tokens};

pub fn pill(label: impl Into<SharedString>, cx: &App) -> Tag {
    let recipe = tokens().badge;
    Styled::h(
        Tag::secondary().outline().rounded(radius::full()),
        text::fit(recipe.height, text::xs(), recipe.py),
    )
        .px(sp(recipe.px))
        .py(sp(recipe.py))
        .text_size(text::xs())
        .font_weight(FontWeight::MEDIUM)
        .border_color(color::border(cx))
        .text_color(color::foreground(cx))
        .child(label.into())
}

/// Warning-colored star with a one-decimal score.
pub fn star_rating(score: f32, cx: &App) -> Div {
    h_flex()
        .gap(sp(1.))
        .child(
            Icon::new(assets::IconName::Star)
                .size(icon::sm())
                .text_color(color::warning(cx)),
        )
        .child(format!("{score:.1}"))
}
