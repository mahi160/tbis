//! gpui-kit Settings page at the theme's `design.settings` size.

use gpui_kit::component::Sizable as _;
use gpui_kit::component::setting::Settings;
use gpui_kit::*;

pub fn settings(id: impl Into<ElementId>) -> Settings {
    Settings::new(id).with_size(crate::design::tokens().settings.kit())
}
