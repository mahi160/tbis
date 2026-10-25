//! Text inputs, overriding gpui-kit Input's own sizing: recipe height, item-gap
//! padding, the one radius, body text, `dark:bg-input/30`. gpui-kit keeps the border
//! color and the focus ring.

use gpui_kit::component::input::{Input, InputState, Textarea, TextareaState};
use gpui_kit::component::{Icon, IconName, v_flex};
use gpui_kit::*;

use crate::design::{color, font, gap, icon, input_bg, radius, shadow, sp, text, tokens};

/// Input `height` spacing units tall.
fn input(state: &Entity<InputState>, height: f32, cx: &App) -> Input {
    // Input::h is its own multi-line height; Styled::h sets the box
    Styled::h(
        Input::new(state),
        text::fit(height, text::body(), gap::tight()),
    )
    .px(gap::item())
    .py(gap::tight())
    .rounded(radius::base())
    .bg(input_bg(cx))
    .shadow(shadow::xs())
    .text_size(text::body())
}

/// Field: label (medium weight, `leading-none`) above the input.
pub fn text_field(
    label: &'static str,
    state: &Entity<InputState>,
    disabled: bool,
    cx: &App,
) -> Div {
    v_flex()
        .gap(gap::item())
        .child(
            div()
                .text_size(text::body())
                .font_weight(FontWeight::MEDIUM)
                .line_height(relative(1.))
                .child(label),
        )
        .child(input(state, tokens().input.height, cx).disabled(disabled))
}

/// Multi-line monospace field for config-file text; its height comes from the state's
/// `rows`/`auto_grow`.
pub fn code_area(state: &Entity<TextareaState>, cx: &App) -> Textarea {
    Textarea::new(state)
        .px(gap::item())
        .py(gap::tight())
        .rounded(radius::base())
        .bg(input_bg(cx))
        .shadow(shadow::xs())
        .text_size(text::body())
        .font_family(font::mono(cx))
}

/// Small clearable input with a search icon, filling its parent's width.
pub fn search_box(state: &Entity<InputState>, cx: &App) -> Input {
    input(state, tokens().input.height, cx)
        .cleanable(true)
        .prefix(Icon::new(IconName::Search).size(icon::default()))
}

/// `search_box` for the title bar.
pub fn search_input(state: &Entity<InputState>, cx: &App) -> Div {
    div().w(sp(50.)).child(
        search_box(state, cx)
            // outer 3px ring clips at window edge; field only opens while searching, so ring-tinted border marks focus
            .focus_bordered(false)
            .border_color(color::ring(cx)),
    )
}
