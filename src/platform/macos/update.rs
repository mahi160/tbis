use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, anyhow, bail, ensure};

pub fn asset_suffix() -> Result<&'static str> {
    Ok("-macos-arm64.zip")
}

pub fn bundle_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let bundle = exe.parent()?.parent()?.parent()?;
    (bundle.extension()? == "app").then(|| bundle.to_path_buf())
}

pub fn install(bundle: &Path, zip: &[u8]) -> Result<()> {
    let parent = bundle.parent().context("app has no parent folder")?;
    let staging = parent.join(format!(".tbis-update-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&staging)
        .with_context(|| format!("can't write to {}", parent.display()))?;
    let result = swap_in(bundle, &staging, zip);
    if (result.is_ok() || !staging.join("old.app").exists())
        && let Err(err) = std::fs::remove_dir_all(&staging)
    {
        eprintln!("update cleanup {}: {err}", staging.display());
    }
    result
}

fn swap_in(bundle: &Path, staging: &Path, zip: &[u8]) -> Result<()> {
    let zip_path = staging.join("tbis.zip");
    std::fs::write(&zip_path, zip)?;
    let extract = [
        OsStr::new("-x"),
        OsStr::new("-k"),
        zip_path.as_os_str(),
        staging.as_os_str(),
    ];
    run("ditto", &extract)?;
    let new = staging.join("tbis.app");
    let verify = [
        OsStr::new("--verify"),
        OsStr::new("--deep"),
        OsStr::new("--strict"),
        new.as_os_str(),
    ];
    run("codesign", &verify).context("downloaded app failed its signature check")?;

    let old = staging.join("old.app");
    std::fs::rename(bundle, &old).context("can't move the current app aside")?;
    if let Err(err) = std::fs::rename(&new, bundle) {
        if let Err(rollback) = std::fs::rename(&old, bundle) {
            bail!(
                "can't move the new app into place: {err}; rollback failed: {rollback}; old app retained at {}",
                old.display()
            );
        }
        bail!("can't move the new app into place: {err}; restored the current app");
    }
    Ok(())
}

fn run(program: &str, args: &[&OsStr]) -> Result<String> {
    let output = Command::new(program).args(args).output()?;
    ensure!(
        output.status.success(),
        "{program} failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn relaunch() -> Result<()> {
    let bundle = bundle_path().ok_or_else(|| anyhow!("not running from tbis.app"))?;
    Command::new("/bin/sh")
        .args([
            "-c",
            r#"while kill -0 "$0" 2>/dev/null; do sleep 0.2; done; open "$1""#,
        ])
        .arg(std::process::id().to_string())
        .arg(bundle)
        .spawn()?;
    Ok(())
}
