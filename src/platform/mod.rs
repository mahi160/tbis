//! Everything that differs per OS. Each OS folder (`macos/`, `linux/`, `windows/`)
//! exposes the same modules, and the rest of the app only reaches them through here:
//!
//! - `init`: process-wide setup at startup, returning a `Runtime` to keep alive
//! - `video`: on-screen surface + GL present for mpv (`Surface`, `Renderer`; ADR-0001)
//! - `now_playing`: media keys and the system Now Playing widget (`NowPlaying`)
//! - `power`: keep the display awake while playing (`Awake`)
//! - `cursor`: `hide()` the pointer during playback
//! - `window`: one-time native window setup (`prepare`, `hide_title_buttons`, `lock_aspect`)
//! - `update`: self-update glue (`ASSET_SUFFIX`, `bundle_path`, `install`, `relaunch`)
//!
//! `noop` holds do-nothing versions for OSes that haven't implemented them yet: `window` is
//! shared (Linux and Windows), while `Runtime`, `init`, `now_playing`, `cursor` and `update`
//! are Linux-only. An OS replaces one by adding its own file.

pub mod log;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(target_os = "macos"))]
mod noop;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(target_os = "macos")]
pub use macos::*;
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!("tbis supports macOS, Linux and Windows only");
