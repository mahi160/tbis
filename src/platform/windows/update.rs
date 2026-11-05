use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct Entry {
    name: String,
    attributes: i32,
    length: u64,
}

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

fn validate_entries(entries: &[Entry]) -> Result<Vec<String>> {
    let mut paths = BTreeMap::new();
    let mut roots = BTreeSet::new();
    for entry in entries {
        let directory = entry.name.ends_with('/');
        let name = entry.name.strip_suffix('/').unwrap_or(&entry.name);
        let parts: Vec<_> = name.split('/').collect();
        ensure!(
            parts.iter().all(|part| safe_component(part)),
            "unsafe zip path: {}",
            entry.name
        );
        ensure!(
            managed_name(parts[0]),
            "unexpected bundle file: {}",
            entry.name
        );
        ensure!(
            if parts.len() > 1 || directory {
                directory_name(parts[0])
            } else {
                !directory_name(parts[0])
            },
            "unexpected bundle layout: {}",
            entry.name
        );
        let kind = (entry.attributes >> 16) & 0xf000;
        ensure!(
            entry.attributes & 0x400 == 0
                && (kind == 0 || kind == if directory { 0x4000 } else { 0x8000 })
                && (entry.attributes & 0x10 == 0 || directory),
            "zip contains a link or special file: {}",
            entry.name
        );
        ensure!(
            !directory || entry.length == 0,
            "zip directory has data: {}",
            entry.name
        );
        let key = name.to_ascii_lowercase();
        ensure!(
            paths.insert(key, directory).is_none(),
            "duplicate zip path: {}",
            entry.name
        );
        roots.insert(parts[0].to_string());
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
        ensure!(
            entries
                .iter()
                .any(|entry| entry.name.eq_ignore_ascii_case(required) && entry.length > 0),
            "bundle has empty {required}"
        );
    }
    let mut names = BTreeMap::new();
    for root in roots {
        let key = root.to_ascii_lowercase();
        ensure!(
            names.insert(key, root).is_none(),
            "inconsistent bundle path casing"
        );
    }
    Ok(names.into_values().collect())
}

fn validate_tree(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(
        !metadata.file_type().is_symlink(),
        "bundle contains a link: {}",
        path.display()
    );
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt as _;
        ensure!(
            metadata.file_attributes() & 0x400 == 0,
            "bundle contains a reparse point: {}",
            path.display()
        );
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            validate_tree(&entry?.path())?;
        }
    } else {
        ensure!(
            metadata.is_file(),
            "bundle contains a special file: {}",
            path.display()
        );
    }
    Ok(())
}

fn inventory(bundle: &Path) -> Result<Vec<String>> {
    let mut names = Vec::new();
    let metadata = fs::symlink_metadata(bundle)?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "installation is not a regular directory"
    );
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::MetadataExt as _;
        ensure!(
            metadata.file_attributes() & 0x400 == 0,
            "installation is a reparse point"
        );
    }
    for entry in fs::read_dir(bundle)? {
        let entry = entry?;
        let name = entry.file_name();
        if let Some(name) = name.to_str().filter(|name| managed_name(name)) {
            ensure!(safe_component(name), "unsafe installed bundle name: {name}");
            validate_tree(&entry.path())?;
            ensure!(
                entry.file_type()?.is_dir() == directory_name(name),
                "unexpected installed bundle layout: {name}"
            );
            names.push(name.to_string());
        }
    }
    names.sort();
    Ok(names)
}

#[cfg(target_os = "windows")]
mod native {
    use super::*;
    use std::os::windows::process::CommandExt as _;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use anyhow::Context as _;

    const CREATE_NO_WINDOW: u32 = 0x08000000;
    static PENDING: Mutex<Option<PathBuf>> = Mutex::new(None);

    #[derive(Serialize)]
    struct Plan {
        bundle: PathBuf,
        old: Vec<String>,
        new: Vec<String>,
        pid: u32,
    }

