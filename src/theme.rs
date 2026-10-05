//! App themes, backed by [`assets/themes/tbis.json`].
//!
//! The JSON is a `gpui-kit` theme set (Zed's theme JSON schema) whose themes also carry
//! a `"design"` object ([`Design`]): spacing, type scale, base radius, shadows, and
//! component recipes. One theme entry is the app's whole look.
//! Adding a theme is: add it to the JSON, then list its name in [`NAMES`].

use gpui_kit::App;
use gpui_kit::component::{Theme, ThemeRegistry};
use serde::Deserialize;

use crate::config::Config;
use crate::design::{self, Design};

/// Theme names as in the JSON's `"name"`, also the Settings labels. First is the default.
pub const NAMES: [&str; 4] = [
    "Raptor Night",
    "Bronto Morning",
    "Stego Dusk",
    "Angry T-Rex",
];

/// Earlier names still found in saved configs.
const RENAMED: [(&str, &str); 3] = [
    ("Rental Night", "Raptor Night"),
    ("Saturday Morning", "Bronto Morning"),
    ("Jurassic Dusk", "Stego Dusk"),
];

/// Bundled theme `name` refers to, renames followed; the default when unknown.
fn resolve(name: &str) -> &'static str {
    let name = RENAMED
        .iter()
        .find(|(old, _)| *old == name)
        .map_or(name, |(_, new)| new);
    NAMES.into_iter().find(|n| *n == name).unwrap_or(NAMES[0])
}

const FILE: &str = include_str!("../assets/themes/tbis.json");

/// The parts of the theme file gpui-kit ignores: each theme's `"design"`.
#[derive(Deserialize)]
struct DesignSet {
    themes: Vec<ThemeDesign>,
}

#[derive(Deserialize)]
struct ThemeDesign {
    name: String,
    #[serde(default)]
    design: Design,
}

/// Design of bundled theme `name`; vega when the theme sets none. Parsed once.
fn design_of(name: &str) -> Design {
    static SET: std::sync::OnceLock<DesignSet> = std::sync::OnceLock::new();
    SET.get_or_init(|| serde_json::from_str(FILE).expect("bundled theme file must parse"))
        .themes
        .iter()
        .find(|t| t.name == name)
        .map(|t| t.design)
        .unwrap_or_default()
}

/// Registers the bundled themes and activates `name`, or the default when `None` or unknown.
pub fn init(name: Option<&str>, cx: &mut App) {
    ThemeRegistry::global_mut(cx)
        .load_themes_from_str(FILE)
        .expect("bundled theme file must parse");
    apply(name.unwrap_or(NAMES[0]), cx);
}

/// Activates theme `name` (default when unknown) and redraws every window.
pub fn apply(name: &str, cx: &mut App) {
    let name = resolve(name);
    let mut config = ThemeRegistry::global(cx)
        .themes()
        .get(name)
        .cloned()
        .unwrap_or_else(|| panic!("theme {name:?} not found among registered themes"));
    let design = design_of(name);
    design::set(design);
    // gpui-kit's own sizes follow the design: rem = text base, radii from the one base
    let kit = std::rc::Rc::make_mut(&mut config);
    kit.font_size = Some(design.text.base);
    kit.radius = Some(design::radius::md().as_f32().round() as usize);
    kit.radius_lg = Some(design::radius::lg().as_f32().round() as usize);
    let mode = config.mode;
    // stores config as the mode's theme; change() then applies it and syncs the Base layer
    Theme::global_mut(cx).apply_config(&config);
    Theme::change(mode, None, cx);
    cx.refresh_windows();
}

#[cfg(test)]
mod tests {
    use super::{DesignSet, FILE, NAMES};

    #[test]
    fn every_bundled_theme_has_a_design() {
        let set: DesignSet = serde_json::from_str(FILE).expect("theme file parses");
        for name in NAMES {
            let theme = set.themes.iter().find(|t| t.name == name).expect(name);
            assert!(theme.design.spacing > 0., "{name} spacing");
        }
    }
}

/// The theme name `config` selects, resolved to a bundled one.
pub fn active(config: &Config) -> &'static str {
    resolve(config.theme.as_deref().unwrap_or_default())
}
