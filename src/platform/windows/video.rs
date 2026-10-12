//! Windows video surface: not implemented (Spike B in .scratch/cross-platform/PLAN.md).

use libmpv_sys::mpv_render_context;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::mpv::{Extent, Handle};

/// mpv's `hwdec` option for this platform. Photon ran "auto" on real hardware with a GL
/// context (falls back to d3d11va-copy by itself); unverified on our surface.
pub const HWDEC: &str = "auto";

pub struct Surface;

impl Surface {
    pub fn attach(
        _: &(impl HasWindowHandle + HasDisplayHandle),
    ) -> Result<(Self, Renderer), String> {
        Err("Video playback on Windows is not supported yet".into())
    }
}

pub struct Renderer;

impl Renderer {
    pub fn init(&mut self, _: Handle) -> Result<*mut mpv_render_context, String> {
        unreachable!("attach never returns a Renderer")
    }

    pub fn draw(&mut self, _: *mut mpv_render_context, _: Extent) -> bool {
        unreachable!("attach never returns a Renderer")
    }
}
