//! Installing a downloaded Windows bundle zip over the install folder, in process.
//! Windows lets a running exe or loaded DLL be renamed but not overwritten, so each
//! replaced file is renamed aside to `<name>.old` and the new one moved in; the next
//! launch deletes the `.old` files. The rules live here, not in `platform/windows`,
//! so their tests run on every OS.

use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Cursor, Read, Seek};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, anyhow, ensure};
use zip::ZipArchive;

const STAGE_PREFIX: &str = ".tbis-update-";
const ASIDE_SUFFIX: &str = ".old";

fn managed_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    matches!(
        name.as_str(),
        "tbis.exe" | "license" | "bundled-components.txt" | "licenses" | "components"
    ) || name.ends_with(".dll")
}

fn directory_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "licenses" | "components"
    )
}

fn safe_component(name: &str) -> bool {
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    !name.is_empty()
        && name.is_ascii()
        && !name.ends_with(['.', ' '])
        && !name
            .chars()
            .any(|ch| ch.is_control() || "<>:\"\\|?*".contains(ch))
        && !matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        )
        && !(stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.chars().last(), Some('1'..='9')))
}

/// Checks every entry before anything is written; returns the top-level names to install.
fn validate(archive: &mut ZipArchive<impl Read + Seek>) -> Result<Vec<String>> {
    let mut paths = BTreeMap::new();
    let mut roots = BTreeMap::new();
    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        let entry = file.name()?.into_owned();
        let directory = file.is_dir();
        ensure!(!file.is_symlink(), "zip contains a link: {entry}");
        let name = entry.strip_suffix('/').unwrap_or(&entry);
        let parts: Vec<_> = name.split('/').collect();
        ensure!(
            parts.iter().all(|part| safe_component(part)),
            "unsafe zip path: {entry}"
        );
        ensure!(managed_name(parts[0]), "unexpected bundle file: {entry}");
        ensure!(
            if parts.len() > 1 || directory {
                directory_name(parts[0])
            } else {
                !directory_name(parts[0])
            },
            "unexpected bundle layout: {entry}"
        );
        ensure!(
            paths.insert(name.to_ascii_lowercase(), directory).is_none(),
            "duplicate zip path: {entry}"
        );
        let root = roots
            .entry(parts[0].to_ascii_lowercase())
            .or_insert_with(|| parts[0].to_string());
        ensure!(root == parts[0], "inconsistent bundle path casing: {entry}");
    }
    for (name, directory) in &paths {
        if !directory {
            let prefix = format!("{name}/");
            ensure!(
                !paths.keys().any(|path| path.starts_with(&prefix)),
                "zip file/directory collision: {name}"
            );
        }
    }
    for required in ["tbis.exe", "libmpv-2.dll"] {
        ensure!(
            paths.get(required) == Some(&false),
            "bundle is missing {required}"
        );
    }
    Ok(roots.into_values().collect())
}

fn extract(archive: &mut ZipArchive<impl Read + Seek>, stage: &Path) -> Result<()> {
    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let path = stage.join(file.name()?.as_ref());
        if file.is_dir() {
            fs::create_dir_all(&path)?;
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            io::copy(&mut file, &mut fs::File::create(&path)?)
                .with_context(|| format!("can't unpack {}", path.display()))?;
        }
    }
    Ok(())
}

fn aside_path(bundle: &Path, name: &str) -> PathBuf {
    bundle.join(format!("{name}{ASIDE_SUFFIX}"))
}

