//! Design tokens: the single source UI code reads for every visual value. Each theme in
//! `assets/themes/tbis.json` sets only colors and font families; everything else is
//! the constant [`tokens`], the same for every theme. The system is small on purpose:
//! three type sizes (`text`), one corner radius (`radius`), three gaps (`gap`), and
//! component heights. `theme::apply` derives gpui-kit's own font size and radii from
//! it. Only this file reads `cx.theme()` for visuals.
//!
//! Spacing between and inside things is always a `gap`. Fixed dimensions (control
//! heights, poster widths) are multiples of the spacing unit: `sp(n)`.

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

/// The non-color look, shared by every theme. `spacing`, `text`, `radius`
/// and `gap` are px; recipe sizes are spacing units.
#[derive(Clone, Copy)]
pub struct Design {
    pub spacing: f32,
    pub text: TextScale,
    /// The one corner radius, for everything.
    pub radius: f32,
    pub gap: Gaps,
    pub shadow_opacity: f32,
    pub button: ButtonRecipe,
    pub input: InputRecipe,
    pub badge: BadgeRecipe,
    pub avatar: AvatarRecipe,
    pub kbd: KbdRecipe,
    pub icon: IconRecipe,
    pub progress_height: f32,
    /// gpui-kit size of the tab strips.
    pub tabs: gpui_kit::component::Size,
}

/// The three type sizes.
#[derive(Clone, Copy)]
pub struct TextScale {
    /// Secondary details: years, episode labels, badges, descriptions.
    pub meta: f32,
    /// Everything else: body, controls, rows, card titles.
    pub body: f32,
    /// Page and section headings.
    pub title: f32,
}

/// The three gaps, px.
#[derive(Clone, Copy)]
pub struct Gaps {
    /// Inside a control: icon to label, title to its meta line.
    pub tight: f32,
    /// Between related items, and padding inside controls.
    pub item: f32,
    /// Between groups and sections.
    pub group: f32,
    /// Window padding: the gutter around every page.
    pub page: f32,
}

/// Icon sizes in spacing units.
#[derive(Clone, Copy)]
pub struct IconRecipe {
    pub sm: f32,
    pub default: f32,
}

/// Heights per button size; icon-only buttons are squares.
#[derive(Clone, Copy)]
pub struct ButtonRecipe {
    pub xs: f32,
    pub sm: f32,
    pub default: f32,
    pub lg: f32,
    pub icon_xs: f32,
    pub icon: f32,
}

#[derive(Clone, Copy)]
pub struct InputRecipe {
    pub height: f32,
}

#[derive(Clone, Copy)]
pub struct BadgeRecipe {
    pub height: f32,
}

#[derive(Clone, Copy)]
pub struct AvatarRecipe {
    pub default: f32,
    pub lg: f32,
}

#[derive(Clone, Copy)]
pub struct KbdRecipe {
    pub size: f32,
}

const TOKENS: Design = Design {
    spacing: 4.,
    text: TextScale {
        meta: 11.,
        body: 13.,
        title: 20.,
    },
    radius: 8.,
    gap: Gaps {
        tight: 4.,
        item: 8.,
        group: 16.,
        page: 24.,
    },
    shadow_opacity: 0.1,
    button: ButtonRecipe {
        xs: 6.,
        sm: 7.,
        default: 8.,
        lg: 9.,
        icon_xs: 6.,
        icon: 8.,
    },
    input: InputRecipe { height: 8. },
    badge: BadgeRecipe { height: 5. },
    avatar: AvatarRecipe {
        default: 8.,
        lg: 10.,
    },
    kbd: KbdRecipe { size: 5. },
    icon: IconRecipe {
        sm: 3.,
        default: 4.,
    },
    progress_height: 1.,
    tabs: gpui_kit::component::Size::Medium,
};

/// The design tokens.
pub const fn tokens() -> Design {
    TOKENS
}

/// `n` spacing units: fixed dimensions only; spacing between things is a `gap`.
pub fn sp(n: f32) -> Pixels {
    px(n * tokens().spacing)
}

/// Page gutter on every screen.
pub fn page() -> Pixels {
    px(tokens().gap.page)
}

/// The three gaps.
pub mod gap {
    use super::{Pixels, px, tokens};

