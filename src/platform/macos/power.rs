//! Holds off display/idle sleep while alive (macOS power assertion via NSProcessInfo).

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::{NSActivityOptions, NSObjectProtocol, NSProcessInfo, ns_string};

pub struct Awake(Retained<ProtocolObject<dyn NSObjectProtocol>>);

impl Awake {
    pub fn new() -> Self {
        Self(
            NSProcessInfo::processInfo().beginActivityWithOptions_reason(
                NSActivityOptions::UserInitiated | NSActivityOptions::IdleDisplaySleepDisabled,
                ns_string!("Playing video"),
            ),
        )
    }
}

impl Drop for Awake {
    fn drop(&mut self) {
        // SAFETY: token came from beginActivityWithOptions_reason
        unsafe { NSProcessInfo::processInfo().endActivity(&self.0) };
    }
}
