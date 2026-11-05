pub use super::noop::window;

pub mod cursor;
pub mod now_playing;
pub mod power;
pub mod update;
pub mod video;
mod wgl;

pub struct Runtime;

impl Runtime {
    pub fn new() -> Result<Self, String> {
        unsafe {
            windows::Win32::System::WinRT::RoInitialize(
                windows::Win32::System::WinRT::RO_INIT_SINGLETHREADED,
            )
        }
        .map_err(|error| error.to_string())?;
        Ok(Self)
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        unsafe { windows::Win32::System::WinRT::RoUninitialize() };
    }
}

pub(super) fn hwnd(
    window: &impl raw_window_handle::HasWindowHandle,
) -> Result<windows::Win32::Foundation::HWND, String> {
    match window
        .window_handle()
        .map_err(|error| error.to_string())?
        .as_raw()
    {
        raw_window_handle::RawWindowHandle::Win32(handle) => {
            Ok(windows::Win32::Foundation::HWND(handle.hwnd.get() as *mut _))
        }
        _ => Err("expected a Win32 window".into()),
    }
}
