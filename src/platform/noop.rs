//! Do-nothing platform pieces more than one OS shares; see `platform/mod.rs`.

pub mod window {
    pub fn prepare(_: &gpui_kit::Window) {}
    pub fn hide_title_buttons(_: &gpui_kit::Window) {}
    pub fn lock_aspect(_: &gpui_kit::Window, _: f32, _: f32) {}
}
