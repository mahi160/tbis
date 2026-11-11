//! macOS: AppKit, Metal and the Media Player framework.

pub mod cursor;
pub mod now_playing;
pub mod power;
pub mod update;
pub mod video;
pub mod window;

/// Process-wide setup; nothing to do on macOS.
pub struct Runtime;

pub fn init() -> Runtime {
    Runtime
}
