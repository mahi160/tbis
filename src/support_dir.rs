//! Per-user data shared by session/settings, fonts, shaders and mpv configuration.
//! App bundles use `tbis`; unbundled runs (including `cargo run --release`) use
//! `tbis-dev`, so development never reads or overwrites the installed app's data.

use std::path::PathBuf;

pub fn app_support_dir() -> PathBuf {
    let name = if crate::update::bundle_path().is_some() {
        "tbis"
    } else {
        "tbis-dev"
    };
    dirs::data_dir()
        .expect("no per-user data directory")
        .join(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_runs_use_separate_data() {
        assert_eq!(app_support_dir(), dirs::data_dir().unwrap().join("tbis-dev"));
    }
}
