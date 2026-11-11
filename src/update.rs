//! Self-update from GitHub Releases (built by .github/workflows/release.yml):
//! find a newer release, download the zip for this OS, check it against the SHA-256
//! GitHub records for the asset, swap it in for the running app, and relaunch.
//! macOS swaps the `tbis.app` bundle; Windows renames the running files aside in-process
//! (`bundle`). Linux has no self-update: `bundle_path` is `None`. Only works when running
//! from an installed bundle.

use std::sync::Arc;

use anyhow::{Context as _, Result, anyhow, ensure};
use futures::AsyncReadExt as _;
use gpui_kit::http_client::HttpClient;
use gpui_kit::http_client::github::{GithubReleaseAsset, latest_github_release};
use sha2::{Digest as _, Sha256};

use crate::platform::update as native;

// Only the Windows updater installs through it; its tests run on every OS.
#[cfg(any(target_os = "windows", test))]
pub mod bundle;

// `bundle_path`: the installed app folder, `None` when not running from one (or on Linux).
// `relaunch`: starts the updated app after a successful `install`.
pub use native::{bundle_path, relaunch};

const REPO: &str = "mahi160/tbis";
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
pub struct Update {
    pub version: String,
    /// Release notes as markdown; empty when the release has none or they failed to load.
    pub notes: String,
    /// The release's GitHub page.
    pub page: String,
    url: String,
    /// SHA-256 of the zip, as recorded by GitHub.
    sha256: [u8; 32],
}

/// Newest non-prerelease, if it is newer than this build.
pub async fn check(http: Arc<dyn HttpClient>) -> Result<Option<Update>> {
    let release = latest_github_release(REPO, true, false, http.clone()).await?;
    let version = release.tag_name.trim_start_matches('v').to_string();
    if !newer(&version, CURRENT) {
        return Ok(None);
    }
    let asset = select_asset(release.assets, native::ASSET_SUFFIX)?;
    let sha256 = parse_checksum(&asset.digest.context("release has no SHA-256 checksum")?)?;
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
        page: format!(
            "https://github.com/{REPO}/releases/tag/{}",
            release.tag_name
        ),
        url: asset.browser_download_url,
        sha256,
    }))
}

fn select_asset(assets: Vec<GithubReleaseAsset>, suffix: &str) -> Result<GithubReleaseAsset> {
    let mut matching = assets
        .into_iter()
        .filter(|asset| asset.name.ends_with(suffix));
    let asset = matching
        .next()
        .ok_or_else(|| anyhow!("release has no download ending in {suffix}"))?;
    ensure!(
        matching.next().is_none(),
        "release has ambiguous downloads for {suffix}"
    );
    Ok(asset)
}

fn parse_checksum(checksum: &str) -> Result<[u8; 32]> {
    ensure!(
        checksum.len() == 64 && checksum.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "release has an invalid SHA-256 checksum"
    );
    let mut digest = [0; 32];
    for (byte, pair) in digest.iter_mut().zip(checksum.as_bytes().chunks(2)) {
        // all ASCII hex digits, checked above
        *byte = u8::from_str_radix(std::str::from_utf8(pair)?, 16)?;
    }
    Ok(digest)
}

fn verify_checksum(bytes: &[u8], expected: &[u8; 32]) -> Result<()> {
    ensure!(
        Sha256::digest(bytes).as_slice() == expected,
        "download is corrupt (checksum mismatch)"
    );
    Ok(())
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
/// is only replaced once the new one is fully in place.
pub async fn install(http: Arc<dyn HttpClient>, update: &Update) -> Result<()> {
    let bundle =
        bundle_path().context("automatic updates are unavailable for this installation")?;
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
    verify_checksum(&zip, &update.sha256)?;
    native::install(&bundle, &zip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_numerically() {
        assert!(newer("0.0.2", "0.0.1"));
        assert!(newer("0.10.0", "0.9.9"));
        assert!(newer("1.0.0", "0.99.99"));
        assert!(!newer("0.0.1", "0.0.1"));
        assert!(!newer("0.0.1", "0.0.2"));
        assert!(!newer("0.0.2-beta", "0.0.1"));
    }

    fn asset(name: &str) -> GithubReleaseAsset {
        GithubReleaseAsset {
            name: name.into(),
            browser_download_url: format!("https://example.com/{name}"),
            digest: None,
        }
    }

    #[test]
    fn selects_only_the_platform_asset() {
        for suffix in ["-macos-arm64.zip", "-windows-x86_64.zip"] {
            let selected = select_asset(
                vec![
                    asset("tbis-1.0.0-macos-arm64.zip"),
                    asset("tbis-1.0.0-windows-x86_64.zip"),
                ],
                suffix,
            )
            .unwrap();
            assert!(selected.name.ends_with(suffix));
        }
        assert!(
            select_asset(vec![asset("tbis-windows-arm64.zip")], "-windows-x86_64.zip").is_err()
        );
        assert!(
            select_asset(
                vec![asset("a-windows-x86_64.zip"), asset("b-windows-x86_64.zip")],
                "-windows-x86_64.zip",
            )
            .is_err()
        );
    }

    #[test]
    fn requires_exact_sha256_and_checks_download_bytes() {
        let digest = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let parsed_digest = parse_checksum(digest).unwrap();
        assert!(verify_checksum(b"abc", &parsed_digest).is_ok());
        assert!(verify_checksum(b"abc", &parse_checksum(&digest.to_uppercase()).unwrap()).is_ok());
        assert!(verify_checksum(b"changed", &parsed_digest).is_err());
        for invalid in [
            String::new(),
            "f".repeat(63),
            "f".repeat(65),
            "g".repeat(64),
            format!("sha256:{digest}"),
            format!("{digest}\n"),
        ] {
            assert!(parse_checksum(&invalid).is_err());
        }
        let parsed: GithubReleaseAsset = serde_json::from_value(serde_json::json!({
            "name": "tbis-windows-x86_64.zip",
            "browser_download_url": "https://example.com/tbis.zip",
            "digest": format!("sha256:{digest}")
        }))
        .unwrap();
        assert_eq!(parsed.digest.as_deref(), Some(digest));
    }
}
