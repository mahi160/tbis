//! App-wide fonts: Inter (assets/fonts/inter/LICENSE-OFL) for UI text, JetBrains Mono
//! (assets/fonts/jetbrains-mono/LICENSE-OFL) for the app's `mono_font.family`.
//! Both embedded so they render regardless of what's installed on the machine.
//!
//! Used two ways: [`embed`] registers them with GPUI's text system for the UI;
//! [`extract_dir`] writes the Inter files to disk so libmpv/libass (mpv.rs, pip.rs)
//! can pick it up as `sub-font` via `sub-fonts-dir` -- libass reads font files, not
//! in-memory bytes.

use std::path::PathBuf;
use std::sync::OnceLock;

use crate::embed::embedded_by_name;

/// (file name, bytes) pairs; the name is also the on-disk name [`extract_dir`] writes.
const INTER_FILES: &[(&str, &[u8])] = &embedded_by_name!(
    "fonts/inter",
    [
        "Inter-Regular.ttf",
        "Inter-Medium.ttf",
        "Inter-SemiBold.ttf",
        "Inter-Bold.ttf",
    ]
);

/// (file name, bytes) pairs for the monospace family; UI-only, not written to disk.
const MONO_FILES: &[(&str, &[u8])] = &embedded_by_name!(
    "fonts/jetbrains-mono",
    [
        "JetBrainsMono-Regular.ttf",
        "JetBrainsMono-Medium.ttf",
        "JetBrainsMono-SemiBold.ttf",
        "JetBrainsMono-Bold.ttf",
    ]
);

/// The family name every embedded Inter file above resolves to.
pub const FAMILY: &str = "Inter";

/// Registers Inter and JetBrains Mono with GPUI's text system so `theme.font.family`
/// and `theme.mono_font.family` render.
pub fn embed(cx: &gpui_kit::App) -> gpui_kit::Result<()> {
    cx.text_system().add_fonts(
        INTER_FILES
            .iter()
            .chain(MONO_FILES)
            .map(|(_, bytes)| std::borrow::Cow::Borrowed(*bytes))
            .collect(),
    )
}

/// Writes [`INTER_FILES`] under Application Support and returns that directory, for
/// mpv's `sub-fonts-dir`. Best-effort: on write failure, subtitles just fall back to
/// mpv's own default font. Written once per process and cached; every Player open
/// and PiP start used to rewrite the ~1.67MB on the UI thread.
pub fn extract_dir() -> Option<PathBuf> {
    static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    DIR.get_or_init(write_dir).clone()
}

fn write_dir() -> Option<PathBuf> {
    let dir = crate::support_dir::app_support_dir().join("fonts");
    std::fs::create_dir_all(&dir).ok()?;
    for (name, bytes) in INTER_FILES {
        std::fs::write(dir.join(name), bytes).ok()?;
    }
    Some(dir)
}
