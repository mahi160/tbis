use std::ffi::{CStr, c_char, c_void};
use std::sync::OnceLock;

use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetDC, HDC, ReleaseDC};
use windows::Win32::Graphics::OpenGL::*;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{BOOL, PCSTR, w};

pub(super) const FRAMEBUFFER: u32 = 0x8D40;
pub(super) const COLOR_ATTACHMENT0: u32 = 0x8CE0;
pub(super) const FRAMEBUFFER_COMPLETE: u32 = 0x8CD5;

macro_rules! load {
    ($name:literal, $ty:ty) => {{
        let pointer =
            unsafe { get_proc(std::ptr::null_mut(), concat!($name, "\0").as_ptr().cast()) };
        if pointer.is_null() {
            return Err(concat!("missing GL/WGL entry point: ", $name).into());
        }
        unsafe { std::mem::transmute::<*mut c_void, $ty>(pointer) }
    }};
}
pub(super) unsafe extern "C" fn get_proc(_: *mut c_void, name: *const c_char) -> *mut c_void {
    let name = PCSTR(name.cast());
    if let Some(proc) = unsafe { wglGetProcAddress(name) } {
        let pointer = proc as *mut c_void;
        if !matches!(pointer as isize, -1..=3) {
            return pointer;
        }
    }
    unsafe { GetModuleHandleW(w!("opengl32.dll")) }
        .ok()
        .and_then(|module| unsafe { GetProcAddress(module, name) })
        .map_or(std::ptr::null_mut(), |proc| proc as *mut c_void)
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

pub(super) struct Gl {
    window: HWND,
    dc: HDC,
    context: HGLRC,
}

impl Gl {
    pub fn new() -> Result<Self, String> {
        static CLASS: OnceLock<Result<(), String>> = OnceLock::new();
        CLASS
            .get_or_init(|| {
                let class = WNDCLASSW {
                    style: CS_OWNDC,
                    lpfnWndProc: Some(window_proc),
                    hInstance: unsafe { GetModuleHandleW(None) }
                        .map_err(|e| e.to_string())?
                        .into(),
                    lpszClassName: w!("tbis-video-gl"),
                    ..Default::default()
                };
                if unsafe { RegisterClassW(&class) } == 0 {
                    Err("could not register video GL window class".into())
                } else {
                    Ok(())
                }
            })
            .clone()?;
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("tbis-video-gl"),
                w!(""),
                WS_POPUP,
                0,
                0,
                1,
                1,
                None,
                None,
                Some(GetModuleHandleW(None).map_err(|e| e.to_string())?.into()),
                None,
            )
        }
        .map_err(|e| e.to_string())?;
        let mut gl = Self {
            window,
            dc: HDC::default(),
            context: HGLRC::default(),
        };
        gl.dc = unsafe { GetDC(Some(window)) };
        if gl.dc.0.is_null() {
            return Err("video GetDC failed".into());
        }
        let format = PIXELFORMATDESCRIPTOR {
            nSize: std::mem::size_of::<PIXELFORMATDESCRIPTOR>() as u16,
            nVersion: 1,
            dwFlags: PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER,
            iPixelType: PFD_TYPE_RGBA,
            cColorBits: 32,
            cAlphaBits: 8,
            ..Default::default()
        };
        let index = unsafe { ChoosePixelFormat(gl.dc, &format) };
        if index == 0 {
            return Err("video ChoosePixelFormat failed".into());
        }
        unsafe { SetPixelFormat(gl.dc, index, &format) }.map_err(|e| e.to_string())?;
        gl.context = unsafe { wglCreateContext(gl.dc) }.map_err(|e| e.to_string())?;
        unsafe { wglMakeCurrent(gl.dc, gl.context) }.map_err(|e| e.to_string())?;
        let create = load!(
            "wglCreateContextAttribsARB",
            unsafe extern "system" fn(HDC, HGLRC, *const i32) -> HGLRC
        );
        let attributes = [0x2091, 3, 0x2092, 3, 0x9126, 1, 0];
        let context = unsafe { create(gl.dc, HGLRC::default(), attributes.as_ptr()) };
        if context.0.is_null() {
            return Err("could not create OpenGL 3.3 core context".into());
        }
        let old = std::mem::replace(&mut gl.context, context);
        unsafe {
            let _ = wglMakeCurrent(HDC::default(), HGLRC::default());
            let _ = wglDeleteContext(old);
            wglMakeCurrent(gl.dc, context).map_err(|e| e.to_string())?;
        }
        // logged before the interop check so a failed start still names the driver
        let gl_string = |name| {
            let text = unsafe { glGetString(name) };
            if text.is_null() {
                "unknown".into()
            } else {
                unsafe { CStr::from_ptr(text.cast()) }.to_string_lossy()
            }
        };
        crate::log::write(&format!(
            "windows video: GL {} | {} | {}",
            gl_string(GL_VENDOR),
            gl_string(GL_RENDERER),
            gl_string(GL_VERSION)
        ));
        let extensions = load!(
            "wglGetExtensionsStringARB",
            unsafe extern "system" fn(HDC) -> *const c_char
        );
        let extensions = unsafe { extensions(gl.dc) };
        if extensions.is_null()
            || !unsafe { CStr::from_ptr(extensions) }
                .to_bytes()
                .split(|byte| *byte == b' ')
                .any(|name| name == b"WGL_NV_DX_interop2")
        {
            return Err("WGL_NV_DX_interop2 is required by the Windows video backend".into());
        }
        Ok(gl)
    }
}

