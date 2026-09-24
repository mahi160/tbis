//! App-wide font: Inter (assets/fonts/inter/LICENSE-OFL), embedded so it renders
//! regardless of what's installed on the machine.
//!
//! Used two ways: [`embed`] registers it with GPUI's text system for the UI;
//! [`extract_dir`] writes the same files to disk so libmpv/libass (mpv.rs, pip.rs)
//! can pick it up as `sub-font` via `sub-fonts-dir` -- libass reads font files, not
//! in-memory bytes.

use std::path::PathBuf;

/// (file name, bytes) pairs; the name is also the on-disk name [`extract_dir`] writes.
const FILES: &[(&str, &[u8])] = &[
    (
        "Inter-Regular.ttf",
        include_bytes!("../assets/fonts/inter/Inter-Regular.ttf"),
    ),
    (
        "Inter-Medium.ttf",
        include_bytes!("../assets/fonts/inter/Inter-Medium.ttf"),
    ),
    (
        "Inter-SemiBold.ttf",
        include_bytes!("../assets/fonts/inter/Inter-SemiBold.ttf"),
    ),
    (
        "Inter-Bold.ttf",
        include_bytes!("../assets/fonts/inter/Inter-Bold.ttf"),
    ),
];

/// The family name every embedded file above resolves to.
pub const FAMILY: &str = "Inter";

/// Registers [`FILES`] with GPUI's text system so `theme.font.family = "Inter"` renders.
pub fn embed(cx: &gpui_kit::App) -> gpui_kit::Result<()> {
    cx.text_system().add_fonts(
        FILES
            .iter()
            .map(|(_, bytes)| std::borrow::Cow::Borrowed(*bytes))
            .collect(),
    )
}

/// Writes [`FILES`] under Application Support and returns that directory, for mpv's
/// `sub-fonts-dir`. Best-effort: on write failure, subtitles just fall back to
/// mpv's own default font.
pub fn extract_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let dir = PathBuf::from(home).join("Library/Application Support/tbis/fonts");
    std::fs::create_dir_all(&dir).ok()?;
    for (name, bytes) in FILES {
        std::fs::write(dir.join(name), bytes).ok()?;
    }
    Some(dir)
}
