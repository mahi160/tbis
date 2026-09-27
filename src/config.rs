use std::collections::HashMap;
use std::fs::{self, OpenOptions, Permissions};
use std::io::{self, Write as _};
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::jellyfin::{Session, Sort};

/// Remembered audio/subtitle pick for one exact Movie/Episode, by mpv track id --
/// safe to reuse verbatim since replaying the same file gives the same track layout
/// every time. `subtitle: None` alone means "no memory"; `subtitle_off` disambiguates
/// that from "explicitly turned off", which `select_track`'s own `None` can't carry.
#[derive(Clone, Copy, Default, Serialize, Deserialize)]
pub struct TrackChoice {
    pub audio: Option<i64>,
    pub subtitle: Option<i64>,
    #[serde(default)]
    pub subtitle_off: bool,
}

/// A remembered audio/subtitle language, scoped to one Series (or, as `TrackPrefs`'s
/// own fields, to the whole app). `subtitles_enabled: None` means "never picked at this
/// scope" -- distinct from `Some(false)`, an explicit off, so it falls through to a
/// wider scope instead of wrongly suppressing that scope's own subtitles.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct LanguagePref {
    #[serde(default)]
    pub audio_lang: Option<String>,
    #[serde(default)]
    pub subtitle_lang: Option<String>,
    #[serde(default)]
    pub subtitles_enabled: Option<bool>,
}

/// Audio/subtitle memory across the whole app: exact per-item picks, a per-series
/// language (an Episode never played before still comes up in the language the rest
/// of its Series was watched in, without dragging that pick into an unrelated Series
/// or Movie), and an app-wide language of last resort for content with no series
/// (Movies) or no pick yet.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct TrackPrefs {
    #[serde(default)]
    pub memory: HashMap<String, TrackChoice>,
    #[serde(default)]
    pub series: HashMap<String, LanguagePref>,
    #[serde(default)]
    pub global: LanguagePref,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    /// Survives Log out, so the server sees one device per install.
    pub device_id: String,
    pub session: Option<Session>,
    #[serde(default)]
    pub movies_sort: Sort,
    #[serde(default)]
    pub series_sort: Sort,
    #[serde(default = "full_volume")]
    pub volume: f64,
    #[serde(default)]
    pub track_prefs: TrackPrefs,
    #[serde(default)]
    pub muted: bool,
    /// Settings' preferred languages; below per-item/per-Series memory, above the
    /// app-wide learned language.
    #[serde(default)]
    pub language: LanguagePref,
}

fn full_volume() -> f64 {
    100.
}

fn path() -> PathBuf {
    crate::support_dir::app_support_dir().join("config.json")
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
            series_sort: Sort::default(),
            volume: full_volume(),
            track_prefs: TrackPrefs::default(),
            muted: false,
            language: LanguagePref::default(),
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
