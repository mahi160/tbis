use std::rc::Rc;

use libmpv_sys::mpv_render_context;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use windows::Win32::Foundation::{HANDLE, HMODULE, HWND};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::DirectComposition::*;
use windows::Win32::Graphics::Dxgi::{Common::*, *};
use windows::Win32::Graphics::OpenGL::{GL_TEXTURE_2D, glDeleteTextures, glGenTextures};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
use windows::Win32::UI::WindowsAndMessaging::IsIconic;
use windows::core::Interface;

use super::wgl::{self, COLOR_ATTACHMENT0, FRAMEBUFFER, FRAMEBUFFER_COMPLETE, Functions, Gl};
use crate::mpv::{self, Extent, Handle};

pub const HWDEC: &str = "d3d11va,d3d11va-copy,no";

/// Whether the surface can move to another window (Picture-in-Picture).
pub const SUPPORTS_PIP: bool = false;

pub struct Surface;

impl Surface {
    pub fn attach(
        window: &(impl HasWindowHandle + HasDisplayHandle),
    ) -> Result<(Self, Renderer), String> {
        let hwnd = super::hwnd(window)?;
        Ok((
            Self,
            Renderer {
                hwnd,
                runtime: None,
            },
        ))
    }

    pub fn move_to(&mut self, _: &(impl HasWindowHandle + HasDisplayHandle)) -> Result<(), String> {
        Err("Picture-in-Picture is not implemented on Windows".into())
    }
}

pub struct Renderer {
    hwnd: HWND,
    runtime: Option<Runtime>,
}

// Only moved into the render thread before init; all GPU/GL objects are created and
// dropped there. HWND is borrowed, not owned: Mpv joins rendering before Surface drops;
// the caller must keep the gpui window alive through Mpv teardown.
unsafe impl Send for Renderer {}

impl Renderer {
    pub fn init(&mut self, mpv: Handle) -> Result<*mut mpv_render_context, String> {
        self.runtime = Some(Runtime::new(self.hwnd)?);
        mpv::create_gl_render_context(mpv, wgl::get_proc, None)
    }

    /// No per-phase timings here: `Phases` stays empty.
    pub fn draw(
        &mut self,
        ctx: *mut mpv_render_context,
        extent: Extent,
        _: Option<&mut mpv::Phases>,
    ) -> Result<bool, String> {
        if unsafe { IsIconic(self.hwnd) }.as_bool() {
            mpv::skip_frame(ctx);
            return Ok(false);
        }
        match &mut self.runtime {
            Some(runtime) => runtime.draw(ctx, extent),
            None => Err("Windows renderer is not initialized".into()),
        }
        .map_err(|error| {
            format!("Windows video failed; close and reopen the player to retry: {error}")
        })
    }
}

struct Apartment;

impl Apartment {
    fn new() -> Result<Self, String> {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
            .ok()
            .map_err(|e| e.to_string())?;
        Ok(Self)
    }
}

impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

struct Runtime {
    target: Option<Texture>,
    composition: Composition,
    interop: Rc<Interop>,
    _gl: Gl,
    _apartment: Apartment,
    extent: Extent,
}

struct Composition {
    target: IDCompositionTarget,
    _visual: IDCompositionVisual,
    device: IDCompositionDevice,
    swapchain: IDXGISwapChain1,
}

impl Drop for Composition {
    fn drop(&mut self) {
        unsafe {
            let _ = self.target.SetRoot(None);
            let _ = self.device.Commit();
        }
    }
}

struct Interop {
    handle: HANDLE,
    functions: Functions,
    device: ID3D11Device,
    context: ID3D11DeviceContext,
}

impl Drop for Interop {
    fn drop(&mut self) {
        if !unsafe { (self.functions.close)(self.handle) }.as_bool() {
            crate::log::write("wglDXCloseDeviceNV failed during teardown");
        }
    }
}

