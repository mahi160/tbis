use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result};

use crate::update::bundle;

pub fn asset_suffix() -> Result<&'static str> {
    Ok("-windows-x86_64.zip")
}

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

pub fn install(bundle: &Path, zip: &[u8]) -> Result<()> {
    bundle::install(bundle, zip)
}

pub fn relaunch() -> Result<()> {
    // after the swap the running exe reports its renamed-aside name; launch its replacement
    let exe = std::env::current_exe()?.with_file_name("tbis.exe");
    Command::new(&exe)
        .current_dir(exe.parent().context("tbis.exe has no folder")?)
        .spawn()
        .context("can't start the updated tbis")?;
    Ok(())
}
