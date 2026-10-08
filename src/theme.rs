//! App themes, backed by [`assets/themes/tbis.json`].
//!
//! Each theme sets the color [`ROLES`] (OKLCH or hex) and font families; an optional
//! `"design"` object ([`Design`]) overrides the shared defaults (type, radius, gaps,
//! heights). Before gpui-kit reads the file, [`kit_file`] converts the roles to hex and
//! expands them into gpui-kit's own color keys, so gpui-kit components and ours draw
//! from the same roles. The tests hold every theme to the roles and to WCAG contrast
//! minimums.
//! Adding a theme is: add it to the JSON, then list its name in [`NAMES`].

use gpui_kit::component::{Theme, ThemeRegistry, oklch};
use gpui_kit::{App, Rgba};
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::config::Config;
use crate::design::{self, Design};

/// Theme names as in the JSON's `"name"`, also the Settings labels. First is the default.
pub const NAMES: [&str; 4] = ["Raptor Night", "Stego Moss", "Plesio Lagoon", "Fossil Bone"];

/// Earlier names still found in saved configs.
const RENAMED: [(&str, &str); 1] = [("WMC", "Plesio Lagoon")];

/// Bundled theme `name` refers to, renames followed; the default when unknown.
fn resolve(name: &str) -> &'static str {
    let name = RENAMED
        .iter()
        .find(|(old, _)| *old == name)
        .map_or(name, |(_, new)| new);
    NAMES.into_iter().find(|n| *n == name).unwrap_or(NAMES[0])
}

/// The theme name `config` selects, resolved to a bundled one.
pub fn active(config: &Config) -> &'static str {
    resolve(config.theme.as_deref().unwrap_or_default())
}

const FILE: &str = include_str!("../assets/themes/tbis.json");

/// The color roles every theme sets, and nothing else (`KIT_KEYS` maps each one).
#[cfg(test)]
const ROLES: [&str; 15] = [
    "background",
    "foreground",
    "surface.background", // cards, menus, popovers
    "muted.background",   // fills: tab strip, tiles, chips
    "muted.foreground",
    "border",
    "border.subtle", // window, bar and row dividers
    "hover.background",
    "active.background",
    "selection.background",
    "primary.background", // also links, progress, charts, the rating star
    "primary.foreground",
    "focus", // focus ring and caret
    "danger.background",
    "danger.foreground",
];

/// gpui-kit color key and the role it takes.
const KIT_KEYS: [(&str, &str); 44] = [
    ("background", "background"),
    ("foreground", "foreground"),
    ("popover.background", "surface.background"),
    ("popover.foreground", "foreground"),
    ("muted.background", "muted.background"),
    ("muted.foreground", "muted.foreground"),
    ("skeleton.background", "muted.background"),
    ("secondary.background", "muted.background"),
    ("secondary.foreground", "foreground"),
    ("secondary.hover.background", "hover.background"),
    ("secondary.active.background", "active.background"),
    // shadcn's accent is the hover fill (ghost buttons, menu items)
    ("accent.background", "hover.background"),
    ("accent.foreground", "foreground"),
    ("list.hover.background", "hover.background"),
    ("list.active.background", "active.background"),
    ("switch.background", "active.background"),
    ("title_bar.background", "background"),
    ("sidebar.background", "background"),
    ("sidebar.foreground", "foreground"),
    ("status_bar.background", "background"),
    ("border", "border"),
    ("input.border", "border"),
    ("scrollbar.thumb.background", "border"),
    ("window.border", "border.subtle"),
    ("title_bar.border", "border.subtle"),
    ("sidebar.border", "border.subtle"),
    ("status_bar.border", "border.subtle"),
    ("table.row.border", "border.subtle"),
    ("selection.background", "selection.background"),
    ("ring", "focus"),
    ("caret", "focus"),
    ("primary.background", "primary.background"),
    ("primary.foreground", "primary.foreground"),
    ("link", "primary.background"),
    ("warning.background", "primary.background"),
    ("progress.bar.background", "primary.background"),
    ("slider.background", "primary.background"),
    ("chart.1", "primary.background"),
    ("chart.2", "primary.background"),
    ("chart.3", "primary.background"),
    ("chart.4", "primary.background"),
    ("chart.5", "primary.background"),
    ("danger.background", "danger.background"),
    ("danger.foreground", "danger.foreground"),
];

/// `#rrggbb` for a role value: hex as is, `oklch(L C H)` converted (gpui-kit reads
/// only hex and palette names).
fn hex(value: &str) -> String {
    let Some(args) = value
        .strip_prefix("oklch(")
        .and_then(|v| v.strip_suffix(')'))
    else {
        return value.to_owned();
    };
    let [l, c, h] = [0, 1, 2].map(|i| {
        args.split_whitespace()
            .nth(i)
            .and_then(|n| n.parse::<f32>().ok())
            .unwrap_or_else(|| panic!("bad oklch value {value:?}"))
    });
    let rgb = Rgba::from(oklch(l, c, h));
    let byte = |x: f32| (x.clamp(0., 1.) * 255.).round() as u8;
    format!("#{:02x}{:02x}{:02x}", byte(rgb.r), byte(rgb.g), byte(rgb.b))
}