impl Drop for Gl {
    fn drop(&mut self) {
        unsafe {
            if !self.context.0.is_null() {
                let _ = wglMakeCurrent(HDC::default(), HGLRC::default());
                let _ = wglDeleteContext(self.context);
            }
            if !self.dc.0.is_null() {
                ReleaseDC(Some(self.window), self.dc);
            }
            let _ = DestroyWindow(self.window);
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Functions {
    pub open: unsafe extern "system" fn(*mut c_void) -> HANDLE,
    pub close: unsafe extern "system" fn(HANDLE) -> BOOL,
    pub register: unsafe extern "system" fn(HANDLE, *mut c_void, u32, u32, u32) -> HANDLE,
    pub unregister: unsafe extern "system" fn(HANDLE, HANDLE) -> BOOL,
    pub lock: unsafe extern "system" fn(HANDLE, i32, *mut HANDLE) -> BOOL,
    pub unlock: unsafe extern "system" fn(HANDLE, i32, *mut HANDLE) -> BOOL,
    pub gen_fbo: unsafe extern "system" fn(i32, *mut u32),
    pub delete_fbo: unsafe extern "system" fn(i32, *const u32),
    pub bind_fbo: unsafe extern "system" fn(u32, u32),
    pub attach: unsafe extern "system" fn(u32, u32, u32, u32, i32),
    pub status: unsafe extern "system" fn(u32) -> u32,
}

impl Functions {
    pub fn load() -> Result<Self, String> {
        Ok(Self {
            open: load!(
                "wglDXOpenDeviceNV",
                unsafe extern "system" fn(*mut c_void) -> HANDLE
            ),
            close: load!(
                "wglDXCloseDeviceNV",
                unsafe extern "system" fn(HANDLE) -> BOOL
            ),
            register: load!(
                "wglDXRegisterObjectNV",
                unsafe extern "system" fn(HANDLE, *mut c_void, u32, u32, u32) -> HANDLE
            ),
            unregister: load!(
                "wglDXUnregisterObjectNV",
                unsafe extern "system" fn(HANDLE, HANDLE) -> BOOL
            ),
            lock: load!(
                "wglDXLockObjectsNV",
                unsafe extern "system" fn(HANDLE, i32, *mut HANDLE) -> BOOL
            ),
            unlock: load!(
                "wglDXUnlockObjectsNV",
                unsafe extern "system" fn(HANDLE, i32, *mut HANDLE) -> BOOL
            ),
            gen_fbo: load!(
                "glGenFramebuffers",
                unsafe extern "system" fn(i32, *mut u32)
            ),
            delete_fbo: load!(
                "glDeleteFramebuffers",
                unsafe extern "system" fn(i32, *const u32)
            ),
            bind_fbo: load!("glBindFramebuffer", unsafe extern "system" fn(u32, u32)),
            attach: load!(
                "glFramebufferTexture2D",
                unsafe extern "system" fn(u32, u32, u32, u32, i32)
            ),
            status: load!(
                "glCheckFramebufferStatus",
                unsafe extern "system" fn(u32) -> u32
            ),
        })
    }
}
