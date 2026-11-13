//! macOS: AppKit, Metal and the Media Player framework.

pub mod cursor;
pub mod now_playing;
pub mod power;
pub mod update;
pub mod video;
pub mod window;

/// Process-wide setup; nothing to hold on macOS.
pub struct Runtime;

/// Clears update staging an earlier run left beside the installed app.
pub fn init() -> Runtime {
    if let Some(bundle) = update::bundle_path() {
        update::clean_up_stale_staging(&bundle);
    }
    Runtime
}
