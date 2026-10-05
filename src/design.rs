//! Design tokens: the single source UI code reads for every visual value. Each theme in
//! `assets/themes/tbis.json` carries its whole look: gpui-kit's colors and font
//! families, plus a `"design"` object (parsed into [`Design`]) with the spacing unit,
//! type scale, one base radius, shadow strength, and each component's recipe (button,
//! input, card, badge, item, avatar, key cap sizes). Missing keys fall back to shadcn
//! vega. `theme::apply` installs the active `Design` and derives gpui-kit's own font
//! size and radii from it. Only this file reads `cx.theme()` for visuals.
//!
//! Lengths are multiples of the theme's spacing unit (`sp(4.)` is 16px at the default
//! 4px unit, like Tailwind's `p-4`); components use `sp(n)` for their own dimensions.

use std::sync::RwLock;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;
use serde::Deserialize;

/// A theme's non-color look, from its `"design"` object. Lengths are spacing units
/// except `spacing`, `text` and `radius` (px).
#[derive(Clone, Copy, Deserialize)]
#[serde(default)]
pub struct Design {
    pub spacing: f32,
    pub page: f32,
    pub text: TextScale,
    /// Base corner radius (shadcn's `--radius`); the radius scale derives from it.
    pub radius: f32,
    pub shadow_opacity: f32,
    pub button: ButtonRecipe,
    pub input: InputRecipe,
    pub card: CardRecipe,
    pub badge: BadgeRecipe,
    pub item: ItemRecipe,
    pub avatar: AvatarRecipe,
    pub kbd: KbdRecipe,
    pub icon: IconRecipe,
    pub progress_height: f32,
    /// gpui-kit size of the tab strips and the Settings page.
    pub tabs: KitSize,
    pub settings: KitSize,
}

/// Icon sizes in spacing units.
#[derive(Clone, Copy, Deserialize)]
#[serde(default)]
pub struct IconRecipe {
    pub sm: f32,
    pub default: f32,
}

/// A gpui-kit size step, for components themed only through gpui-kit's own sizes.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KitSize {
    Xs,
    Sm,
    Md,
    Lg,
}

impl KitSize {
    pub fn kit(self) -> gpui_kit::component::Size {
        use gpui_kit::component::Size;
        match self {
            Self::Xs => Size::XSmall,
            Self::Sm => Size::Small,
            Self::Md => Size::Medium,
            Self::Lg => Size::Large,
        }
    }
}

#[derive(Clone, Copy, Deserialize)]
#[serde(default)]
pub struct TextScale {
    pub xs: f32,
    pub sm: f32,
    pub base: f32,
    pub lg: f32,
    pub xl: f32,
    #[serde(rename = "2xl")]
    pub x2l: f32,
    #[serde(rename = "3xl")]
    pub x3l: f32,
}

/// `[height, padding-x]` per text size; icon-only buttons are squares.
#[derive(Clone, Copy, Deserialize)]
#[serde(default)]
pub struct ButtonRecipe {
    pub xs: [f32; 2],
    pub sm: [f32; 2],
    pub default: [f32; 2],
    pub lg: [f32; 2],
    pub icon_xs: f32,
    pub icon_sm: f32,
    pub icon: f32,
    /// gpui-kit size whose text the sm/default/lg buttons use: `md` is text-sm,
    /// `xs` is text-xs.
    pub text: KitSize,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(default)]
pub struct InputRecipe {
    pub height: f32,
    pub height_sm: f32,
    pub px: f32,
    pub py: f32,
    /// Gap between a field's label and its input.
    pub label_gap: f32,
}

/// Padding and gap of a card, default and small.
#[derive(Clone, Copy, Deserialize)]
#[serde(default)]
pub struct CardRecipe {
    pub spacing: f32,
    pub spacing_sm: f32,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(default)]
pub struct BadgeRecipe {
    pub height: f32,
    pub px: f32,
    pub py: f32,
}

/// `[gap, padding-x, padding-y]` of list rows.
#[derive(Clone, Copy, Deserialize)]
#[serde(default)]
pub struct ItemRecipe {
    pub sm: [f32; 3],
    pub xs: [f32; 3],
}

#[derive(Clone, Copy, Deserialize)]
#[serde(default)]
pub struct AvatarRecipe {
    pub default: f32,
    pub lg: f32,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(default)]
pub struct KbdRecipe {
    pub size: f32,
    pub px: f32,
}

