use std::collections::HashMap;
use std::fs::{self, OpenOptions, Permissions};
use std::io::{self, Write as _};
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};

use gpui_kit::{Bounds, Pixels, Size, WindowBounds, size};
use serde::{Deserialize, Serialize};

use crate::jellyfin::{Session, Sort};
use crate::shaders::ShaderProfile;

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

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ItemPlaybackPrefs {
    pub speed: Option<f64>,
    pub bitrate_set: bool,
    pub max_bitrate_mbps: Option<u32>,
    pub subtitle_delay: Option<f64>,
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
    pub playback: HashMap<String, ItemPlaybackPrefs>,
    #[serde(default)]
    pub series: HashMap<String, LanguagePref>,
    #[serde(default)]
    pub global: LanguagePref,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TailscaleOptions {
    pub enabled: bool,
    pub address: String,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    /// Survives Log out, so the server sees one device per install.
    pub device_id: String,
    #[serde(skip)]
    load_failed: bool,
    #[serde(skip)]
    load_error: Option<String>,
    pub session: Option<Session>,
    #[serde(default)]
    pub tailscale: TailscaleOptions,
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
    /// Text-subtitle look (Settings), for the Player and PiP.
    #[serde(default)]
    pub subtitles: SubtitleStyle,
    /// Streaming cap in Mbps (Settings); `None` always direct plays.
    #[serde(default)]
    pub max_bitrate_mbps: Option<u32>,
    /// Upscaling shaders the Player starts with (Settings).
    #[serde(default)]
    pub shaders: ShaderProfile,
    /// Player seek distances (Settings).
    #[serde(default)]
    pub seek: SeekSteps,
    /// Recent search queries, newest first.
    #[serde(default)]
    pub recent_searches: Vec<String>,
    /// Last window size/position, restored on launch.
    #[serde(default)]
    pub window: Option<WindowState>,
    /// Theme name (Settings); `None` is the default theme.
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub hide_spoilers: HideSpoilers,
    #[serde(default)]
    pub backdrops: ShowBackdrops,
    /// Audio passthrough and loudness normalization (Settings), for the Player and PiP.
    #[serde(default)]
    pub audio: AudioStyle,
}

/// Streaming caps offered in Settings and the Player; `None` plays the original file.
pub const BITRATE_CAPS: [(Option<u32>, &str); 6] = [
    (None, "Original"),
    (Some(40), "40 Mbps"),
    (Some(20), "20 Mbps"),
    (Some(10), "10 Mbps"),
    (Some(4), "4 Mbps"),
    (Some(2), "2 Mbps"),
];

/// Blur the stills and hide the descriptions of unwatched Episodes (Settings).
/// Also an app global, so Episode lists read the live value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HideSpoilers(pub bool);

impl gpui_kit::Global for HideSpoilers {}

/// Dimmed library backdrops behind Home, Movies and Series (Settings); on by default.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ShowBackdrops(pub bool);

impl Default for ShowBackdrops {
    fn default() -> Self {
        Self(true)
    }
}

/// Text-subtitle styling; image subtitles (PGS/VobSub) keep their own look.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct SubtitleStyle {
    /// Percent of mpv's default size.
    pub scale: u16,
    /// `#RRGGBB`.
    pub color: String,
    /// Vertical position, 100 = mpv's default bottom placement, lower is higher up.
    pub position: u8,
    /// Dark box behind the text instead of an outline.
    pub background: bool,
}

impl Default for SubtitleStyle {
    fn default() -> Self {
        Self {
            scale: 100,
            color: "#FFFFFF".into(),
            position: 100,
            background: false,
        }
    }
}

/// Bitstream passthrough to an AV receiver, and loudness normalization (Settings).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AudioStyle {
    /// AC3/DTS/E-AC3 sent to the receiver undecoded, in CoreAudio exclusive mode.
    pub passthrough: bool,
    /// "Night mode": mpv's `dynaudnorm` af filter evens out quiet and loud passages.
    pub loudness_norm: bool,
}

impl AudioStyle {
    /// mpv options applying this style, shared by the Player and PiP.
    pub fn mpv_options(&self) -> [(&'static str, String); 3] {
        [
            (
                "audio-spdif",
                if self.passthrough {
                    "ac3,dts,eac3".to_string()
                } else {
                    String::new()
                },
            ),
            (
                "audio-exclusive",
                if self.passthrough { "yes" } else { "no" }.to_string(),
            ),
            (
                "af",
                if self.loudness_norm {
                    "dynaudnorm".to_string()
                } else {
                    String::new()
                },
            ),
        ]
    }
}

/// Seconds per seek: `short` for arrows (and media keys), `long` for Option+arrows.
/// Also an app global, so the Player and shortcuts help read the live values.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SeekSteps {
    pub short: u32,
    pub long: u32,
}