    pub fn asset_suffix() -> Result<&'static str> {
        Ok("-windows-x86_64.zip")
    }

    pub fn bundle_path() -> Option<PathBuf> {
        let exe = std::env::current_exe().ok()?;
        if !exe.file_name()?.to_str()?.eq_ignore_ascii_case("tbis.exe") {
            return None;
        }
        let bundle = exe.parent()?;
        let names = inventory(bundle).ok()?;
        names
            .iter()
            .any(|name| name.eq_ignore_ascii_case("libmpv-2.dll"))
            .then(|| bundle.to_path_buf())
    }

    fn command(stage: &Path, mode: &str) -> Result<Command> {
        let system = std::env::var_os("SystemRoot").context("SystemRoot is missing")?;
        let mut command = Command::new(
            PathBuf::from(system).join("System32/WindowsPowerShell/v1.0/powershell.exe"),
        );
        command.args([
            "-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
            "& ([scriptblock]::Create([IO.File]::ReadAllText([IO.Path]::Combine($env:TBIS_UPDATE_STAGE, 'update.ps1'))))",
        ])
            .env("TBIS_UPDATE_STAGE", stage)
            .env("TBIS_UPDATE_MODE", mode)
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(Stdio::null());
        Ok(command)
    }

    fn run(stage: &Path, mode: &str) -> Result<Vec<u8>> {
        let stdout = stage.join(format!("{mode}.stdout"));
        let stderr = stage.join(format!("{mode}.stderr"));
        let mut child = command(stage, mode)?
            .stdout(Stdio::from(fs::File::create(&stdout)?))
            .stderr(Stdio::from(fs::File::create(&stderr)?))
            .spawn()?;
        let deadline = Instant::now() + Duration::from_secs(120);
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if Instant::now() >= deadline {
                child
                    .kill()
                    .context("can't stop the timed-out staging helper")?;
                let stop_deadline = Instant::now() + Duration::from_secs(10);
                while child.try_wait()?.is_none() {
                    ensure!(
                        Instant::now() < stop_deadline,
                        "timed-out staging helper did not exit"
                    );
                    std::thread::sleep(Duration::from_millis(100));
                }
                anyhow::bail!("Windows update {mode} exceeded 120 seconds");
            }
            std::thread::sleep(Duration::from_millis(100));
        };
        ensure!(
            status.success(),
            "Windows update {mode} failed: {}",
            String::from_utf8_lossy(&fs::read(stderr)?).trim()
        );
        Ok(fs::read(stdout)?)
    }

    pub fn install(bundle: &Path, zip: &[u8]) -> Result<()> {
        let mut pending = PENDING
            .lock()
            .map_err(|_| anyhow::anyhow!("update state is poisoned"))?;
        ensure!(pending.is_none(), "an update is already staged");
        let stage = bundle.join(format!(".tbis-update-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&stage).context("can't stage an update beside the executable")?;
        let result = (|| -> Result<()> {
            fs::write(stage.join("update.ps1"), include_str!("update.ps1"))?;
            fs::write(stage.join("tbis.zip"), zip)?;
            let entries: Vec<Entry> = serde_json::from_slice(&run(&stage, "Inspect")?)?;
            let new = validate_entries(&entries)?;
            let old = inventory(bundle)?;
            fs::write(stage.join("entries.json"), serde_json::to_vec(&entries)?)?;
            run(&stage, "Extract")?;
            let plan = Plan {
                bundle: bundle.to_path_buf(),
                old,
                new,
                pid: std::process::id(),
            };
            fs::write(stage.join("plan.json"), serde_json::to_vec(&plan)?)?;
            fs::create_dir(stage.join("old"))?;
            Ok(())
        })();
        if let Err(err) = result {
            if let Err(cleanup) = fs::remove_dir_all(&stage) {
                return Err(err.context(format!(
                    "staging cleanup failed at {}: {cleanup}",
                    stage.display()
                )));
            }
            return Err(err);
        }
        *pending = Some(stage);
        Ok(())
    }

    pub fn relaunch() -> Result<()> {
        let mut pending = PENDING
            .lock()
            .map_err(|_| anyhow::anyhow!("update state is poisoned"))?;
        let stage = pending.take().context("no Windows update is staged")?;
        drop(pending);
        let log = fs::OpenOptions::new().create(true).append(true).open(
            stage
                .parent()
                .context("staging has no parent")?
                .join("tbis-update.log"),
        )?;
        let mut child = command(&stage, "Apply")?
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log))
            .spawn()
            .context("can't start the Windows update helper")?;
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if stage.join("ready").is_file() {
                return Ok(());
            }
            if let Some(status) = child.try_wait()? {
                anyhow::bail!("Windows update helper exited ({status}); see tbis-update.log");
            }
            if Instant::now() >= deadline {
                fs::write(stage.join("cancel"), b"cancel")?;
                anyhow::bail!("Windows update helper did not become ready; see tbis-update.log");
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

#[cfg(target_os = "windows")]
pub use native::{asset_suffix, bundle_path, install, relaunch};

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str) -> Entry {
        Entry {
            name: name.into(),
            attributes: 0,
            length: 1,
        }
    }

    fn bundle() -> Vec<Entry> {
        vec![
            file("tbis.exe"),
            file("libmpv-2.dll"),
            file("other.dll"),
            file("LICENSE"),
            file("bundled-components.txt"),
            file("licenses/mpv/COPYING"),
            file("components/data.bin"),
        ]
    }

    #[test]
    fn accepts_portable_bundle_and_all_dependencies() {
        let names = validate_entries(&bundle()).unwrap();
        assert_eq!(
            names,
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
    fn rejects_unsafe_or_unrelated_zip_entries_before_extraction() {
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
        ] {
            let mut entries = bundle();
            entries.push(file(name));
            assert!(validate_entries(&entries).is_err(), "{name}");
        }
        for attributes in [-0x60000000, 0x400, 0x10000000, 0x10] {
            let mut entries = bundle();
            entries.push(Entry {
                name: "licenses/link".into(),
                attributes,
                length: 1,
            });
            assert!(validate_entries(&entries).is_err());
        }
        for extra in ["TBIS.EXE", "licenses/mpv/COPYING/child", "Licenses/other"] {
            let mut entries = bundle();
            entries.push(file(extra));
            assert!(validate_entries(&entries).is_err(), "{extra}");
        }
        assert!(validate_entries(&[file("tbis.exe")]).is_err());
    }

    #[test]
    fn inventories_only_shipped_files_without_mutating_the_installation() {
        let root = std::env::temp_dir().join(format!("tbis-update-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("licenses")).unwrap();
        for name in [
            "tbis.exe",
            "libmpv-2.dll",
            "obsolete.dll",
            "LICENSE",
            "bundled-components.txt",
            "licenses/COPYING",
            "config.json",
            "tbis.log",
            "personal.txt",
        ] {
            fs::write(root.join(name), name).unwrap();
        }
        let names = inventory(&root).unwrap();
        assert_eq!(
            names,
            [
                "LICENSE",
                "bundled-components.txt",
                "libmpv-2.dll",
                "licenses",
                "obsolete.dll",
                "tbis.exe",
            ]
        );
        assert_eq!(fs::read(root.join("tbis.exe")).unwrap(), b"tbis.exe");
        assert_eq!(fs::read(root.join("config.json")).unwrap(), b"config.json");
        assert_eq!(fs::read(root.join("tbis.log")).unwrap(), b"tbis.log");
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_installed_links_before_preparing_a_swap() {
        let root = std::env::temp_dir().join(format!("tbis-update-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        std::os::unix::fs::symlink("missing", root.join("old.dll")).unwrap();
        assert!(inventory(&root).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
