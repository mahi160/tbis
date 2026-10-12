//! Wayland video surface (ADR-0001, Spike A in .scratch/cross-platform/PLAN.md).
//!
//! gpui paints its whole `wl_surface` through wgpu, so mpv gets its own `wl_surface`
//! attached as a desynchronised `wl_subsurface` placed *below* gpui's. With the gpui
//! window transparent (`WindowBackgroundAppearance::Transparent`, set when the Player
//! shows), the video shows through under the controls. mpv renders with OpenGL through
//! EGL straight into that subsurface's `wl_egl_window`.
//!
//! Unverified until run on a real compositor. Known gaps: the video covers the whole
//! gpui surface (ignores client-side decoration insets).

use std::ffi::{CStr, c_char, c_void};
use std::sync::OnceLock;

use khronos_egl as egl;
use libmpv_sys::{mpv_render_context, mpv_render_param_type_MPV_RENDER_PARAM_WL_DISPLAY};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use wayland_backend::client::{Backend, ObjectId};
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_region::WlRegion;
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::protocol::wl_subcompositor::WlSubcompositor;
use wayland_client::protocol::wl_subsurface::WlSubsurface;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle, delegate_noop};
use wayland_egl::WlEglSurface;
use wayland_protocols::wp::viewporter::client::wp_viewport::WpViewport;
use wayland_protocols::wp::viewporter::client::wp_viewporter::WpViewporter;

use crate::mpv::{Extent, Handle, create_gl_render_context, render_gl};

/// We never read events on our queue: every object here is request-only.
struct State;

impl Dispatch<WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(State: WlCompositor);
delegate_noop!(State: WlSubcompositor);
delegate_noop!(State: WlSubsurface);
delegate_noop!(State: WlRegion);
delegate_noop!(State: WpViewporter);
delegate_noop!(State: WpViewport);
delegate_noop!(State: ignore WlSurface);

/// Main-thread side: the subsurface under gpui's surface.
pub struct Surface {
    subsurface: WlSubsurface,
    surface: WlSurface,
    // objects above are bound to this queue; keep it alive until they are destroyed
    _queue: EventQueue<State>,
    conn: Connection,
}

impl Surface {
    pub fn attach(
        window: &(impl HasWindowHandle + HasDisplayHandle),
    ) -> Result<(Self, Renderer), String> {
        let RawWindowHandle::Wayland(parent) =
            window.window_handle().map_err(|e| e.to_string())?.as_raw()
        else {
            return Err("expected a Wayland window".into());
        };
        let RawDisplayHandle::Wayland(display) =
            window.display_handle().map_err(|e| e.to_string())?.as_raw()
        else {
            return Err("expected a Wayland display".into());
        };

        // SAFETY: gpui owns the display for the process lifetime; a foreign-display
        // backend never disconnects it
        let backend = unsafe { Backend::from_foreign_display(display.display.as_ptr().cast()) };
        let conn = Connection::from_backend(backend);
        let (globals, queue) = registry_queue_init::<State>(&conn).map_err(|e| e.to_string())?;
        let qh = queue.handle();
        let compositor: WlCompositor = globals.bind(&qh, 1..=6, ()).map_err(|e| e.to_string())?;
        let subcompositor: WlSubcompositor =
            globals.bind(&qh, 1..=1, ()).map_err(|e| e.to_string())?;
        // without it, `Renderer::draw` falls back to integer buffer scale
        let viewporter: Option<WpViewporter> = globals.bind(&qh, 1..=1, ()).ok();

        // SAFETY: gpui keeps its surface alive as long as the window
        let parent_id =
            unsafe { ObjectId::from_ptr(WlSurface::interface(), parent.surface.as_ptr().cast()) }
                .map_err(|e| e.to_string())?;
        let parent = WlSurface::from_id(&conn, parent_id).map_err(|e| e.to_string())?;

        let surface = compositor.create_surface(&qh, ());
        let subsurface = subcompositor.get_subsurface(&surface, &parent, &qh, ());
        // own commits (EGL swaps) apply at once; no tie to gpui's frame cadence
        subsurface.set_desync();
        subsurface.place_below(&parent);
        // empty input region: clicks go to gpui, never to the video
        let empty = compositor.create_region(&qh, ());
        surface.set_input_region(Some(&empty));
        empty.destroy();
        let viewport = viewporter.map(|v| v.get_viewport(&surface, &qh, ()));
        conn.flush().map_err(|e| e.to_string())?;

        let renderer = Renderer {
            conn: conn.clone(),
            surface: surface.clone(),
            viewport,
            gl: None,
        };
        Ok((
            Self {
                subsurface,
                surface,
                _queue: queue,
                conn,
            },
            renderer,
        ))
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        // renderer (EGL window, viewport) is already gone; see `Mpv::drop`
        self.subsurface.destroy();
        self.surface.destroy();
        let _ = self.conn.flush();
    }
}

type Egl = egl::DynamicInstance<egl::EGL1_4>;

/// mpv's `get_proc_address` has no closure, so the loaded library lives here.
static EGL: OnceLock<Egl> = OnceLock::new();

fn egl_instance() -> Result<&'static Egl, String> {
    if let Some(instance) = EGL.get() {
        return Ok(instance);
    }
    // SAFETY: loads the system libEGL; its initialisers are trusted
    let instance = unsafe { Egl::load_required() }.map_err(|e| format!("libEGL: {e}"))?;
    Ok(EGL.get_or_init(|| instance))
}

