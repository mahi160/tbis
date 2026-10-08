//! App buttons in shadcn's vega style, built on gpui-kit Button with its sizing
//! overridden. Every button in the app starts here, so one edit restyles them all.
//! Like shadcn's single `Button`, icon-only buttons are presets here too.
//!
//! One radius, body text, medium weight; heights per size come from the theme's
//! `design.button` recipe, horizontal padding is the item gap.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{Disableable as _, Icon, Selectable as _, Sizable as _, Size};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::design::{color, font, gap, input_bg, overlay, radius, sp, text, tokens};

/// Vega sizes; chain after a preset. Default size needs no call.
pub trait VegaSize: Sized {
    fn xs(self) -> Self;
    fn sm(self) -> Self;
    fn lg(self) -> Self;
    fn icon_xs(self) -> Self;
}

/// State setters for ghost presets: the preset's foreground override would otherwise
/// mask gpui-kit's disabled/selected text colors.
pub trait GhostState: Sized {
    fn ghost_disabled(self, disabled: bool, cx: &App) -> Self;
    fn ghost_selected(self, selected: bool, cx: &App) -> Self;
}

impl GhostState for Button {
    fn ghost_disabled(self, disabled: bool, cx: &App) -> Self {
        self.disabled(disabled)
            .when(disabled, |b| b.text_color(color::muted_foreground(cx).opacity(0.5)))
    }
    fn ghost_selected(self, selected: bool, cx: &App) -> Self {
        self.selected(selected)
            .when(selected, |b| b.text_color(color::accent_foreground(cx)))
    }
}

/// Ghost in vega: inherits `foreground`. gpui-kit uses `secondary_foreground`, which
/// is meant for the secondary fill and can vanish on the page background.
fn ghost(button: Button, cx: &App) -> Button {
    button.ghost().text_color(color::foreground(cx))
}

/// Text button `h` spacing units tall. gpui-kit Medium draws text-sm, which
/// `theme::apply` makes the body size, so every size shares it.
fn text_size(button: Button, h: f32) -> Button {
    Styled::h(button.with_size(Size::Medium), text::fit(h, text::body(), px(0.))).px(gap::item())
}

/// Icon-only button, `n` units square.
fn square(button: Button, n: f32) -> Button {
    Styled::size(button, sp(n))
}

impl VegaSize for Button {
    fn xs(self) -> Self {
        text_size(self, tokens().button.xs)
    }
    fn sm(self) -> Self {
        text_size(self, tokens().button.sm)
    }
    fn lg(self) -> Self {
        text_size(self, tokens().button.lg)
    }
    fn icon_xs(self) -> Self {
        square(self.with_size(Size::XSmall), tokens().button.icon_xs)
    }
}

/// Every button starts here: gpui-kit Button, medium weight, no size yet.
fn plain(id: impl Into<ElementId>) -> Button {
    Button::new(id).font_weight(FontWeight::MEDIUM)
}

/// Text-button base: the recipe's default size. Icon-only buttons skip it, since its
/// padding would squeeze an icon out of a small square.
fn base(id: impl Into<ElementId>) -> Button {
    text_size(plain(id), tokens().button.default)
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
pub fn ghost_button(id: impl Into<ElementId>, label: impl Into<SharedString>, cx: &App) -> Button {
    ghost(base(id), cx).label(label.into())
}

/// Round ghost text button in mono type, e.g. the Player's speed.
pub fn round_ghost_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    cx: &App,
) -> Button {
    ghost_button(id, label, cx)
        .rounded(radius::base())
        .text_color(overlay::WHITE)
        .font_family(font::mono(cx))
        .text_size(text::meta())
}

/// Ghost icon-only button at the recipe's icon size; chain `.icon_xs()`.
pub fn icon_button(id: impl Into<ElementId>, icon: impl Into<Icon>, cx: &App) -> Button {
    ghost(plain_icon(id, icon), cx)
}

/// `icon_button` on the surface fill, the twin of `avatar_button` in the title bar.
pub fn surface_icon_button(id: impl Into<ElementId>, icon: impl Into<Icon>, cx: &App) -> Button {
    icon_button(id, icon, cx).bg(color::muted(cx))
}

fn plain_icon(id: impl Into<ElementId>, icon: impl Into<Icon>) -> Button {
    square(plain(id).ghost().icon(icon), tokens().button.icon)
}

/// Round ghost icon button, for the Player's bars.
pub fn round_icon(id: impl Into<ElementId>, icon: impl Into<Icon>) -> Button {
    // white: bars sit on video, never on theme background
    plain_icon(id, icon).rounded(radius::base()).text_color(overlay::WHITE)
}

/// `round_icon` with an SVG from `assets/icons`.
pub fn round_icon_button(id: impl Into<ElementId>, path: &'static str) -> Button {
    round_icon(id, Icon::empty().path(path))
}

/// Round avatar (initials of `name`) as a ghost button at the recipe's avatar size.
pub fn avatar_button(id: impl Into<ElementId>, name: impl Into<SharedString>, cx: &App) -> Button {
    let size = tokens().avatar.default;
    square(ghost(plain(id).with_size(Size::Small), cx), size)
        .p(px(0.))
        .rounded(radius::base())
        .child(super::avatar::avatar(None, name, sp(size), cx))
}

/// Select trigger: the current choice in a small outline button with a caret.
pub fn select_button(id: impl Into<ElementId>, label: impl Into<SharedString>, cx: &App) -> Button {
    outline_button(id, label, cx).sm().dropdown_caret(true)
}

/// Small ghost button with a caret, opening a dropdown.
pub fn menu_button(id: impl Into<ElementId>, cx: &App) -> Button {
    ghost(base(id), cx).sm().dropdown_caret(true)
}
