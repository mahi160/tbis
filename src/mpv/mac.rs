//! macOS surface (ADR-0001): an NSView with a CAMetalLayer below gpui's view. mpv renders with
//! OpenGL into an IOSurface-backed FBO; the same IOSurface is a Metal texture blitted onto the layer.

// NSOpenGL: only GL context route for mpv on macOS
#![allow(deprecated)]

use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_uint};

use libmpv_sys::mpv_render_context;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{AnyThread, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSOpenGLContext, NSOpenGLPFAAccelerated, NSOpenGLPFAColorSize,
    NSOpenGLPFADoubleBuffer, NSOpenGLPFAOpenGLProfile, NSOpenGLPixelFormat,
    NSOpenGLPixelFormatAttribute, NSOpenGLProfileVersion3_2Core, NSView, NSWindowOrderingMode,
};
use objc2_core_foundation::{CFDictionary, CFNumber, CFRetained, CFString};
use objc2_foundation::NSSize;
use objc2_io_surface::IOSurfaceRef;
use objc2_metal::{
    MTLBlitCommandEncoder, MTLCommandBuffer, MTLCommandEncoder, MTLCommandQueue,
    MTLCreateSystemDefaultDevice, MTLDevice, MTLPixelFormat, MTLStorageMode, MTLTexture,
    MTLTextureDescriptor, MTLTextureUsage,
};
use objc2_quartz_core::{CAMetalDrawable, CAMetalLayer};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use super::{Handle, create_gl_render_context, render_gl};

/// Main-thread side: the view holding the video layer.
pub struct Surface {
    view: Retained<NSView>,
}

impl Surface {
    pub fn attach(window: &impl HasWindowHandle) -> Result<(Self, Renderer), String> {
        let RawWindowHandle::AppKit(appkit) =
            window.window_handle().map_err(|e| e.to_string())?.as_raw()
        else {
            return Err("expected an AppKit window".into());
        };
        let mtm = MainThreadMarker::new().ok_or("mpv surface must attach on the main thread")?;
        // SAFETY: gpui keeps its view alive as long as the window; non-owning borrow
        let gpui_view: &NSView = unsafe { &*(appkit.ns_view.as_ptr() as *const NSView) };
        let content_view = unsafe { gpui_view.superview() }.ok_or("gpui view has no superview")?;

        let view = NSView::initWithFrame(NSView::alloc(mtm), content_view.bounds());
        view.setWantsLayer(true);
        view.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        let layer = CAMetalLayer::new();
        layer.setPixelFormat(MTLPixelFormat::BGRA8Unorm);
        layer.setFramebufferOnly(false); // blit target, not render target
        view.setLayer(Some(&layer));
        content_view.addSubview_positioned_relativeTo(
            &view,
            NSWindowOrderingMode::Below,
            Some(gpui_view),
        );

        Ok((Self { view }, Renderer { layer, gpu: None }))
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        self.view.removeFromSuperview();
    }
}

/// Render-thread side: GL context, Metal queue, current frame target.
pub struct Renderer {
    layer: Retained<CAMetalLayer>,
    gpu: Option<Gpu>,
}

// only touched on render thread; CAMetalLayer drawable/size calls are thread-safe
unsafe impl Send for Renderer {}

struct Gpu {
    gl: Retained<NSOpenGLContext>,
    device: Retained<ProtocolObject<dyn MTLDevice>>,
    queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
    target: Option<Target>,
}

impl Drop for Gpu {
    fn drop(&mut self) {
        self.gl.makeCurrentContext(); // glDelete* in Target::drop need this context
        self.target = None;
    }
}

impl Renderer {
    pub fn init(&mut self, mpv: Handle) -> Result<*mut mpv_render_context, String> {
        let device = MTLCreateSystemDefaultDevice().ok_or("no Metal device")?;
        let queue = device.newCommandQueue().ok_or("no Metal command queue")?;
        self.layer.setDevice(Some(&device));

        let attrs: [NSOpenGLPixelFormatAttribute; 7] = [
            NSOpenGLPFAAccelerated,
            NSOpenGLPFAOpenGLProfile,
            NSOpenGLProfileVersion3_2Core,
            NSOpenGLPFAColorSize,
            32,
            NSOpenGLPFADoubleBuffer,
            0,
        ];
        let attrs_ptr =
            std::ptr::NonNull::new(attrs.as_ptr() as *mut NSOpenGLPixelFormatAttribute).unwrap();
        let pixel_format = unsafe {
            NSOpenGLPixelFormat::initWithAttributes(NSOpenGLPixelFormat::alloc(), attrs_ptr)
        }
        .ok_or("no accelerated OpenGL pixel format")?;
        let gl = NSOpenGLContext::initWithFormat_shareContext(
            NSOpenGLContext::alloc(),
            &pixel_format,
            None,
        )
        .ok_or("NSOpenGLContext creation failed")?;
        gl.makeCurrentContext();

        let ctx = create_gl_render_context(mpv, gl_get_proc_address)?;
        self.gpu = Some(Gpu {
            gl,
            device,
            queue,
            target: None,
        });
        Ok(ctx)
    }

