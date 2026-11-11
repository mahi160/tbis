use windows::Win32::UI::WindowsAndMessaging::{IDC_ARROW, LoadCursorW, SetCursor};

pub fn hide() {
    unsafe { SetCursor(None) };
}

pub fn show() {
    if let Ok(cursor) = unsafe { LoadCursorW(None, IDC_ARROW) } {
        unsafe { SetCursor(Some(cursor)) };
    }
}