/// shadcn vega.
impl Default for Design {
    fn default() -> Self {
        Self {
            spacing: 4.,
            page: 8.,
            text: TextScale {
                xs: 12.,
                sm: 14.,
                base: 16.,
                lg: 18.,
                xl: 20.,
                x2l: 24.,
                x3l: 30.,
            },
            radius: 10.,
            shadow_opacity: 0.1,
            button: ButtonRecipe {
                xs: [6., 2.],
                sm: [8., 2.5],
                default: [9., 2.5],
                lg: [10., 2.5],
                icon_xs: 6.,
                icon_sm: 8.,
                icon: 9.,
                text: KitSize::Md,
            },
            input: InputRecipe {
                height: 9.,
                height_sm: 8.,
                px: 2.5,
                py: 1.,
                label_gap: 3.,
            },
            card: CardRecipe {
                spacing: 6.,
                spacing_sm: 4.,
            },
            badge: BadgeRecipe {
                height: 5.,
                px: 2.,
                py: 0.5,
            },
            item: ItemRecipe {
                sm: [2.5, 3., 2.5],
                xs: [2., 2.5, 2.],
            },
            avatar: AvatarRecipe {
                default: 8.,
                lg: 10.,
            },
            kbd: KbdRecipe { size: 5., px: 1. },
            icon: IconRecipe {
                sm: 3.,
                default: 4.,
            },
            progress_height: 1.5,
            // segmented tabs at gpui-kit Large are vega's h-9 list
            tabs: KitSize::Lg,
            settings: KitSize::Sm,
        }
    }
}

macro_rules! default_parts {
    ($($ty:ident . $field:ident),* $(,)?) => {
        $(impl Default for $ty {
            fn default() -> Self {
                Design::default().$field
            }
        })*
    };
}

default_parts!(
    TextScale.text,
    ButtonRecipe.button,
    InputRecipe.input,
    CardRecipe.card,
    BadgeRecipe.badge,
    ItemRecipe.item,
    AvatarRecipe.avatar,
    KbdRecipe.kbd,
    IconRecipe.icon,
);

static ACTIVE: RwLock<Option<Design>> = RwLock::new(None);

/// Installs the active theme's design; `theme::apply` calls this before redrawing.
pub fn set(design: Design) {
    *ACTIVE.write().unwrap_or_else(|e| e.into_inner()) = Some(design);
}

/// The active theme's design (vega before any theme is applied).
pub fn tokens() -> Design {
    ACTIVE
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .unwrap_or_default()
}

/// `n` spacing units.
pub fn sp(n: f32) -> Pixels {
    px(n * tokens().spacing)
}

/// Page gutter on every screen.
pub fn page() -> Pixels {
    sp(tokens().page)
}

macro_rules! theme_colors {
    ($($name:ident),* $(,)?) => {
        /// Theme colors, from the active theme.
        pub mod color {
            use super::*;
            $(pub fn $name(cx: &App) -> Hsla { cx.theme().$name })*
        }
    };
}

theme_colors!(
    background,
    foreground,
    muted,
    muted_foreground,
    popover,
    primary,
    primary_foreground,
    border,
    danger,
    warning,
);

/// Input fill: shadcn's `bg-transparent dark:bg-input/30`.
pub fn input_bg(cx: &App) -> Hsla {
    if cx.theme().is_dark() {
        cx.theme().input.opacity(0.3)
    } else {
        overlay::CLEAR
    }
}

/// Font families, from the active theme.
pub mod font {
    use super::*;

    pub fn mono(cx: &App) -> SharedString {
        cx.theme().mono_font_family.clone()
    }
}

/// Type scale (Tailwind `text-xs` .. `text-3xl`), from the theme.
pub mod text {
    use super::{px, tokens, Pixels};

    macro_rules! steps {
        ($($name:ident),*) => {
            $(pub fn $name() -> Pixels { px(tokens().text.$name) })*
        };
    }

    steps!(xs, sm, base, lg, xl, x2l, x3l);
}

/// Icon sizes, from the theme's `icon` recipe.
pub mod icon {
    use super::{sp, tokens, Pixels};

    pub fn sm() -> Pixels {
        sp(tokens().icon.sm)
    }

    pub fn default() -> Pixels {
        sp(tokens().icon.default)
    }
}

/// Corner radii: shadcn's scale off the theme's one base `radius` (`--radius`).
/// `theme::apply` hands gpui-kit `md` (controls) and `lg` (dialogs, popovers).
pub mod radius {
    use super::{Pixels, px, tokens};