    /// Draws current frame at `w`×`h` device pixels. Returns true if a frame was presented.
    pub fn draw(&mut self, ctx: *mut mpv_render_context, w: i32, h: i32) -> bool {
        let Some(gpu) = &mut self.gpu else {
            return false;
        };
        gpu.gl.makeCurrentContext();
        if !matches!(&gpu.target, Some(t) if t.w == w && t.h == h) {
            gpu.target = None; // free old GL objects first
            match make_target(&gpu.device, w, h) {
                Ok(t) => {
                    self.layer.setDrawableSize(NSSize::new(w as f64, h as f64));
                    gpu.target = Some(t);
                }
                Err(err) => {
                    eprintln!("mpv render target: {err}");
                    return false;
                }
            }
        }
        let Some(t) = &gpu.target else { return false };

        unsafe {
            glBindFramebuffer(GL_FRAMEBUFFER, t.fbo);
            glViewport(0, 0, w, h);
        }
        if !render_gl(ctx, t.fbo as c_int, w, h) {
            return false;
        }
        unsafe { glFlush() }; // GL writes land before Metal reads same IOSurface

        let Some(drawable) = self.layer.nextDrawable() else {
            return false;
        };
        let Some(cmd) = gpu.queue.commandBuffer() else {
            return false;
        };
        let Some(blit) = cmd.blitCommandEncoder() else {
            return false;
        };
        unsafe { blit.copyFromTexture_toTexture(&t.metal_texture, &drawable.texture()) };
        blit.endEncoding();
        cmd.presentDrawable(drawable.as_ref());
        cmd.commit();
        true
    }
}

// ---- hand-declared GL/CGL FFI: tiny frozen surface, not worth a GL crate ----
type GLenum = c_uint;
type GLuint = c_uint;
type GLint = c_int;
type GLsizei = c_int;

const GL_TEXTURE_RECTANGLE: GLenum = 0x84F5;
const GL_RGBA: GLenum = 0x1908;
const GL_BGRA: GLenum = 0x80E1;
const GL_UNSIGNED_INT_8_8_8_8_REV: GLenum = 0x8367;
const GL_FRAMEBUFFER: GLenum = 0x8D40;
const GL_COLOR_ATTACHMENT0: GLenum = 0x8CE0;
const GL_FRAMEBUFFER_COMPLETE: GLenum = 0x8CD5;
const GL_TEXTURE_MIN_FILTER: GLenum = 0x2801;
const GL_TEXTURE_MAG_FILTER: GLenum = 0x2800;
const GL_LINEAR: GLenum = 0x2601;

#[link(name = "OpenGL", kind = "framework")]
unsafe extern "C" {
    fn CGLGetCurrentContext() -> *mut c_void;
    fn CGLTexImageIOSurface2D(
        ctx: *mut c_void,
        target: GLenum,
        internal_format: GLenum,
        width: GLsizei,
        height: GLsizei,
        format: GLenum,
        type_: GLenum,
        io_surface: *mut c_void,
        plane: GLuint,
    ) -> c_int;
    fn glGenTextures(n: GLsizei, textures: *mut GLuint);
    fn glDeleteTextures(n: GLsizei, textures: *const GLuint);
    fn glBindTexture(target: GLenum, texture: GLuint);
    fn glTexParameteri(target: GLenum, pname: GLenum, param: GLint);
    fn glGenFramebuffers(n: GLsizei, framebuffers: *mut GLuint);
    fn glDeleteFramebuffers(n: GLsizei, framebuffers: *const GLuint);
    fn glBindFramebuffer(target: GLenum, framebuffer: GLuint);
    fn glFramebufferTexture2D(
        target: GLenum,
        attachment: GLenum,
        textarget: GLenum,
        texture: GLuint,
        level: GLint,
    );
    fn glCheckFramebufferStatus(target: GLenum) -> GLenum;
    fn glViewport(x: GLint, y: GLint, width: GLsizei, height: GLsizei);
    fn glFlush();
}

