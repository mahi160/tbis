//! `embedded_by_path!`/`embedded_by_name!` bundle files under `assets/<dir>` into
//! `(key, bytes)` pairs, for `assets.rs`'s icon overrides and `fonts.rs`'s embedded
//! font files.

/// `embedded_by_path!("icons", ["a.svg", ...])` keys each entry `"<dir>/<name>"`,
/// matching the full path `AssetSource::load` is asked for (`assets.rs`).
macro_rules! embedded_by_path {
    ($dir:literal, [$($name:literal),+ $(,)?]) => {
        [$((concat!($dir, "/", $name), include_bytes!(concat!("../assets/", $dir, "/", $name)))),+]
    };
}
pub(crate) use embedded_by_path;

/// `embedded_by_name!("fonts/onest", ["Onest-Regular.ttf", ...])` keys each entry by
/// `<name>` alone, the on-disk file name `fonts::extract_dir` writes (`fonts.rs`).
macro_rules! embedded_by_name {
    ($dir:literal, [$($name:literal),+ $(,)?]) => {
        [$(($name, include_bytes!(concat!("../assets/", $dir, "/", $name)))),+]
    };
}
pub(crate) use embedded_by_name;
