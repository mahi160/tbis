//! Media-key and system "Now Playing" commands for the Player. Registering handlers makes
//! tbis the "now playing" app, so keyboard ⏯ and headset taps reach it. The OS side lives
//! in `platform::now_playing`; this file holds the types both sides share.

/// Media-key / Control Center request, forwarded to the Player.
#[derive(Clone, Copy, Debug)]
pub enum RemoteCommand {
    Play,
    Pause,
    Toggle,
    SkipForward,
    SkipBackward,
    Previous,
    Next,
    /// Absolute position in seconds (Control Center scrubber).
    Seek(f64),
}

/// What Now Playing shows; `elapsed` + `rate` let the OS extrapolate the position.
#[derive(Clone, Debug, PartialEq)]
pub struct Info {
    pub title: String,
    pub subtitle: Option<String>,
    pub duration: f64,
    pub elapsed: f64,
    /// 0 while paused, else playback speed.
    pub rate: f64,
}

pub use crate::platform::now_playing::NowPlaying;
