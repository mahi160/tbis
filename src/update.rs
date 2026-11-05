use std::sync::Arc;

use anyhow::{Context as _, Result, anyhow, ensure};
use futures::AsyncReadExt as _;
use gpui_kit::http_client::HttpClient;
use gpui_kit::http_client::github::{GithubReleaseAsset, latest_github_release};
use sha2::{Digest as _, Sha256};

use crate::platform::update as native;

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
    /// SHA-256 hex of the zip, as recorded by GitHub.
    sha256: String,
}

pub async fn check(http: Arc<dyn HttpClient>) -> Result<Option<Update>> {
    let suffix = native::asset_suffix()?;
    let release = latest_github_release(REPO, true, false, http.clone()).await?;
    let version = release.tag_name.trim_start_matches('v').to_string();
    if !newer(&version, CURRENT) {
        return Ok(None);
    }
    let asset = select_asset(release.assets, suffix)?;
    let sha256 = asset.digest.context("release has no SHA-256 checksum")?;
    validate_checksum(&sha256)?;
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

fn validate_checksum(checksum: &str) -> Result<()> {
    ensure!(
        checksum.len() == 64 && checksum.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "release has an invalid SHA-256 checksum"
    );
    Ok(())
}

fn verify_checksum(bytes: &[u8], expected: &str) -> Result<()> {
    validate_checksum(expected)?;
    let actual = format!("{:x}", Sha256::digest(bytes));
    ensure!(
        actual.eq_ignore_ascii_case(expected),
        "download is corrupt (checksum mismatch)"
    );
    Ok(())
}

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

fn newer(a: &str, b: &str) -> bool {
    let parse = |v: &str| -> Option<(u64, u64, u64)> {
        let mut parts = v.split('.').map(|p| p.parse().ok());
        let version = (parts.next()??, parts.next()??, parts.next()??);
        parts.next().is_none().then_some(version)
    };
    matches!((parse(a), parse(b)), (Some(a), Some(b)) if a > b)
}

pub async fn install(http: Arc<dyn HttpClient>, update: &Update) -> Result<()> {
    let bundle =
        bundle_path().context("automatic updates are unavailable for this installation")?;
    validate_checksum(&update.sha256)?;
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
        assert!(verify_checksum(b"abc", digest).is_ok());
        assert!(verify_checksum(b"abc", &digest.to_uppercase()).is_ok());
        assert!(verify_checksum(b"changed", digest).is_err());
        for invalid in [
            String::new(),
            "f".repeat(63),
            "f".repeat(65),
            "g".repeat(64),
            format!("sha256:{digest}"),
            format!("{digest}\n"),
        ] {
            assert!(validate_checksum(&invalid).is_err());
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
