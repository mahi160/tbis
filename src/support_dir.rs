//! Per-user data dir (`~/Library/Application Support/tbis` on macOS, `$XDG_DATA_HOME/tbis` on
//! Linux, `%APPDATA%\tbis` on Windows), shared by `config.rs` (session/settings) and
//! `fonts.rs` (files mpv/libass need on disk). One panics on a missing dir, the
//! other falls back to `None`, so each still wraps this with its own failure handling.

use std::path::PathBuf;

pub fn app_support_dir() -> PathBuf {
    dirs::data_dir()
        .expect("no per-user data directory")
        .join("tbis")
}
