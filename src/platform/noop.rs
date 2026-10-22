//! Do-nothing platform pieces; see `platform/mod.rs`.

pub mod now_playing {
    use futures::channel::mpsc::UnboundedSender;

    use crate::now_playing::{Info, RemoteCommand};

    pub struct NowPlaying;

    impl NowPlaying {
        pub fn new(_: UnboundedSender<RemoteCommand>, _: f64) -> Self {
            Self
        }
        pub fn update(&self, _: &Info) {}
        pub fn set_artwork(&self, _: &[u8]) {}
        pub fn set_episode_nav(&self, _: bool, _: bool) {}
    }
}

pub mod power {
    pub struct Awake;

    impl Awake {
        pub fn new() -> Self {
            Self
        }
    }
}

pub mod cursor {
    pub fn hide() {}
}

pub mod window {
    pub fn prepare(_: &gpui_kit::Window) {}
    pub fn hide_title_buttons(_: &gpui_kit::Window) {}
    pub fn lock_aspect(_: &gpui_kit::Window, _: f32, _: f32) {}
}