impl Default for SeekSteps {
    fn default() -> Self {
        Self {
            short: 10,
            long: 60,
        }
    }
}

impl gpui_kit::Global for SeekSteps {}

impl SubtitleStyle {
    /// mpv options applying this style, shared by the Player and PiP.
    pub fn mpv_options(&self) -> [(&'static str, String); 4] {
        [
            ("sub-scale", format!("{}", f64::from(self.scale) / 100.)),
            ("sub-color", self.color.clone()),
            ("sub-pos", self.position.to_string()),
            (
                "sub-border-style",
                if self.background {
                    "background-box"
                } else {
                    "outline-and-shadow"
                }
                .into(),
            ),
        ]
    }
}

/// Restore bounds of the main window; fullscreen is saved as its windowed bounds.
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct WindowState {
    pub bounds: Bounds<Pixels>,
    pub maximized: bool,
}

impl From<WindowBounds> for WindowState {
    fn from(bounds: WindowBounds) -> Self {
        match bounds {
            WindowBounds::Windowed(bounds) | WindowBounds::Fullscreen(bounds) => Self {
                bounds,
                maximized: false,
            },
            WindowBounds::Maximized(bounds) => Self {
                bounds,
                maximized: true,
            },
        }
    }
}

impl WindowState {
    /// `None` when no display shows any part of it (e.g. its monitor was unplugged).
    pub fn restore(self, displays: &[Bounds<Pixels>], min: Size<Pixels>) -> Option<WindowBounds> {
        if !displays.iter().any(|d| d.intersects(&self.bounds)) {
            return None;
        }
        let bounds = Bounds::new(
            self.bounds.origin,
            size(
                self.bounds.size.width.max(min.width),
                self.bounds.size.height.max(min.height),
            ),
        );
        Some(if self.maximized {
            WindowBounds::Maximized(bounds)
        } else {
            WindowBounds::Windowed(bounds)
        })
    }
}

fn full_volume() -> f64 {
    100.
}

fn path() -> PathBuf {
    crate::support_dir::app_support_dir().join("config.json")
}

pub fn load() -> Config {
    load_from(&path())
}

fn load_from(path: &Path) -> Config {
    match fs::read(path) {
        Ok(bytes) => match serde_json::from_slice::<Config>(&bytes) {
            Ok(config) if !config.device_id.is_empty() => config,
            Ok(_) => failed_config("configuration has an empty device identity"),
            Err(_) => failed_config("configuration is invalid JSON"),
        },
        Err(err) if err.kind() == io::ErrorKind::NotFound => defaults(),
        Err(err) => failed_config(&format!("could not read configuration: {err}")),
    }
}

fn defaults() -> Config {
    Config {
        device_id: uuid::Uuid::new_v4().to_string(),
        load_failed: false,
        load_error: None,
        session: None,
        tailscale: TailscaleOptions::default(),
        movies_sort: Sort::default(),
        series_sort: Sort::default(),
        volume: full_volume(),
        track_prefs: TrackPrefs::default(),
        muted: false,
        language: LanguagePref::default(),
        window: None,
        subtitles: SubtitleStyle::default(),
        max_bitrate_mbps: None,
        shaders: ShaderProfile::Off,
        seek: SeekSteps::default(),
        recent_searches: Vec::new(),
        theme: None,
        hide_spoilers: HideSpoilers::default(),
        backdrops: ShowBackdrops::default(),
        audio: AudioStyle::default(),
    }
}

fn failed_config(error: &str) -> Config {
    let mut config = defaults();
    config.load_failed = true;
    config.load_error = Some(error.to_string());
    config
}

pub fn load_error(config: &Config) -> Option<&str> {
    config.load_error.as_deref()
}

/// Holds the access token in plaintext, so only the user may read it.
pub fn save(config: &Config) -> io::Result<()> {
    save_to(config, &path())
}

fn save_to(config: &Config, path: &Path) -> io::Result<()> {
    if config.load_failed {
        return Err(io::Error::other(
            "configuration was not loaded; refusing to overwrite it",
        ));
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
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
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::{WindowState, defaults, load_from, save_to};
    use std::fs;
    use std::path::PathBuf;

    use gpui_kit::{Bounds, WindowBounds, point, px, size};

    fn temp_path() -> PathBuf {
        std::env::temp_dir().join(format!("tbis-config-test-{}", uuid::Uuid::new_v4()))
    }

    #[test]
    fn missing_configuration_creates_saveable_defaults() {
        let path = temp_path();
        let config = load_from(&path);
        assert!(!config.load_failed);
        assert!(!config.device_id.is_empty());
        save_to(&config, &path).unwrap();
        assert!(path.exists());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn invalid_configuration_is_not_overwritten() {
        let path = temp_path();
        fs::write(&path, b"{").unwrap();
        let config = load_from(&path);
        assert!(config.load_failed);
        assert!(save_to(&config, &path).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"{");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn empty_device_identity_is_rejected_without_overwriting() {
        let path = temp_path();
        fs::write(&path, br#"{"device_id":""}"#).unwrap();
        let config = load_from(&path);
        assert!(config.load_failed);
        assert!(save_to(&config, &path).is_err());
        assert_eq!(fs::read(&path).unwrap(), br#"{"device_id":""}"#);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn unreadable_configuration_is_not_first_launch() {
        let path = temp_path();
        fs::create_dir(&path).unwrap();
        let config = load_from(&path);
        assert!(config.load_failed);
        assert!(config.load_error.is_some());
        fs::remove_dir(path).unwrap();
    }

    #[test]
    fn valid_configuration_loads_and_remains_saveable() {
        let config = defaults();
        let bytes = serde_json::to_vec(&config).unwrap();
        let path = temp_path();
        fs::write(&path, bytes).unwrap();
        let loaded = load_from(&path);
        assert!(!loaded.load_failed);
        assert_eq!(loaded.device_id, config.device_id);
        save_to(&loaded, &path).unwrap();
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn tailscale_options_load_old_configs_and_round_trip() {
        let mut value = serde_json::to_value(defaults()).unwrap();
        value.as_object_mut().unwrap().remove("tailscale");
        let mut config: super::Config = serde_json::from_value(value).unwrap();
        assert_eq!(config.tailscale, super::TailscaleOptions::default());
        config.tailscale.enabled = true;
        config.tailscale.address = "jellyfin:8096".into();
        let loaded: super::Config =
            serde_json::from_slice(&serde_json::to_vec(&config).unwrap()).unwrap();
        assert_eq!(loaded.tailscale, config.tailscale);
    }

    #[test]
    fn playback_preferences_preserve_old_track_memory() {
        let mut prefs: super::TrackPrefs =
            serde_json::from_str(r#"{"memory":{"episode":{"audio":2,"subtitle":3}},"global":{}}"#)
                .unwrap();
        assert!(prefs.playback.is_empty());
        assert_eq!(prefs.memory["episode"].audio, Some(2));
        prefs.playback.insert(
            "episode".into(),
            super::ItemPlaybackPrefs {
                speed: Some(1.5),
                bitrate_set: true,
                max_bitrate_mbps: None,
                subtitle_delay: Some(-5.),
            },
        );
        let saved = serde_json::to_string(&prefs).unwrap();
        let loaded: super::TrackPrefs = serde_json::from_str(&saved).unwrap();
        assert_eq!(loaded.memory["episode"].subtitle, Some(3));
        let item = loaded.playback["episode"];
        assert_eq!(item.speed, Some(1.5));
        assert!(item.bitrate_set);
        assert_eq!(item.max_bitrate_mbps, None);
        assert_eq!(item.subtitle_delay, Some(-5.));
        assert!(!loaded.playback.contains_key("other"));
    }

    #[test]
    fn restores_only_on_a_visible_display() {
        let display = Bounds::new(point(px(0.), px(0.)), size(px(1440.), px(900.)));
        let min = size(px(800.), px(560.));
        let saved = |x: f32, w: f32| WindowState {
            bounds: Bounds::new(point(px(x), px(40.)), size(px(w), px(700.))),
            maximized: false,
        };

        let Some(WindowBounds::Windowed(b)) = saved(100., 1000.).restore(&[display], min) else {
            panic!("on-screen bounds restore windowed");
        };
        assert_eq!(b.origin.x, px(100.));
        assert!(
            saved(3000., 1000.).restore(&[display], min).is_none(),
            "off-screen"
        );
        let Some(WindowBounds::Windowed(b)) = saved(100., 300.).restore(&[display], min) else {
            panic!();
        };
        assert_eq!(b.size.width, px(800.), "clamped to minimum");
    }
}
