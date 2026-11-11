use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result};

use crate::update::bundle;

pub const ASSET_SUFFIX: &str = "-windows-x86_64.zip";

/// The install folder when running an installed `tbis.exe` beside its `libmpv-2.dll`; `None`
/// for `cargo run`.
pub fn bundle_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    if !exe.file_name()?.to_str()?.eq_ignore_ascii_case("tbis.exe") {
        return None;
    }
    let bundle = exe.parent()?;
    bundle
        .join("libmpv-2.dll")
        .is_file()
        .then(|| bundle.to_path_buf())
}

/// Swaps `zip` into `bundle` in-process (see `update::bundle`): the running files are
/// renamed aside, never overwritten.
pub fn install(bundle: &Path, zip: &[u8]) -> Result<()> {
    bundle::install(bundle, zip)
}

/// Starts the updated `tbis.exe`.
pub fn relaunch() -> Result<()> {
    // after the swap the running exe reports its renamed-aside name; launch its replacement
    let exe = std::env::current_exe()?.with_file_name("tbis.exe");
    Command::new(&exe)
        .current_dir(exe.parent().context("tbis.exe has no folder")?)
        .spawn()
        .context("can't start the updated tbis")?;
    Ok(())
}
