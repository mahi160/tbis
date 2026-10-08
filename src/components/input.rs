//! Text inputs, overriding gpui-kit Input's own sizing with the theme's
//! `design.input` recipe (vega: `h-9 rounded-md border px-2.5 py-1 text-sm shadow-xs`,
//! `dark:bg-input/30`). gpui-kit keeps the border color and the focus ring.

use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{Icon, IconName, v_flex};
use gpui_kit::*;

use crate::design::{color, icon, input_bg, radius, shadow, sp, text, tokens};

/// Input `height` spacing units tall.
fn input(state: &Entity<InputState>, height: f32, cx: &App) -> Input {
    let recipe = tokens().input;
    // Input::h is its own multi-line height; Styled::h sets the box
    Styled::h(Input::new(state), text::fit(height, text::sm(), recipe.py))
        .px(sp(recipe.px))
        .py(sp(recipe.py))
        .rounded(radius::md())
        .bg(input_bg(cx))
        .shadow(shadow::xs())
        .text_size(text::sm())
}

/// Field: label (`text-sm font-medium leading-none`) above the input.
pub fn text_field(
    label: &'static str,
    state: &Entity<InputState>,
    disabled: bool,
    cx: &App,
) -> Div {
    let recipe = tokens().input;
    v_flex()
        .gap(sp(recipe.label_gap))
        .child(
            div()
                .text_size(text::sm())
                .font_weight(FontWeight::MEDIUM)
                .line_height(relative(1.))
                .child(label),
        )
        .child(input(state, recipe.height, cx).disabled(disabled))
}

/// Small clearable input with a search icon, for the title bar.
pub fn search_input(state: &Entity<InputState>, cx: &App) -> Div {
    div().w(sp(50.)).child(
        input(state, tokens().input.height_sm, cx)
            // outer 3px ring clips at window edge; field only opens while searching, so ring-tinted border marks focus
            .focus_bordered(false)
            .border_color(color::ring(cx))
            .cleanable(true)
            .prefix(Icon::new(IconName::Search).size(icon::default())),
    )
}
