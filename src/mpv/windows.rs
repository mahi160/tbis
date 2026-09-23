//! Windows surface: not built yet (ADR-0001 covers macOS only). Same interface as mac.rs.

use libmpv_sys::mpv_render_context;
use raw_window_handle::HasWindowHandle;

use super::Handle;

pub struct Surface;

impl Surface {
    pub fn attach(_window: &impl HasWindowHandle) -> Result<(Self, Renderer), String> {
        Err("video playback is not supported on Windows yet".into())
    }
}

pub struct Renderer;

impl Renderer {
    pub fn init(&mut self, _mpv: Handle) -> Result<*mut mpv_render_context, String> {
        unreachable!("attach never succeeds on Windows")
    }

    pub fn draw(&mut self, _ctx: *mut mpv_render_context, _w: i32, _h: i32) -> bool {
        unreachable!("attach never succeeds on Windows")
    }
}
