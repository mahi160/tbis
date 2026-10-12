//! Linux: Wayland and X11. Not implemented yet: media keys (MPRIS), cursor hiding, window
//! setup. They use `noop` until they get a file here.

pub use super::noop::{cursor, now_playing, window};

pub mod power;
pub mod video;