    fn scaled(ratio: f32) -> Pixels {
        px(tokens().radius * ratio)
    }

    /// Small bits: key caps.
    pub fn sm() -> Pixels {
        scaled(0.6)
    }

    /// Controls, list rows, small overlays.
    pub fn md() -> Pixels {
        scaled(0.8)
    }

    /// Images and art tiles.
    pub fn lg() -> Pixels {
        scaled(1.)
    }

    /// Cards, panels, dialogs.
    pub fn xl() -> Pixels {
        scaled(1.4)
    }

    /// Pills and round buttons; square when the theme's radius is 0.
    pub fn full() -> Pixels {
        if tokens().radius == 0. { px(0.) } else { px(9999.) }
    }
}

/// Layout sizes shared by more than one file, in spacing units.
pub mod size {
    use super::{Pixels, px, sp, tokens};

    /// gpui-kit TitleBar's own left padding (traffic lights on macOS).
    pub const TITLE_BAR_INSET: Pixels = px(if cfg!(target_os = "macos") { 80. } else { 12. });

    macro_rules! units {
        ($($name:ident = $n:expr),* $(,)?) => {
            $(pub fn $name() -> Pixels { sp($n) })*
        };
    }

    units!(
        poster_w = 44.,
        poster_min_w = 37.5,
        wide_w = 72.,
        header_poster_w = 40.,
        header_poster_h = 60.,
        header_poster_sm_w = 30.,
        header_poster_sm_h = 45.,
        header_still_w = 80.,
        header_still_h = 45.,
        form_w = 90.,
        list_max_w = 120.,
        prose_max_w = 140.,
        dialog_w = 112., // vega max-w-md
        chart_h = 45.,
        up_next_w = 80.,
    );
    /// Scrub-preview width in px; the Player scales sprite tiles to it.
    pub fn trickplay_w() -> f32 {
        40. * tokens().spacing
    }
}

/// Window geometry (OS window, not themed).
pub mod window {
    use gpui_kit::{Pixels, Point, Size, point, px, size};

    pub const DEFAULT: Size<Pixels> = size(px(1200.), px(800.));
    pub const MIN: Size<Pixels> = size(px(800.), px(560.));
    pub const TRAFFIC_LIGHTS: Point<Pixels> = point(px(13.), px(13.));
}

/// Opacity levels.
pub mod opacity {
    pub const FAINT: f32 = 0.3;
    pub const HALF: f32 = 0.5;
}

/// Fixed tints for chrome over live video or images (Tailwind `black/60`, `white/75`).
/// Not themed on purpose: they sit over arbitrary frames, not the app's background.
pub mod overlay {
    use gpui_kit::{Hsla, hsla};

    pub const fn black(alpha: f32) -> Hsla {
        hsla(0., 0., 0., alpha)
    }

    pub const fn white(alpha: f32) -> Hsla {
        hsla(0., 0., 1., alpha)
    }

    pub const CLEAR: Hsla = black(0.);
    pub const WHITE: Hsla = white(1.);
    pub const CARD: Hsla = black(0.8);
    pub const TEXT_FAINT: Hsla = white(0.5);
    pub const TEXT_MUTED: Hsla = white(0.75);
}

/// Tailwind elevation scale (`shadow-xs` / `shadow-sm` / `shadow-lg`), its strength
/// from the theme's `shadow_opacity`.
pub mod shadow {
    use super::overlay::black;
    use super::tokens;
    use gpui_kit::{BoxShadow, px};

    fn layer(y: f32, blur: f32, spread: f32, strength: f32) -> BoxShadow {
        BoxShadow::new(px(0.), px(y), black(tokens().shadow_opacity * strength))
            .blur_radius(px(blur))
            .spread_radius(px(spread))
    }

    /// Inputs, cards.
    pub fn xs() -> Vec<BoxShadow> {
        vec![layer(1., 2., 0., 0.5)]
    }

    /// Resting tiles.
    pub fn sm() -> Vec<BoxShadow> {
        vec![layer(1., 3., 0., 1.), layer(1., 2., -1., 1.)]
    }

    /// Raised: hovered tiles, floating previews.
    pub fn lg() -> Vec<BoxShadow> {
        vec![layer(10., 15., -3., 1.), layer(4., 6., -4., 1.)]
    }
}
