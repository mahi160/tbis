//! gpui-kit Settings page at the theme's `design.settings` size.

use gpui_kit::component::Sizable as _;
use gpui_kit::component::setting::Settings;
use gpui_kit::*;

use crate::design::color;

pub fn settings(id: impl Into<ElementId>, cx: &App) -> Settings {
    // sidebar on page background: kit's sidebar fill made a hard stop under the header
    let sidebar = StyleRefinement::default().bg(color::background(cx));
    Settings::new(id)
        .with_size(crate::design::tokens().settings.kit())
        .sidebar_style(&sidebar)
}
