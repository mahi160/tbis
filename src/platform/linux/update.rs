use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

pub fn asset_suffix() -> Result<&'static str> {
    bail!("automatic updates are not supported on Linux")
}

pub fn bundle_path() -> Option<PathBuf> {
    None
}

pub fn install(_: &Path, _: &[u8]) -> Result<()> {
    bail!("automatic updates are not supported on Linux")
}

pub fn relaunch() -> Result<()> {
    bail!("automatic updates are not supported on Linux")
}
