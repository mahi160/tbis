use gpui_kit::Window;
use objc2_app_kit::{NSView, NSWindowButton};
use objc2_core_graphics::{CGColorSpace, kCGColorSpaceSRGB};
use objc2_quartz_core::CAMetalLayer;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// Hides the traffic lights, leaving a bare frame (Picture-in-Picture).
pub fn hide_title_buttons(window: &Window) {
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return;
    };
    // SAFETY: gpui keeps its view alive as long as the window; non-owning borrow
    let view: &NSView = unsafe { &*(appkit.ns_view.as_ptr() as *const NSView) };
    let Some(ns_window) = view.window() else {
        return;
    };
    for kind in [
        NSWindowButton::CloseButton,
        NSWindowButton::MiniaturizeButton,
        NSWindowButton::ZoomButton,
    ] {
        if let Some(button) = ns_window.standardWindowButton(kind) {
            button.setHidden(true);
        }
    }
}

/// gpui's Metal layer carries no colorspace, so macOS sends its sRGB values to the
/// display raw and wide-gamut (P3) screens oversaturate every color and poster.
/// Tagging it sRGB makes macOS color-match, as browsers do.
pub fn prepare(window: &Window) {
    // trait method, not gpui's inherent `Window::window_handle`
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return;
    };
    // SAFETY: gpui keeps its view alive as long as the window; non-owning borrow
    let view: &NSView = unsafe { &*(appkit.ns_view.as_ptr() as *const NSView) };
    let Some(layer) = view.layer() else {
        return;
    };
    let Ok(layer) = layer.downcast::<CAMetalLayer>() else {
        return;
    };
    let srgb = CGColorSpace::with_name(Some(unsafe { kCGColorSpaceSRGB }));
    layer.setColorspace(srgb.as_deref());
}
