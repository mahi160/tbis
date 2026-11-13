//! Linux: Wayland and X11. Not implemented yet: media keys (MPRIS), cursor hiding, window
//! setup, self-update. They do nothing until they get a real implementation.

pub use super::noop::window;

pub mod power;
pub mod video;

/// Process-wide setup; nothing to do on Linux.
pub struct Runtime;

pub fn init() -> Runtime {
    Runtime
}

pub mod now_playing {
    use futures::channel::mpsc::UnboundedSender;

    use crate::now_playing::{Info, RemoteCommand};

    pub struct System;

    impl System {
        pub fn new(_: &gpui_kit::Window, _: UnboundedSender<RemoteCommand>, _: f64) -> Self {
            Self
        }
        pub fn update(&self, _: &Info) {}
        pub fn set_artwork(&self, _: &[u8]) {}
        pub fn set_episode_nav(&self, _: bool, _: bool) {}
    }
}

pub mod cursor {
    pub fn hide() {}
    pub fn show() {}
}

/// No self-update on Linux.
pub mod update {
    use crate::update::Installed;

    pub fn installed() -> Option<Box<dyn Installed>> {
        None
    }
}
