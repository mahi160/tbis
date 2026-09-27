//! The app's one font, Onest (assets/fonts/onest/LICENSE-OFL), for UI text, numbers
//! (tabular figures, see app.rs), and subtitles. Embedded so it renders regardless of
//! what's installed on the machine.
//!
//! Used two ways: [`embed`] registers it with GPUI's text system for the UI;
//! [`extract_dir`] writes the files to disk so libmpv/libass (mpv.rs, pip.rs) can pick
//! it up as `sub-font` via `sub-fonts-dir` -- libass reads font files, not in-memory
//! bytes.

use std::path::PathBuf;
use std::sync::OnceLock;

use crate::embed::embedded_by_name;

/// (file name, bytes) pairs; the name is also the on-disk name [`extract_dir`] writes.
const FILES: &[(&str, &[u8])] = &embedded_by_name!(
    "fonts/onest",
    [
        "Onest-Regular.ttf",
        "Onest-Medium.ttf",
        "Onest-SemiBold.ttf",
        "Onest-Bold.ttf",
    ]
);

/// The family name every embedded file above resolves to.
pub const FAMILY: &str = "Onest";

/// Registers [`FILES`] with GPUI's text system so the themes' `font.family` renders.
pub fn embed(cx: &gpui_kit::App) -> gpui_kit::Result<()> {
    cx.text_system().add_fonts(
        FILES
            .iter()
            .map(|(_, bytes)| std::borrow::Cow::Borrowed(*bytes))
            .collect(),
    )
}

/// Writes [`FILES`] under Application Support and returns that directory, for
/// mpv's `sub-fonts-dir`. Best-effort: on write failure, subtitles just fall back to
/// mpv's own default font. Written once per process and cached, not on every Player
/// open or PiP start.
pub fn extract_dir() -> Option<PathBuf> {
    static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    DIR.get_or_init(write_dir).clone()
}

fn write_dir() -> Option<PathBuf> {
    let dir = crate::support_dir::app_support_dir().join("fonts");
    std::fs::create_dir_all(&dir).ok()?;
    for (name, bytes) in FILES {
        std::fs::write(dir.join(name), bytes).ok()?;
    }
    Some(dir)
}