impl Runtime {
    fn new(hwnd: HWND) -> Result<Self, String> {
        let apartment = Apartment::new()?;
        let mut device = None;
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                None,
            )
        }
        .map_err(|e| e.to_string())?;
        let device = device.ok_or("D3D11CreateDevice returned no device")?;
        let context = unsafe { device.GetImmediateContext() }.map_err(|e| e.to_string())?;
        let dxgi: IDXGIDevice = device.cast().map_err(|e| e.to_string())?;
        let adapter = unsafe { dxgi.GetAdapter() }.map_err(|e| e.to_string())?;
        // logged before the GL setup, whose failures are usually a GL/D3D adapter mismatch
        let name = unsafe { adapter.GetDesc() }.map_or_else(
            |_| "unknown".into(),
            |desc| {
                let text = &desc.Description;
                let end = text.iter().position(|&c| c == 0).unwrap_or(text.len());
                String::from_utf16_lossy(&text[..end])
            },
        );
        crate::log::write(&format!("windows video: D3D adapter {name}"));
        let gl = Gl::new()?;
        let functions = Functions::load()?;
        let handle = unsafe { (functions.open)(device.as_raw()) };
        if handle.0.is_null() {
            return Err(
                "wglDXOpenDeviceNV failed (GL/D3D adapter mismatch or unsupported driver)".into(),
            );
        }
        let interop = Rc::new(Interop {
            handle,
            functions,
            device,
            context,
        });
        let factory: IDXGIFactory2 = unsafe { adapter.GetParent() }.map_err(|e| e.to_string())?;
        let desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: 1,
            Height: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: DXGI_SCALING_STRETCH,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode: DXGI_ALPHA_MODE_IGNORE,
            ..Default::default()
        };
        let swapchain =
            unsafe { factory.CreateSwapChainForComposition(&interop.device, &desc, None) }
                .map_err(|e| e.to_string())?;
        let device: IDCompositionDevice =
            unsafe { DCompositionCreateDevice(&dxgi) }.map_err(|e| e.to_string())?;
        let target =
            unsafe { device.CreateTargetForHwnd(hwnd, false) }.map_err(|e| e.to_string())?;
        let visual = unsafe { device.CreateVisual() }.map_err(|e| e.to_string())?;
        let composition = Composition {
            target,
            _visual: visual,
            device,
            swapchain,
        };
        unsafe {
            composition
                ._visual
                .SetContent(&composition.swapchain)
                .map_err(|e| e.to_string())?;
            composition
                .target
                .SetRoot(&composition._visual)
                .map_err(|e| e.to_string())?;
            composition.device.Commit().map_err(|e| e.to_string())?;
        }
        Ok(Self {
            target: None,
            composition,
            interop,
            _gl: gl,
            _apartment: apartment,
            extent: Extent::default(),
        })
    }

    fn draw(&mut self, ctx: *mut mpv_render_context, extent: Extent) -> Result<bool, String> {
        unsafe { self.interop.device.GetDeviceRemovedReason() }.map_err(|e| e.to_string())?;
        if self.extent.width != extent.width || self.extent.height != extent.height {
            self.target = None;
            unsafe {
                self.composition.swapchain.ResizeBuffers(
                    2,
                    extent.width as u32,
                    extent.height as u32,
                    DXGI_FORMAT_B8G8R8A8_UNORM,
                    DXGI_SWAP_CHAIN_FLAG::default(),
                )
            }
            .map_err(|e| e.to_string())?;
            self.target = Some(Texture::new(self.interop.clone(), extent)?);
            self.extent = extent;
        }
        let target = self
            .target
            .as_mut()
            .ok_or("Windows video texture is missing")?;
        target.lock()?;
        let functions = self.interop.functions;
        unsafe { (functions.bind_fbo)(FRAMEBUFFER, target.fbo) };
        let complete = unsafe { (functions.status)(FRAMEBUFFER) } == FRAMEBUFFER_COMPLETE;
        let drawn =
            complete && mpv::render_gl(ctx, target.fbo as i32, extent.width, extent.height, false);
        unsafe { (functions.bind_fbo)(FRAMEBUFFER, 0) };
        target.unlock()?;
        if !complete {
            return Err("DX interop framebuffer is incomplete".into());
        }
        if !drawn {
            return Err("mpv OpenGL rendering failed".into());
        }
        let backbuffer: ID3D11Texture2D =
            unsafe { self.composition.swapchain.GetBuffer(0) }.map_err(|e| e.to_string())?;
        unsafe {
            self.interop
                .context
                .CopyResource(&backbuffer, &target.texture);
            self.composition
                .swapchain
                .Present(1, DXGI_PRESENT::default())
                .ok()
        }
        .map_err(|e| e.to_string())?;
        Ok(true)
    }
}

struct Texture {
    interop: Rc<Interop>,
    texture: ID3D11Texture2D,
    object: HANDLE,
    name: u32,
    fbo: u32,
    locked: bool,
}

impl Texture {
    fn new(interop: Rc<Interop>, extent: Extent) -> Result<Self, String> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: extent.width as u32,
            Height: extent.height as u32,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: (D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE).0 as u32,
            ..Default::default()
        };
        let mut texture = None;
        unsafe {
            interop
                .device
                .CreateTexture2D(&desc, None, Some(&mut texture))
        }
        .map_err(|e| e.to_string())?;
        let texture = texture.ok_or("CreateTexture2D returned no texture")?;
        let mut target = Self {
            interop,
            texture,
            object: HANDLE::default(),
            name: 0,
            fbo: 0,
            locked: false,
        };
        let functions = target.interop.functions;
        unsafe {
            glGenTextures(1, &mut target.name);
            (functions.gen_fbo)(1, &mut target.fbo);
        }
        if target.name == 0 || target.fbo == 0 {
            return Err("could not allocate video GL objects".into());
        }
        target.object = unsafe {
            (functions.register)(
                target.interop.handle,
                target.texture.as_raw(),
                target.name,
                GL_TEXTURE_2D,
                0x0002,
            )
        };
        if target.object.0.is_null() {
            return Err("wglDXRegisterObjectNV failed".into());
        }
        target.lock()?;
        unsafe {
            (functions.bind_fbo)(FRAMEBUFFER, target.fbo);
            (functions.attach)(
                FRAMEBUFFER,
                COLOR_ATTACHMENT0,
                GL_TEXTURE_2D,
                target.name,
                0,
            );
            (functions.bind_fbo)(FRAMEBUFFER, 0);
        }
        target.unlock()?;
        Ok(target)
    }

    fn lock(&mut self) -> Result<(), String> {
        if !unsafe { (self.interop.functions.lock)(self.interop.handle, 1, &mut self.object) }
            .as_bool()
        {
            return Err("wglDXLockObjectsNV failed".into());
        }
        self.locked = true;
        Ok(())
    }

    fn unlock(&mut self) -> Result<(), String> {
        if !unsafe { (self.interop.functions.unlock)(self.interop.handle, 1, &mut self.object) }
            .as_bool()
        {
            return Err("wglDXUnlockObjectsNV failed".into());
        }
        self.locked = false;
        Ok(())
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        if self.locked {
            let _ = self.unlock();
        }
        unsafe {
            let functions = self.interop.functions;
            if self.fbo != 0 {
                (functions.delete_fbo)(1, &self.fbo);
            }
            if !self.object.0.is_null()
                && !(functions.unregister)(self.interop.handle, self.object).as_bool()
            {
                crate::log::write("wglDXUnregisterObjectNV failed during teardown");
            }
            if self.name != 0 {
                glDeleteTextures(1, &self.name);
            }
        }
    }
}