unsafe extern "C" fn gl_get_proc_address(_ctx: *mut c_void, name: *const c_char) -> *mut c_void {
    unsafe { libc::dlsym(libc::RTLD_DEFAULT, name) }
}

/// FBO + IOSurface + Metal texture for one frame size.
struct Target {
    w: i32,
    h: i32,
    fbo: GLuint,
    gl_texture: GLuint,
    _io_surface: CFRetained<IOSurfaceRef>,
    metal_texture: Retained<ProtocolObject<dyn MTLTexture>>,
}

impl Drop for Target {
    fn drop(&mut self) {
        unsafe {
            glDeleteFramebuffers(1, &self.fbo);
            glDeleteTextures(1, &self.gl_texture);
        }
    }
}

fn make_target(device: &ProtocolObject<dyn MTLDevice>, w: i32, h: i32) -> Result<Target, String> {
    // no IOSurfaceBytesPerRow: let IOSurface pick Metal's 16-byte-aligned stride
    let keys = [
        CFString::from_str("IOSurfaceWidth"),
        CFString::from_str("IOSurfaceHeight"),
        CFString::from_str("IOSurfaceBytesPerElement"),
        CFString::from_str("IOSurfacePixelFormat"),
    ];
    let values = [
        CFNumber::new_i32(w),
        CFNumber::new_i32(h),
        CFNumber::new_i32(4),
        CFNumber::new_i32(0x42475241), // 'BGRA'
    ];
    let key_refs: Vec<&CFString> = keys.iter().map(|k| &**k).collect();
    let value_refs: Vec<&CFNumber> = values.iter().map(|v| &**v).collect();
    let dict: CFRetained<CFDictionary<CFString, CFNumber>> =
        CFDictionary::from_slices(&key_refs, &value_refs);
    let io_surface =
        unsafe { IOSurfaceRef::new(dict.as_ref()) }.ok_or("IOSurface creation failed")?;

    let mut gl_texture: GLuint = 0;
    let mut fbo: GLuint = 0;
    unsafe {
        glGenTextures(1, &mut gl_texture);
        glBindTexture(GL_TEXTURE_RECTANGLE, gl_texture);
        glTexParameteri(
            GL_TEXTURE_RECTANGLE,
            GL_TEXTURE_MIN_FILTER,
            GL_LINEAR as GLint,
        );
        glTexParameteri(
            GL_TEXTURE_RECTANGLE,
            GL_TEXTURE_MAG_FILTER,
            GL_LINEAR as GLint,
        );
        let rc = CGLTexImageIOSurface2D(
            CGLGetCurrentContext(),
            GL_TEXTURE_RECTANGLE,
            GL_RGBA,
            w,
            h,
            GL_BGRA,
            GL_UNSIGNED_INT_8_8_8_8_REV,
            &*io_surface as *const IOSurfaceRef as *mut c_void,
            0,
        );
        if rc != 0 {
            glDeleteTextures(1, &gl_texture);
            return Err(format!("CGLTexImageIOSurface2D failed ({rc})"));
        }
        glGenFramebuffers(1, &mut fbo);
        glBindFramebuffer(GL_FRAMEBUFFER, fbo);
        glFramebufferTexture2D(
            GL_FRAMEBUFFER,
            GL_COLOR_ATTACHMENT0,
            GL_TEXTURE_RECTANGLE,
            gl_texture,
            0,
        );
        let status = glCheckFramebufferStatus(GL_FRAMEBUFFER);
        if status != GL_FRAMEBUFFER_COMPLETE {
            glDeleteFramebuffers(1, &fbo);
            glDeleteTextures(1, &gl_texture);
            return Err(format!("incomplete framebuffer (0x{status:x})"));
        }
    }

    let descriptor = MTLTextureDescriptor::new();
    descriptor.setPixelFormat(MTLPixelFormat::BGRA8Unorm);
    unsafe {
        descriptor.setWidth(w as usize);
        descriptor.setHeight(h as usize);
    }
    descriptor.setStorageMode(MTLStorageMode::Shared); // required for IOSurface-backed texture
    descriptor.setUsage(MTLTextureUsage::ShaderRead);
    let Some(metal_texture) =
        device.newTextureWithDescriptor_iosurface_plane(&descriptor, &io_surface, 0)
    else {
        unsafe {
            glDeleteFramebuffers(1, &fbo);
            glDeleteTextures(1, &gl_texture);
        }
        return Err("Metal texture from IOSurface failed".into());
    };

    Ok(Target {
        w,
        h,
        fbo,
        gl_texture,
        _io_surface: io_surface,
        metal_texture,
    })
}
