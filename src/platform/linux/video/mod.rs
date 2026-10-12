//! Linux video surface (ADR-0001): picks the Wayland or X11 implementation from the
//! window handle gpui hands out. Both expose the same `Surface` / `Renderer` shape as
//! `macos/video.rs`.

mod wayland;
mod x11;

use libmpv_sys::mpv_render_context;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawWindowHandle};

use crate::mpv::{Extent, Handle};

/// mpv's `hwdec` option for this platform: an explicit list, because `auto`/`auto-safe`
/// probe vulkan and cuda on every load and log decode errors on a plain Mesa box (Photon
/// ADR-0010). Hardware decode needs the display handed to mpv, see `wayland.rs`.
pub const HWDEC: &str = "vaapi,nvdec,vaapi-copy,nvdec-copy,no";

/// Main-thread side: owns the on-screen video surface.
pub enum Surface {
    Wayland(wayland::Surface),
    X11(x11::Surface),
}

/// Render-thread side: GL context and presenting a frame.
pub enum Renderer {
    Wayland(wayland::Renderer),
    X11(x11::Renderer),
}

impl Surface {
    pub fn attach(
        window: &(impl HasWindowHandle + HasDisplayHandle),
    ) -> Result<(Self, Renderer), String> {
        match window.window_handle().map_err(|e| e.to_string())?.as_raw() {
            RawWindowHandle::Wayland(_) => {
                let (surface, renderer) = wayland::Surface::attach(window)?;
                Ok((Self::Wayland(surface), Renderer::Wayland(renderer)))
            }
            RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_) => {
                let (surface, renderer) = x11::Surface::attach(window)?;
                Ok((Self::X11(surface), Renderer::X11(renderer)))
            }
            other => Err(format!("unsupported window system: {other:?}")),
        }
    }
}

impl Renderer {
    pub fn init(&mut self, mpv: Handle) -> Result<*mut mpv_render_context, String> {
        match self {
            Self::Wayland(r) => r.init(mpv),
            Self::X11(r) => r.init(mpv),
        }
    }

    /// Draws current frame; returns true if a frame was presented.
    pub fn draw(&mut self, ctx: *mut mpv_render_context, extent: Extent) -> bool {
        match self {
            Self::Wayland(r) => r.draw(ctx, extent),
            Self::X11(r) => r.draw(ctx, extent),
        }
    }
}
