//! `~/Library/Application Support/tbis`, shared by `config.rs` (session/settings) and
//! `fonts.rs` (files mpv/libass need on disk). One panics on a missing `HOME`, the
//! other falls back to `None`, so each still wraps this with its own failure handling.

use std::path::PathBuf;

pub fn app_support_dir() -> PathBuf {
    let home = std::env::var_os("HOME").expect("HOME is not set");
    PathBuf::from(home).join("Library/Application Support/tbis")
}