unsafe extern "C" fn gl_get_proc_address(_: *mut c_void, name: *const c_char) -> *mut c_void {
    let Some(egl) = EGL.get() else {
        return std::ptr::null_mut();
    };
    // SAFETY: mpv passes a NUL-terminated symbol name
    let Ok(name) = unsafe { CStr::from_ptr(name) }.to_str() else {
        return std::ptr::null_mut();
    };
    egl.get_proc_address(name)
        .map_or(std::ptr::null_mut(), |f| f as *mut c_void)
}

/// Render-thread side: EGL context on the subsurface's `wl_egl_window`.
pub struct Renderer {
    // `gl` first: its EGL window must go before the wl_surface it wraps
    gl: Option<Gl>,
    viewport: Option<WpViewport>,
    surface: WlSurface,
    conn: Connection,
}

struct Gl {
    egl: &'static Egl,
    display: egl::Display,
    context: egl::Context,
    egl_surface: egl::Surface,
    /// Dropped after `Gl::drop` has destroyed `egl_surface`.
    window: WlEglSurface,
    /// Last size/scale applied to window and viewport.
    applied: Extent,
}

// SAFETY: `gl` is None until `init`, and `Renderer` moves to the render thread before
// that; EGL handles are then only touched there (including the drop)
unsafe impl Send for Gl {}

impl Renderer {
    pub fn init(&mut self, mpv: Handle) -> Result<*mut mpv_render_context, String> {
        let egl = egl_instance()?;
        let err = |what: &'static str| move |e: egl::Error| format!("{what}: {e}");

        // SAFETY: the pointer is gpui's live wl_display
        let display = unsafe { egl.get_display(self.conn.backend().display_ptr().cast()) }
            .ok_or("no EGL display")?;
        egl.initialize(display).map_err(err("eglInitialize"))?;
        egl.bind_api(egl::OPENGL_API).map_err(err("eglBindAPI"))?;
        let config = egl
            .choose_first_config(
                display,
                &[
                    egl::SURFACE_TYPE,
                    egl::WINDOW_BIT,
                    egl::RENDERABLE_TYPE,
                    egl::OPENGL_BIT,
                    egl::RED_SIZE,
                    8,
                    egl::GREEN_SIZE,
                    8,
                    egl::BLUE_SIZE,
                    8,
                    egl::NONE,
                ],
            )
            .map_err(err("eglChooseConfig"))?
            .ok_or("no EGL config for OpenGL")?;
        let context = egl
            .create_context(
                display,
                config,
                None,
                &[
                    egl::CONTEXT_MAJOR_VERSION,
                    3,
                    egl::CONTEXT_MINOR_VERSION,
                    2,
                    egl::CONTEXT_OPENGL_PROFILE_MASK,
                    egl::CONTEXT_OPENGL_CORE_PROFILE_BIT,
                    egl::NONE,
                ],
            )
            .map_err(err("eglCreateContext"))?;
        // real size arrives with the first `draw`; a context needs a surface to be current
        let window = WlEglSurface::new(self.surface.id(), 1, 1).map_err(|e| format!("{e:?}"))?;
        // SAFETY: `window` is a live wl_egl_window, destroyed after the EGL surface
        let egl_surface = unsafe {
            egl.create_window_surface(display, config, window.ptr() as *mut c_void, None)
        }
        .map_err(err("eglCreateWindowSurface"))?;
        egl.make_current(display, Some(egl_surface), Some(egl_surface), Some(context))
            .map_err(err("eglMakeCurrent"))?;
        // mpv paces frames itself; vsync on the swap would also block on frame callbacks
        let _ = egl.swap_interval(display, 0);

        self.gl = Some(Gl {
            egl,
            display,
            context,
            egl_surface,
            window,
            applied: Extent::default(),
        });
        // zero-copy VA-API/NVDEC interop needs the same display EGL was created on
        let native = Some((
            mpv_render_param_type_MPV_RENDER_PARAM_WL_DISPLAY,
            self.conn.backend().display_ptr().cast(),
        ));
        create_gl_render_context(mpv, gl_get_proc_address, native)
    }

    /// Draws current frame at `extent`. Returns true if a frame was presented.
    pub fn draw(&mut self, ctx: *mut mpv_render_context, extent: Extent) -> bool {
        let Some(gl) = &mut self.gl else {
            return false;
        };
        if gl.applied != extent {
            gl.window.resize(extent.width, extent.height, 0, 0);
            let scale = extent.scale.max(1.);
            match &self.viewport {
                // fractional scale: buffer is device pixels, shown at the logical size
                Some(viewport) => viewport.set_destination(
                    (extent.width as f32 / scale).round() as i32,
                    (extent.height as f32 / scale).round() as i32,
                ),
                None => self.surface.set_buffer_scale(scale.round() as i32),
            }
            gl.applied = extent;
        }
        if !render_gl(ctx, 0, extent.width, extent.height, true) {
            return false;
        }
        gl.egl.swap_buffers(gl.display, gl.egl_surface).is_ok()
    }
}

impl Drop for Gl {
    fn drop(&mut self) {
        let _ = self.egl.make_current(self.display, None, None, None);
        let _ = self.egl.destroy_surface(self.display, self.egl_surface);
        let _ = self.egl.destroy_context(self.display, self.context);
        // no eglTerminate: the display is shared with whatever else gpui initialised
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.gl = None; // before the viewport and surface it draws into
        if let Some(viewport) = self.viewport.take() {
            viewport.destroy();
        }
        let _ = self.conn.flush();
    }
}
