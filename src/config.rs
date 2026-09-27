use std::collections::HashMap;
use std::fs::{self, OpenOptions, Permissions};
use std::io::{self, Write as _};
use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
use std::path::PathBuf;

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
}

/// Blur the stills and hide the descriptions of unwatched Episodes (Settings).
/// Also an app global, so Episode lists read the live value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HideSpoilers(pub bool);

impl gpui_kit::Global for HideSpoilers {}

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
            window: None,
            subtitles: SubtitleStyle::default(),
            max_bitrate_mbps: None,
            shaders: ShaderProfile::Off,
            seek: SeekSteps::default(),
            recent_searches: Vec::new(),
            theme: None,
            hide_spoilers: HideSpoilers::default(),
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

#[cfg(test)]
mod tests {
    use super::WindowState;
    use gpui_kit::{Bounds, WindowBounds, point, px, size};

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
