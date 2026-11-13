use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Power::{
    PowerClearRequest, PowerCreateRequest, PowerRequestDisplayRequired, PowerRequestSystemRequired,
    PowerSetRequest,
};
use windows::Win32::System::Threading::{
    POWER_REQUEST_CONTEXT_SIMPLE_STRING, REASON_CONTEXT, REASON_CONTEXT_0,
};
use windows::core::PWSTR;

pub struct Awake(Option<HANDLE>);

impl Awake {
    pub fn new() -> Self {
        let mut this = Self(None);
        let mut reason: Vec<u16> = "Playing video\0".encode_utf16().collect();
        let context = REASON_CONTEXT {
            Version: 0,
            Flags: POWER_REQUEST_CONTEXT_SIMPLE_STRING,
            Reason: REASON_CONTEXT_0 {
                SimpleReasonString: PWSTR(reason.as_mut_ptr()),
            },
        };
        let result = (|| -> windows::core::Result<()> {
            let handle = unsafe { PowerCreateRequest(&context)? };
            this.0 = Some(handle);
            unsafe { PowerSetRequest(handle, PowerRequestDisplayRequired)? };
            unsafe { PowerSetRequest(handle, PowerRequestSystemRequired)? };
            Ok(())
        })();
        if let Err(error) = result {
            crate::log::write(&format!("power request failed: {:?}", error.code()));
        }
        this
    }
}

impl Drop for Awake {
    fn drop(&mut self) {
        if let Some(handle) = self.0 {
            // clearing a request that was never set just fails; closing the handle ends both
            unsafe {
                let _ = PowerClearRequest(handle, PowerRequestDisplayRequired);
                let _ = PowerClearRequest(handle, PowerRequestSystemRequired);
                let _ = CloseHandle(handle);
            }
        }
    }
}
