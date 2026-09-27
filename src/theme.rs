//! App themes, backed by [`assets/themes/tbis.json`].
//!
//! The JSON is a `gpui-kit` theme set (Zed's theme JSON schema). Themes differ in
//! color only; font, sizes, and radii are shared so every theme lays out the same.
//! Adding a theme is: add it to the JSON, then list its name in [`NAMES`].

use gpui_kit::App;
use gpui_kit::component::{Theme, ThemeRegistry};

use crate::config::Config;

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
    let config = ThemeRegistry::global(cx)
        .themes()
        .get(name)
        .cloned()
        .unwrap_or_else(|| panic!("theme {name:?} not found among registered themes"));
    let mode = config.mode;
    // stores config as the mode's theme; change() then applies it and syncs the Base layer
    Theme::global_mut(cx).apply_config(&config);
    Theme::change(mode, None, cx);
    cx.refresh_windows();
}

/// The theme name `config` selects, resolved to a bundled one.
pub fn active(config: &Config) -> &'static str {
    resolve(config.theme.as_deref().unwrap_or_default())
}
