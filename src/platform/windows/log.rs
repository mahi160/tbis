use crate::log;

/// Points the process's stderr at `tbis.log` in the app data folder, which stays writable
/// when the install folder is not. Rust's stderr looks up the std handle on every write, so
/// every `log::write` lands in the file. The previous run's log is kept as
/// `tbis.previous.log`, so relaunching after a crash keeps the evidence. If the file can't
/// be created stderr is left alone.
pub fn init() {
    std::panic::set_hook(Box::new(|info| {
        let message = info
            .payload()
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| info.payload().downcast_ref::<&str>().copied())
            .unwrap_or("non-string panic");
        let message = if log::is_private(message) {
            "[private panic text omitted]"
        } else {
            message
        };
        let location = info.location().map(ToString::to_string).unwrap_or_default();
        log::write(&format!("panic at {location}: {message}"));
    }));
    let dir = crate::support_dir::app_support_dir();
    let path = dir.join("tbis.log");
    let _ = std::fs::create_dir_all(&dir);
    // replaces the older previous log, so exactly one is kept
    let _ = std::fs::rename(&path, dir.join("tbis.previous.log"));
    let Ok(file) = std::fs::File::create(path) else {
        return;
    };
    use std::os::windows::io::IntoRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Console::{STD_ERROR_HANDLE, SetStdHandle};
    let handle = file.into_raw_handle();
    // The handle stays open for the life of the process.
    let _ = unsafe { SetStdHandle(STD_ERROR_HANDLE, HANDLE(handle)) };
    let version = env!("CARGO_PKG_VERSION");
    log::write(&format!("tbis {version} Windows runtime"));
}
