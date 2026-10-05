//! App buttons in shadcn's vega style, built on gpui-kit Button with its sizing
//! overridden. Every button in the app starts here, so one edit restyles them all.
//! Like shadcn's single `Button`, icon-only buttons are presets here too.
//!
//! `rounded-md text-sm font-medium`; heights and padding per size come from the theme's
//! `design.button` recipe (vega: xs `h-6 px-2`, sm `h-8 px-2.5`, default `h-9 px-2.5`,
//! lg `h-10 px-2.5`; icon-only `size-6` / `size-8` / `size-9`).

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{Icon, Sizable as _, Size};
use gpui_kit::*;

use crate::design::{font, input_bg, radius, sp, text, tokens};

/// Vega sizes; chain after a preset. Default size needs no call.
pub trait VegaSize: Sized {
    fn xs(self) -> Self;
    fn sm(self) -> Self;
    fn lg(self) -> Self;
    fn icon_xs(self) -> Self;
    fn icon_sm(self) -> Self;
}

/// Text button sized by a recipe `[height, padding-x]` in spacing units; `kit` picks
/// gpui-kit's text/icon size.
fn text_size(button: Button, [h, px]: [f32; 2], kit: Size) -> Button {
    Styled::h(button.with_size(kit), sp(h)).px(sp(px))
}

/// Icon-only button, `n` units square.
fn square(button: Button, n: f32) -> Button {
    Styled::size(button, sp(n))
}

impl VegaSize for Button {
    fn xs(self) -> Self {
        text_size(self, tokens().button.xs, Size::XSmall)
    }
    fn sm(self) -> Self {
        text_size(self, tokens().button.sm, tokens().button.text.kit())
    }
    // lg keeps text-sm; gpui-kit Large would switch to text-base
    fn lg(self) -> Self {
        text_size(self, tokens().button.lg, tokens().button.text.kit())
    }
    fn icon_xs(self) -> Self {
        square(self.with_size(Size::XSmall), tokens().button.icon_xs)
    }
    fn icon_sm(self) -> Self {
        square(self.with_size(Size::Small), tokens().button.icon_sm)
    }
}

/// Base every preset builds on: vega default size, medium weight.
fn base(id: impl Into<ElementId>) -> Button {
    text_size(Button::new(id), tokens().button.default, tokens().button.text.kit())
        .font_weight(FontWeight::MEDIUM)
}

/// Neutral button.
pub fn button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Button {
    base(id).label(label.into())
}

/// Main action of a screen.
pub fn primary_button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Button {
    base(id).primary().label(label.into())
}

/// Secondary action beside a primary one; vega outline fills `dark:bg-input/30`.
pub fn outline_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    cx: &App,
) -> Button {
    base(id).outline().bg(input_bg(cx)).label(label.into())
}

/// Low-emphasis text action.
pub fn ghost_button(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Button {
    base(id).ghost().label(label.into())
}

/// Round ghost text button in mono type, e.g. the Player's speed.
pub fn round_ghost_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    cx: &App,
) -> Button {
    ghost_button(id, label)
        .rounded(radius::full())
        .font_family(font::mono(cx))
        .text_size(text::xs())
}

/// Ghost icon-only button at the recipe's icon size; chain `.icon_sm()` / `.icon_xs()`.
pub fn icon_button(id: impl Into<ElementId>, icon: impl Into<Icon>) -> Button {
    square(base(id).ghost().icon(icon), tokens().button.icon)
}

/// Round ghost icon button, for the Player's bars.
pub fn round_icon(id: impl Into<ElementId>, icon: impl Into<Icon>) -> Button {
    icon_button(id, icon).rounded(radius::full())
}

/// `round_icon` with an SVG from `assets/icons`.
pub fn round_icon_button(id: impl Into<ElementId>, path: &'static str) -> Button {
    round_icon(id, Icon::empty().path(path))
}

/// Round avatar (initials of `name`) as a ghost button at the recipe's avatar size.
pub fn avatar_button(id: impl Into<ElementId>, name: impl Into<SharedString>, cx: &App) -> Button {
    let size = tokens().avatar.default;
    square(base(id).ghost().with_size(Size::Small), size)
        .p(px(0.))
        .rounded(radius::full())
        .child(super::avatar::avatar(None, name, sp(size), cx))
}

/// Small ghost button with a caret, opening a dropdown.
pub fn menu_button(id: impl Into<ElementId>) -> Button {
    base(id).ghost().sm().dropdown_caret(true)
}