    pub fn tight() -> Pixels {
        px(tokens().gap.tight)
    }

    pub fn item() -> Pixels {
        px(tokens().gap.item)
    }

    pub fn group() -> Pixels {
        px(tokens().gap.group)
    }
}

macro_rules! theme_colors {
    ($($name:ident = $field:ident),* $(,)?) => {
        /// Theme colors, from the active theme's roles (`theme::KIT_KEYS` says which
        /// gpui-kit color carries each role).
        pub mod color {
            use super::*;
            $(pub fn $name(cx: &App) -> Hsla { cx.theme().$field })*
        }
    };
}

theme_colors!(
    background = background,
    foreground = foreground,
    surface = popover,
    muted = muted,
    muted_foreground = muted_foreground,
    hover = accent,
    // not list_active: gpui-kit caps that to 20% alpha
    active = secondary_active,
    accent_foreground = accent_foreground,
    border = border,
    primary = primary,
    primary_foreground = primary_foreground,
    ring = ring,
    danger = danger,
    warning = warning,
);

/// Input fill: shadcn's `dark:bg-input/30`; on light themes the raised surface, so
/// fields read as white wells on a tinted page.
pub fn input_bg(cx: &App) -> Hsla {
    if cx.theme().is_dark() {
        cx.theme().input.opacity(0.3)
    } else {
        cx.theme().popover
    }
}

/// Font families, from the active theme.
pub mod font {
    use super::*;

    pub fn mono(cx: &App) -> SharedString {
        cx.theme().mono_font_family.clone()
    }
}

/// The three type sizes, from the theme.
pub mod text {
    use super::{Pixels, px, tokens};

    pub fn meta() -> Pixels {
        px(tokens().text.meta)
    }

    pub fn body() -> Pixels {
        px(tokens().text.body)
    }

    pub fn title() -> Pixels {
        px(tokens().text.title)
    }

    /// One line's height at `size` (Tailwind's `text-xs` leading, 16/12).
    pub fn line(size: Pixels) -> Pixels {
        size * (4. / 3.)
    }

    /// Control height: `units` of spacing, but never shorter than one line at `size`
    /// plus `pad_y` above and below, so big type in tight spacing isn't clipped.
    pub fn fit(units: f32, size: Pixels, pad_y: Pixels) -> Pixels {
        super::sp(units).max(line(size) + pad_y * 2.)
    }
}

/// Icon sizes, from the theme's `icon` recipe.
pub mod icon {
    use super::{Pixels, sp, tokens};

    pub fn sm() -> Pixels {
        sp(tokens().icon.sm)
    }

    pub fn default() -> Pixels {
        sp(tokens().icon.default)
    }
}

/// The one corner radius.
pub mod radius {
    use super::{Pixels, px, tokens};

    pub fn base() -> Pixels {
        px(tokens().radius)
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
        chart_h = 45.,
    );

    // text containers: scale with body type (like Tailwind's rem-based max-w-*); a unit
    // is 4px at a 14px body
    macro_rules! type_units {
        ($($name:ident = $n:expr),* $(,)?) => {
            $(pub fn $name() -> Pixels { px($n * tokens().text.body / 3.5) })*
        };
    }

    type_units!(
        form_w = 90.,
        list_max_w = 120.,
        prose_max_w = 140.,
        dialog_w = 112., // vega max-w-md
        up_next_w = 80.,
    );
    /// Scrub-preview width; the Player scales sprite tiles to it.
    pub fn trickplay_w() -> Pixels {
        sp(40.)
    }
}

/// Window geometry (OS window, not themed).
pub mod window {
    use gpui_kit::{Pixels, Point, Size, point, px, size};

    pub const DEFAULT: Size<Pixels> = size(px(1200.), px(800.));
    pub const MIN: Size<Pixels> = size(px(800.), px(560.));
    /// macOS unified-toolbar height (Finder/Safari): roomy, not themed.
    pub const TITLE_BAR_H: Pixels = px(52.);
    /// Traffic lights (14px tall) centered in the bar, same inset from top and left.
    pub const TRAFFIC_LIGHTS: Point<Pixels> = point(px(19.), px(19.));
    /// Extra lead before the brand: lights sit 6px right of where TitleBar's padding expects.
    pub const BRAND_LEAD: Pixels = px(12.);
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
