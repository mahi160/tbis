//! Windows. Not implemented yet: media keys (SMTC), sleep inhibit, cursor hiding, window
//! setup. They use `noop` until they get a file here.

pub use super::noop::{cursor, now_playing, power, window};

pub mod video;
