use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail, ensure};

use crate::update::Installed;

/// The running `…/tbis.app`.
struct Bundle(PathBuf);

/// Staging folders sit beside the app as `.tbis-update-<uuid>`.
const STAGING_PREFIX: &str = ".tbis-update-";

/// `…/tbis.app` when running from one; `None` for `cargo run`.
pub(super) fn bundle_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let bundle = exe.parent()?.parent()?.parent()?;
    (bundle.extension()? == "app").then(|| bundle.to_path_buf())
}

pub fn installed() -> Option<Box<dyn Installed>> {
    bundle_path().map(|bundle| Box::new(Bundle(bundle)) as _)
}

/// Removes staging folders an earlier update left beside `bundle` (it crashed or was
/// killed mid-update), except one holding the only backup of the app.
pub(super) fn clean_up_stale_staging(bundle: &Path) {
    let Some(parent) = bundle.parent() else {
        return;
    };
    let entries = match std::fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(err) => {
            crate::log::write(&format!("update cleanup {}: {err}", parent.display()));
            return;
        }
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with(STAGING_PREFIX) {
            cleanup_staging(&entry.path());
        }
    }
}

impl Installed for Bundle {
    fn asset_suffix(&self) -> &'static str {
        "-macos-arm64.zip"
    }

    /// Unzips `zip` beside the app, checks its signature, then swaps it in for the running
    /// app. The old app is only replaced once the new one is fully in place and verified.
    fn install(&self, zip: &[u8]) -> Result<()> {
        let bundle = &self.0;
        let parent = bundle.parent().context("app has no parent folder")?;
        let staging = create_staging(parent)?;
        let result = swap_in(bundle, &staging, zip);
        cleanup_staging(&staging);
        result
    }

    /// Reopens the app once this process has exited.
    fn relaunch(&self) -> Result<()> {
        Command::new("/bin/sh")
            .args([
                "-c",
                r#"while kill -0 "$0" 2>/dev/null; do sleep 0.2; done; open "$1""#,
            ])
            .arg(std::process::id().to_string())
            .arg(&self.0)
            .spawn()?;
        Ok(())
    }
}

fn create_staging(parent: &Path) -> Result<PathBuf> {
    let staging = parent.join(format!("{STAGING_PREFIX}{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&staging)
        .with_context(|| format!("can't write to {}", parent.display()))?;
    Ok(staging)
}

/// Removes `staging` unless it holds `old.app`, the app's only backup after a failed rollback.
fn cleanup_staging(staging: &Path) {
    if matches!(staging.join("old.app").try_exists(), Ok(false))
        && let Err(err) = std::fs::remove_dir_all(staging)
    {
        crate::log::write(&format!("update cleanup {}: {err}", staging.display()));
    }
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

    replace_bundle(bundle, staging, |from, to| std::fs::rename(from, to))
}

fn replace_bundle(
    bundle: &Path,
    staging: &Path,
    rename: impl Fn(&Path, &Path) -> std::io::Result<()>,
) -> Result<()> {
    let new = staging.join("tbis.app");
    let old = staging.join("old.app");
    rename(bundle, &old).context("can't move the current app aside")?;
    if let Err(err) = rename(&new, bundle) {
        if let Err(rollback_err) = rename(&old, bundle) {
            bail!(
                "can't move the new app into place: {err}; rollback failed: {rollback_err}; current app backup is at {}",
                old.display()
            );
        }
        bail!("can't move the new app into place: {err}");
    }
    if let Err(err) = std::fs::remove_dir_all(&old) {
        crate::log::write(&format!(
            "new app installed, but old app backup at {} could not be removed: {err}",
            old.display()
        ));
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

#[cfg(test)]
mod tests {
    use super::{clean_up_stale_staging, cleanup_staging, create_staging, replace_bundle};
    use std::cell::Cell;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};

    struct Bundles {
        root: PathBuf,
        bundle: PathBuf,
        staging: PathBuf,
    }

    impl Bundles {
        fn new() -> Self {
            let root =
                std::env::temp_dir().join(format!("tbis-update-test-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&root).unwrap();
            let bundle = root.join("tbis.app");
            fs::create_dir(&bundle).unwrap();
            fs::write(bundle.join("version"), "old").unwrap();
            let staging = create_staging(&root).unwrap();
            let new = staging.join("tbis.app");
            fs::create_dir(&new).unwrap();
            fs::write(new.join("version"), "new").unwrap();
            Self {
                root,
                bundle,
                staging,
            }
        }
    }

    impl Drop for Bundles {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }

    fn replace_with_failures(bundles: &Bundles, failures: &[usize]) -> anyhow::Result<()> {
        let calls = Cell::new(0);
        replace_bundle(
            &bundles.bundle,
            &bundles.staging,
            |from: &Path, to: &Path| {
                let call = calls.get() + 1;
                calls.set(call);
                if failures.contains(&call) {
                    Err(io::Error::other(format!("rename {call} failed")))
                } else {
                    fs::rename(from, to)
                }
            },
        )
    }

    #[test]
    fn replacement_succeeds_and_cleans_staging() {
        let bundles = Bundles::new();
        replace_with_failures(&bundles, &[]).unwrap();
        cleanup_staging(&bundles.staging);
        assert_eq!(
            fs::read_to_string(bundles.bundle.join("version")).unwrap(),
            "new"
        );
        assert!(!bundles.staging.exists());
    }

    #[test]
    fn failure_moving_current_app_leaves_it_intact() {
        let bundles = Bundles::new();
        let err = replace_with_failures(&bundles, &[1]).unwrap_err();
        assert!(err.to_string().contains("can't move the current app aside"));
        cleanup_staging(&bundles.staging);
        assert_eq!(
            fs::read_to_string(bundles.bundle.join("version")).unwrap(),
            "old"
        );
        assert!(!bundles.staging.exists());
    }

    #[test]
    fn failed_replacement_restores_current_app() {
        let bundles = Bundles::new();
        let err = replace_with_failures(&bundles, &[2]).unwrap_err();
        assert!(
            err.to_string()
                .contains("can't move the new app into place")
        );
        cleanup_staging(&bundles.staging);
        assert_eq!(
            fs::read_to_string(bundles.bundle.join("version")).unwrap(),
            "old"
        );
        assert!(!bundles.staging.exists());
    }

    #[test]
    fn failed_rollback_preserves_backup_through_cleanup_and_retry() {
        let bundles = Bundles::new();
        let backup = bundles.staging.join("old.app");
        let err = replace_with_failures(&bundles, &[2, 3]).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("rename 2 failed"));
        assert!(message.contains("rename 3 failed"));
        assert!(message.contains(&backup.display().to_string()));
        cleanup_staging(&bundles.staging);
        let retry = create_staging(&bundles.root).unwrap();
        cleanup_staging(&retry);
        assert_eq!(fs::read_to_string(backup.join("version")).unwrap(), "old");
        assert!(!bundles.bundle.exists());
    }

    #[test]
    fn startup_removes_stale_staging_but_keeps_a_backup() {
        let bundles = Bundles::new();
        let stale = bundles.staging.clone();
        let backup = create_staging(&bundles.root).unwrap();
        fs::create_dir(backup.join("old.app")).unwrap();
        let unrelated = bundles.root.join("other");
        fs::create_dir(&unrelated).unwrap();
        clean_up_stale_staging(&bundles.bundle);
        assert!(!stale.exists());
        assert!(backup.join("old.app").exists());
        assert!(unrelated.exists());
        assert!(bundles.bundle.exists());
    }
}
