//! Do-nothing platform pieces; see `platform/mod.rs`.

#[cfg(target_os = "linux")]
pub struct Runtime;

#[cfg(target_os = "linux")]
pub fn init() -> Runtime {
    Runtime
}

#[cfg(target_os = "linux")]
pub mod now_playing {
    use futures::channel::mpsc::UnboundedSender;

    use crate::now_playing::{Info, RemoteCommand};

    pub struct NowPlaying;

    impl NowPlaying {
        pub fn new(_: &gpui_kit::Window, _: UnboundedSender<RemoteCommand>, _: f64) -> Self {
            Self
        }
        pub fn update(&self, _: &Info) {}
        pub fn set_artwork(&self, _: &[u8]) {}
        pub fn set_episode_nav(&self, _: bool, _: bool) {}
    }
}

#[cfg(target_os = "linux")]
pub mod cursor {
    pub fn hide() {}
    pub fn show() {}
}

/// Linux has no self-update: `bundle_path` is `None`, so `install` and `relaunch` never run.
#[cfg(target_os = "linux")]
pub mod update {
    use std::path::{Path, PathBuf};

    use anyhow::Result;

    pub const ASSET_SUFFIX: &str = "";

    pub fn bundle_path() -> Option<PathBuf> {
        None
    }

    pub fn install(_: &Path, _: &[u8]) -> Result<()> {
        unreachable!("bundle_path is None on Linux")
    }

    pub fn relaunch() -> Result<()> {
        unreachable!("bundle_path is None on Linux")
    }
}

pub mod window {
    pub fn prepare(_: &gpui_kit::Window) {}
    pub fn hide_title_buttons(_: &gpui_kit::Window) {}
    pub fn lock_aspect(_: &gpui_kit::Window, _: f32, _: f32) {}
}