/// The theme file as gpui-kit reads it: each theme's roles expanded into its color keys.
fn kit_file() -> String {
    let mut file: Value = serde_json::from_str(FILE).expect("bundled theme file must parse");
    for theme in file["themes"].as_array_mut().expect("themes array") {
        let dark = theme["mode"] == "dark";
        let role = |key: &str| {
            hex(theme["colors"][key]
                .as_str()
                .unwrap_or_else(|| panic!("{} misses color role {key}", theme["name"])))
        };
        let mut colors: Map<String, Value> = KIT_KEYS
            .iter()
            .map(|(kit, r)| (kit.to_string(), Value::from(role(r))))
            .collect();
        // light knob on dark tracks; on light themes the raised surface
        let thumb = role(if dark {
            "foreground"
        } else {
            "surface.background"
        });
        colors.insert("switch.thumb.background".into(), thumb.into());
        theme["colors"] = Value::Object(colors);
    }
    file.to_string()
}

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

/// Design of bundled theme `name`; the shared defaults when it sets none. Parsed once.
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
        .load_themes_from_str(&kit_file())
        .expect("bundled theme file must parse");
    apply(name.unwrap_or_default(), cx);
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
    // gpui-kit's own sizes follow the design: its controls use text-sm (0.875rem), so
    // rem is set to make that the body size; both kit radii are the one radius
    let kit = std::rc::Rc::make_mut(&mut config);
    kit.font_size = Some(design.text.body / 0.875);
    kit.radius = Some(design.radius.round() as usize);
    kit.radius_lg = Some(design.radius.round() as usize);
    let mode = config.mode;
    // stores config as the mode's theme; change() then applies it and syncs the Base layer
    Theme::global_mut(cx).apply_config(&config);
    Theme::change(mode, None, cx);
    cx.refresh_windows();
}

#[cfg(test)]
mod tests {
    use super::{DesignSet, FILE, NAMES, ROLES, hex};

    #[test]
    fn every_bundled_theme_has_a_design() {
        let set: DesignSet = serde_json::from_str(FILE).expect("theme file parses");
        for name in NAMES {
            let theme = set.themes.iter().find(|t| t.name == name).expect(name);
            assert!(theme.design.spacing > 0., "{name} spacing");
        }
    }

    #[test]
    fn oklch_converts_to_hex() {
        assert_eq!(hex("oklch(1 0 0)"), "#ffffff");
        assert_eq!(hex("oklch(0 0 0)"), "#000000");
        assert_eq!(hex("#12abef"), "#12abef");
    }

    /// WCAG 2.x relative luminance of `#rrggbb`.
    fn luminance(hex: &str) -> f64 {
        let channel = |i: usize| {
            let c = f64::from(u8::from_str_radix(&hex[i..i + 2], 16).expect("hex color")) / 255.;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5)
    }

    fn contrast(a: &str, b: &str) -> f64 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    /// Body text AAA (7), meta and label text AA (4.5), primary and focus as UI
    /// components (3, WCAG 1.4.11), and fill/border steps that stay visible.
    #[test]
    fn themes_meet_contrast_minimums() {
        const PAIRS: [(&str, &str, f64); 14] = [
            ("foreground", "background", 7.),
            ("foreground", "surface.background", 7.),
            ("muted.foreground", "background", 4.5),
            ("muted.foreground", "muted.background", 4.5),
            ("primary.foreground", "primary.background", 4.5),
            ("danger.foreground", "danger.background", 4.5),
            ("primary.background", "background", 3.),
            ("focus", "background", 3.),
            ("danger.background", "background", 3.),
            ("surface.background", "background", 1.08),
            ("border", "background", 1.3),
            ("border.subtle", "background", 1.15),
            ("hover.background", "muted.background", 1.05),
            ("active.background", "hover.background", 1.05),
        ];
        let file: serde_json::Value = serde_json::from_str(FILE).expect("theme file parses");
        for theme in file["themes"].as_array().expect("themes array") {
            let color = |role: &str| hex(theme["colors"][role].as_str().expect(role));
            for (a, b, min) in PAIRS {
                let ratio = contrast(&color(a), &color(b));
                assert!(
                    ratio >= min,
                    "{}: {a} on {b} is {ratio:.2}, needs {min}",
                    theme["name"]
                );
            }
        }
    }

    #[test]
    fn themes_set_exactly_the_color_roles() {
        let file: serde_json::Value = serde_json::from_str(FILE).expect("theme file parses");
        for theme in file["themes"].as_array().expect("themes array") {
            let colors = theme["colors"].as_object().expect("colors object");
            for role in ROLES {
                assert!(colors.contains_key(role), "{} misses {role}", theme["name"]);
            }
            for key in colors.keys() {
                assert!(
                    ROLES.contains(&key.as_str()),
                    "{} sets non-role {key}",
                    theme["name"]
                );
            }
        }
    }
}
