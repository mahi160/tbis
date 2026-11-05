use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Power::{
    PowerClearRequest, PowerCreateRequest, PowerRequestDisplayRequired, PowerRequestSystemRequired,
    PowerSetRequest,
};
use windows::Win32::System::Threading::{
    POWER_REQUEST_CONTEXT_SIMPLE_STRING, REASON_CONTEXT, REASON_CONTEXT_0,
};
use windows::core::PWSTR;

pub struct Awake {
    handle: Option<HANDLE>,
    display: bool,
    system: bool,
}

impl Awake {
    pub fn new() -> Self {
        let mut this = Self {
            handle: None,
            display: false,
            system: false,
        };
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
            this.handle = Some(handle);
            unsafe { PowerSetRequest(handle, PowerRequestDisplayRequired)? };
            this.display = true;
            unsafe { PowerSetRequest(handle, PowerRequestSystemRequired)? };
            this.system = true;
            Ok(())
        })();
        if let Err(error) = result {
            crate::platform::log::write(&format!("power request failed: {:?}", error.code()));
        }
        this
    }
}

impl Drop for Awake {
    fn drop(&mut self) {
        if let Some(handle) = self.handle {
            unsafe {
                if self.display {
                    let _ = PowerClearRequest(handle, PowerRequestDisplayRequired);
                }
                if self.system {
                    let _ = PowerClearRequest(handle, PowerRequestSystemRequired);
                }
                let _ = CloseHandle(handle);
            }
        }
    }
}
