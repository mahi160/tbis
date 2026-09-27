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
    let release = latest_github_release(REPO, true, false, http).await?;
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
    Ok(Some(Update {
        version,
        url: asset.browser_download_url,
        sha256,
    }))
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
    let staging = parent.join(format!(".tbis-update-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir(&staging)
        .with_context(|| format!("can't write to {}", parent.display()))?;
    let result = swap_in(&bundle, &staging, &zip, &update.sha256);
    let _ = std::fs::remove_dir_all(&staging);
    result
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

    let old = staging.join("old.app");
    std::fs::rename(bundle, &old).context("can't move the current app aside")?;
    if let Err(err) = std::fs::rename(&new, bundle) {
        // put the working app back
        let _ = std::fs::rename(&old, bundle);
        bail!("can't move the new app into place: {err}");
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
    use super::newer;

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
