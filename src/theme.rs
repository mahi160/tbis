//! App theme registration, backed by [`assets/themes`].
//!
//! Each file under `assets/themes` is a `gpui-kit` theme set (Zed's theme
//! JSON schema). Registering a file adds every theme it defines to
//! [`ThemeRegistry`] by name; the app then picks the active one by name.
//! Adding another theme is: drop a new JSON file here, list it in
//! [`FILES`], and (optionally) offer it by [`NAME`] wherever themes are
//! picked.

use gpui_kit::App;
use gpui_kit::component::{Theme, ThemeMode, ThemeRegistry};

/// The theme this app ships active by default.
pub const NAME: &str = "Gruvbox Material";

/// Theme set files to register, matched to [`NAME`] by their `"name"` field.
const FILES: &[&str] = &[include_str!("../assets/themes/gruvbox-material.json")];

/// Registers every theme in [`FILES`] and activates [`NAME`].
pub fn init(cx: &mut App) {
    Theme::change(ThemeMode::Dark, None, cx);

    for file in FILES {
        ThemeRegistry::global_mut(cx)
            .load_themes_from_str(file)
            .expect("bundled theme file must parse");
    }

    let active = ThemeRegistry::global(cx)
        .themes()
        .get(NAME)
        .cloned()
        .unwrap_or_else(|| panic!("theme {NAME:?} not found among registered themes"));
    Theme::global_mut(cx).apply_config(&active);
}
