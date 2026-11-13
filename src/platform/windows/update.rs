use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context as _, Result};

use crate::update::{Installed, bundle};

/// The install folder of the running `tbis.exe`.
struct Folder(PathBuf);

/// The install folder when running an installed `tbis.exe` beside its `libmpv-2.dll`; `None`
/// for `cargo run`.
pub(super) fn bundle_path() -> Option<PathBuf> {
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

pub fn installed() -> Option<Box<dyn Installed>> {
    bundle_path().map(|folder| Box::new(Folder(folder)) as _)
}

impl Installed for Folder {
    fn asset_suffix(&self) -> &'static str {
        "-windows-x86_64.zip"
    }

    /// Swaps `zip` into the folder in-process (see `update::bundle`): the running files are
    /// renamed aside, never overwritten.
    fn install(&self, zip: &[u8]) -> Result<()> {
        bundle::install(&self.0, zip)
    }

    /// Starts the updated `tbis.exe`.
    fn relaunch(&self) -> Result<()> {
        // after the swap the running exe reports its renamed-aside name; launch its replacement
        let exe = std::env::current_exe()?.with_file_name("tbis.exe");
        Command::new(&exe)
            .current_dir(exe.parent().context("tbis.exe has no folder")?)
            .spawn()
            .context("can't start the updated tbis")?;
        Ok(())
    }
}