fn remove_any(path: &Path) -> io::Result<()> {
    if fs::symlink_metadata(path)?.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Moves one staged item into the install, its current version aside. Returns whether
/// there was a current version. On failure the current version is back in place.
fn swap_one(bundle: &Path, stage: &Path, name: &str) -> Result<bool> {
    let live = bundle.join(name);
    let aside = aside_path(bundle, name);
    let had_old = live.exists();
    if had_old {
        if aside.exists() {
            remove_any(&aside).with_context(|| format!("can't clear {}", aside.display()))?;
        }
        fs::rename(&live, &aside).with_context(|| format!("can't move {name} aside"))?;
    }
    if let Err(err) = fs::rename(stage.join(name), &live) {
        if had_old {
            fs::rename(&aside, &live).with_context(|| {
                format!(
                    "can't restore {name} ({err}); old copy kept at {}",
                    aside.display()
                )
            })?;
        }
        return Err(anyhow!("can't move {name} into place: {err}"));
    }
    Ok(had_old)
}

fn swap(bundle: &Path, stage: &Path, names: &[String]) -> Result<()> {
    let mut swapped = Vec::new();
    for name in names {
        match swap_one(bundle, stage, name) {
            Ok(had_old) => swapped.push((name, had_old)),
            Err(err) => return Err(roll_back(bundle, stage, &swapped, err)),
        }
    }
    Ok(())
}

/// Puts the already swapped items back, newest first.
fn roll_back(
    bundle: &Path,
    stage: &Path,
    swapped: &[(&String, bool)],
    cause: anyhow::Error,
) -> anyhow::Error {
    let mut failures = Vec::new();
    for (name, had_old) in swapped.iter().rev() {
        let live = bundle.join(name);
        if let Err(err) = fs::rename(&live, stage.join(name)) {
            failures.push(format!("{name}: {err}"));
        } else if *had_old && let Err(err) = fs::rename(aside_path(bundle, name), &live) {
            failures.push(format!("{name}: {err}"));
        }
    }
    if failures.is_empty() {
        cause.context("update failed; restored the current install")
    } else {
        cause.context(format!(
            "update failed and rollback was incomplete ({})",
            failures.join("; ")
        ))
    }
}

/// Validates the zip, unpacks it into a staging folder beside the install, then swaps
/// it in. Nothing is written until every entry has passed validation.
pub fn install(bundle: &Path, zip: &[u8]) -> Result<()> {
    let mut archive =
        ZipArchive::new(Cursor::new(zip)).context("downloaded update is not a valid zip")?;
    let names = validate(&mut archive)?;
    let stage = bundle.join(format!("{STAGE_PREFIX}{}", uuid::Uuid::new_v4()));
    fs::create_dir(&stage).context("can't stage an update beside the executable")?;
    let result = extract(&mut archive, &stage).and_then(|()| swap(bundle, &stage, &names));
    if let Err(err) = fs::remove_dir_all(&stage) {
        crate::log::write(&format!("update cleanup {}: {err}", stage.display()));
    }
    result
}

/// Deletes what a previous update left behind: renamed-aside files and staging folders.
/// Returns whether everything was removed.
pub fn clean_up(bundle: &Path) -> bool {
    let Ok(entries) = fs::read_dir(bundle) else {
        return true;
    };
    let mut clean = true;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        let leftover = name.starts_with(STAGE_PREFIX)
            || name.strip_suffix(ASIDE_SUFFIX).is_some_and(managed_name);
        if leftover && remove_any(&entry.path()).is_err() {
            clean = false;
        }
    }
    clean
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use zip::CompressionMethod;
    use zip::write::{SimpleFileOptions, ZipWriter};

    use super::*;

    const BASE: [&str; 7] = [
        "tbis.exe",
        "libmpv-2.dll",
        "other.dll",
        "LICENSE",
        "bundled-components.txt",
        "licenses/mpv/COPYING",
        "components/data.bin",
    ];

    fn options() -> SimpleFileOptions {
        SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
    }

    fn zip_of(files: &[&str], symlinks: &[&str]) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for name in files {
            writer.start_file(*name, options()).unwrap();
            writer.write_all(b"new").unwrap();
        }
        for name in symlinks {
            writer.add_symlink(*name, "target", options()).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn bundle_with(extra: &str) -> Vec<u8> {
        let mut files = BASE.to_vec();
        files.push(extra);
        zip_of(&files, &[])
    }

    fn validated(zip: &[u8]) -> Result<Vec<String>> {
        validate(&mut ZipArchive::new(Cursor::new(zip)).unwrap())
    }

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tbis-update-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&dir).unwrap();
        dir
    }

    #[test]
    fn accepts_portable_bundle_and_lists_top_level_names() {
        assert_eq!(
            validated(&zip_of(&BASE, &[])).unwrap(),
            [
                "bundled-components.txt",
                "components",
                "libmpv-2.dll",
                "LICENSE",
                "licenses",
                "other.dll",
                "tbis.exe",
            ]
        );
    }

    #[test]
    fn rejects_unsafe_or_unexpected_entries() {
        for name in [
            "../tbis.exe",
            "/tbis.exe",
            "C:/tbis.exe",
            "licenses/../config.json",
            "licenses\\evil",
            "licenses/a:stream",
            "licenses/NUL.txt",
            "licenses/COM1",
            "licenses/LPT¹",
            "licenses/a.",
            "licenses//a",
            "licenses/a ",
            "config.json",
            "tbis.log",
            "other.exe",
            "tbis.exe/payload",
            "licenses",
            "TBIS.EXE",
            "licenses/mpv/COPYING/child",
            "Licenses/other",
        ] {
            assert!(validated(&bundle_with(name)).is_err(), "{name}");
        }
        assert!(validated(&zip_of(&["tbis.exe"], &[])).is_err());
    }

    #[test]
    fn rejects_links() {
        assert!(validated(&zip_of(&BASE, &["licenses/link"])).is_err());
    }

    #[test]
    fn rejects_before_writing_anything() {
        let bundle = temp_dir();
        assert!(install(&bundle, &bundle_with("config.json")).is_err());
        assert_eq!(fs::read_dir(&bundle).unwrap().count(), 0);
        fs::remove_dir_all(bundle).unwrap();
    }

    #[test]
    fn installs_new_files_and_cleans_up_old_ones_later() {
        let bundle = temp_dir();
        fs::write(bundle.join("tbis.exe"), "old").unwrap();
        fs::write(bundle.join("config.json"), "mine").unwrap();
        install(&bundle, &zip_of(&BASE, &[])).unwrap();
        assert_eq!(fs::read(bundle.join("tbis.exe")).unwrap(), b"new");
        assert_eq!(fs::read(bundle.join("tbis.exe.old")).unwrap(), b"old");
        assert_eq!(
            fs::read(bundle.join("licenses/mpv/COPYING")).unwrap(),
            b"new"
        );
        assert_eq!(fs::read(bundle.join("config.json")).unwrap(), b"mine");
        clean_up(&bundle);
        assert!(!bundle.join("tbis.exe.old").exists());
        assert_eq!(fs::read(bundle.join("tbis.exe")).unwrap(), b"new");
        fs::remove_dir_all(bundle).unwrap();
    }

    #[test]
    fn failed_swap_leaves_old_files_in_place() {
        let bundle = temp_dir();
        let stage = bundle.join(".tbis-update-test");
        fs::create_dir(&stage).unwrap();
        fs::write(bundle.join("tbis.exe"), "old exe").unwrap();
        fs::write(bundle.join("libmpv-2.dll"), "old dll").unwrap();
        fs::write(stage.join("tbis.exe"), "new exe").unwrap();
        // libmpv-2.dll is not staged, so its move fails after tbis.exe was already swapped
        let names = ["tbis.exe".to_string(), "libmpv-2.dll".to_string()];
        assert!(swap(&bundle, &stage, &names).is_err());
        assert_eq!(fs::read(bundle.join("tbis.exe")).unwrap(), b"old exe");
        assert_eq!(fs::read(bundle.join("libmpv-2.dll")).unwrap(), b"old dll");
        assert!(!bundle.join("tbis.exe.old").exists());
        assert!(!bundle.join("libmpv-2.dll.old").exists());
        fs::remove_dir_all(bundle).unwrap();
    }
}
