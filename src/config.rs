use std::fs::{self, OpenOptions, Permissions};
use std::io::{self, Write as _};
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::jellyfin::{Session, Sort};

#[derive(Serialize, Deserialize)]
pub struct Config {
    /// Survives Log out, so the server sees one device per install.
    pub device_id: String,
    pub session: Option<Session>,
    #[serde(default)]
    pub movies_sort: Sort,
    #[serde(default = "full_volume")]
    pub volume: f64,
    #[serde(default)]
    pub muted: bool,
}

fn full_volume() -> f64 {
    100.
}

fn path() -> PathBuf {
    let home = std::env::var_os("HOME").expect("HOME is not set");
    Path::new(&home).join("Library/Application Support/tbis/config.json")
}

pub fn load() -> Config {
    fs::read(path())
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Config>(&bytes).ok())
        .filter(|config| !config.device_id.is_empty())
        .unwrap_or_else(|| Config {
            device_id: uuid::Uuid::new_v4().to_string(),
            session: None,
            movies_sort: Sort::default(),
            volume: full_volume(),
            muted: false,
        })
}

/// Holds the access token in plaintext, so only the user may read it.
pub fn save(config: &Config) -> io::Result<()> {
    let path = path();
    fs::create_dir_all(path.parent().unwrap())?;
    let tmp = path.with_extension("json.tmp");
    let _ = fs::remove_file(&tmp);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp)?;
    file.write_all(&serde_json::to_vec_pretty(config)?)?;
    file.sync_all()?;
    fs::set_permissions(&tmp, Permissions::from_mode(0o600))?;
    fs::rename(&tmp, &path)
}
