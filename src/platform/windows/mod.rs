pub use super::noop::window;

pub mod cursor;
pub mod now_playing;
pub mod power;
pub mod update;
pub mod video;
mod wgl;

/// Process-wide setup: log file, leftovers of a previous update (removed in the background), the `--self-test` flag (runs and exits), WinRT.
/// Keep the result alive for the whole run.
pub fn init() -> Runtime {
    super::log::init();
    if let Some(bundle) = update::bundle_path() {
        // right after an update the old process may still hold its files, so retry
        std::thread::spawn(move || {
            for _ in 0..10 {
                if crate::update::bundle::clean_up(&bundle) {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            super::log::write("update cleanup: leftover files could not be removed");
        });
    }
    if std::env::args_os().any(|argument| argument == "--self-test") {
        match crate::mpv::self_test() {
            Ok(()) => std::process::exit(0),
            Err(error) => {
                super::log::write(&error);
                std::process::exit(1);
            }
        }
    }
    Runtime::new().unwrap_or_else(|error| {
        super::log::write(&format!("Windows runtime initialization failed: {error}"));
        std::process::exit(1);
    })
}

pub struct Runtime;

impl Runtime {
    fn new() -> Result<Self, String> {
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
