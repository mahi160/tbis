//! Self-update from GitHub Releases (built by .github/workflows/release.yml):
//! find a newer release, download its app zip, check it against the SHA-256
//! GitHub records for the asset, swap it in for the running `tbis.app`, and
//! relaunch. Only works when running from an app bundle.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use anyhow::{Context as _, Result, anyhow, bail, ensure};
use futures::AsyncReadExt as _;
use gpui_kit::http_client::HttpClient;
use gpui_kit::http_client::github::latest_github_release;

const REPO: &str = "mahi160/tbis";
const ASSET_SUFFIX: &str = "-macos-arm64.zip";
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
pub struct Update {
    pub version: String,
    /// Release notes as markdown; empty when the release has none or they failed to load.
    pub notes: String,
    /// The release's GitHub page.
    pub page: String,
    url: String,
    /// SHA-256 hex of the zip, as recorded by GitHub.
    sha256: String,
}

/// `…/tbis.app` when running from one; `None` for `cargo run`.
pub fn bundle_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let bundle = exe.parent()?.parent()?.parent()?; // tbis.app/Contents/MacOS/tbis
    (bundle.extension()? == "app").then(|| bundle.to_path_buf())
}

/// Newest non-prerelease, if it is newer than this build.
pub async fn check(http: Arc<dyn HttpClient>) -> Result<Option<Update>> {
    let release = latest_github_release(REPO, true, false, http.clone()).await?;
    let version = release.tag_name.trim_start_matches('v').to_string();
    if !newer(&version, CURRENT) {
        return Ok(None);
    }
    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name.ends_with(ASSET_SUFFIX))
        .ok_or_else(|| anyhow!("release {version} has no macOS download"))?;
    // never install bytes we can't verify
    let sha256 = asset
        .digest
        .ok_or_else(|| anyhow!("release {version} has no checksum"))?;
    let notes = match notes(&http, &release.tag_name).await {
        Ok(notes) => notes,
        Err(err) => {
            eprintln!("release notes: {err}");
            String::new()
        }
    };
    Ok(Some(Update {
        version,
        notes,
        page: format!("https://github.com/{REPO}/releases/tag/{}", release.tag_name),
        url: asset.browser_download_url,
        sha256,
    }))
}

/// Markdown notes of the release tagged `tag` (gpui's release type has no body).
async fn notes(http: &Arc<dyn HttpClient>, tag: &str) -> Result<String> {
    #[derive(serde::Deserialize)]
    struct Release {
        body: Option<String>,
    }
    let url = format!("https://api.github.com/repos/{REPO}/releases/tags/{tag}");
    let mut response = http.get(&url, Default::default(), true).await?;
    ensure!(
        response.status().is_success(),
        "HTTP {}",
        response.status().as_u16()
    );
    let mut body = Vec::new();
    response.body_mut().read_to_end(&mut body).await?;
    let release: Release = serde_json::from_slice(&body)?;
    Ok(release.body.unwrap_or_default())
}

/// Whether `a` is a higher `x.y.z` than `b`; unparsable versions never are.
fn newer(a: &str, b: &str) -> bool {
    let parse = |v: &str| -> Option<(u64, u64, u64)> {
        let mut parts = v.split('.').map(|p| p.parse().ok());
        let version = (parts.next()??, parts.next()??, parts.next()??);
        parts.next().is_none().then_some(version)
    };
    matches!((parse(a), parse(b)), (Some(a), Some(b)) if a > b)
}

/// Downloads, verifies, and swaps in `update` for the running bundle. The old app
/// is only replaced once the new one is fully in place and its signature checks out.
pub async fn install(http: Arc<dyn HttpClient>, update: &Update) -> Result<()> {
    let bundle = bundle_path().ok_or_else(|| anyhow!("not running from tbis.app"))?;
    let mut response = http
        .get(&update.url, Default::default(), true)
        .await
        .context("download failed")?;
    ensure!(
        response.status().is_success(),
        "download failed: HTTP {}",
        response.status().as_u16()
    );
    let mut zip = Vec::new();
    response.body_mut().read_to_end(&mut zip).await?;

    // staged beside the bundle: rename only works within one volume
    let parent = bundle.parent().context("app has no parent folder")?;
    let staging = create_staging(parent)?;
    let result = swap_in(&bundle, &staging, &zip, &update.sha256);
    cleanup_staging(&staging);
    result
}

fn create_staging(parent: &Path) -> Result<PathBuf> {
    let staging = parent.join(format!(".tbis-update-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&staging)
        .with_context(|| format!("can't write to {}", parent.display()))?;
    Ok(staging)
}

fn cleanup_staging(staging: &Path) {
    if matches!(staging.join("old.app").try_exists(), Ok(false)) {
        let _ = std::fs::remove_dir_all(staging);
    }
}

fn swap_in(bundle: &Path, staging: &Path, zip: &[u8], sha256: &str) -> Result<()> {
    let zip_path = staging.join("tbis.zip");
    std::fs::write(&zip_path, zip)?;
    let actual = run(
        "shasum",
        &[OsStr::new("-a"), OsStr::new("256"), zip_path.as_os_str()],
    )?;
    let actual = actual.split_whitespace().next().unwrap_or_default();
    ensure!(
        actual.eq_ignore_ascii_case(sha256),
        "download is corrupt (checksum mismatch)"
    );
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
        eprintln!(
            "new app installed, but old app backup at {} could not be removed: {err}",
            old.display()
        );
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

/// Starts the (new) bundle once this process has exited; call right before quitting.
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

#[cfg(test)]
mod tests {
    use super::{cleanup_staging, create_staging, newer, replace_bundle};
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
        replace_bundle(&bundles.bundle, &bundles.staging, |from: &Path, to: &Path| {
            let call = calls.get() + 1;
            calls.set(call);
            if failures.contains(&call) {
                Err(io::Error::other(format!("rename {call} failed")))
            } else {
                fs::rename(from, to)
            }
        })
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
        assert!(err.to_string().contains("can't move the new app into place"));
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
    fn compares_versions_numerically() {
        assert!(newer("0.0.2", "0.0.1"));
        assert!(newer("0.10.0", "0.9.9"));
        assert!(newer("1.0.0", "0.99.99"));
        assert!(!newer("0.0.1", "0.0.1"));
        assert!(!newer("0.0.1", "0.0.2"));
        assert!(
            !newer("0.0.2-beta", "0.0.1"),
            "unparsable never counts as newer"
        );
    }
}
