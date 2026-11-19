//! Settings screen: opened from the title-bar user menu as a detail page. One
//! `SettingPage` per area; add a setting by adding a `SettingItem` to a group.

use gpui_kit::component::input::{InputEvent, TextareaState};
use gpui_kit::component::{IconName, v_flex};
use gpui_kit::*;

use crate::components::button::icon_button;
use crate::components::layout::{grow, page_header};
use crate::components::settings::{SettingField, SettingGroup, SettingItem, SettingPage, settings};
use crate::config::{
    AudioStyle, AutoSkip, BITRATE_CAPS, Config, HideSpoilers, HomeLayout, LanguagePref,
    LibraryGrouping, SeekSteps, ShowBackdrops, SubtitleStyle, VideoQuality,
};
use crate::mpv;
use crate::nav::Nav;
use crate::shaders::ShaderProfile;
use crate::theme;

/// Languages offered in Settings: canonical code (ISO 639-2/B, as most containers
/// tag tracks), label, and the other codes the same language turns up as.
const LANGUAGES: &[(&str, &str, &[&str])] = &[
    ("eng", "English", &["en"]),
    ("jpn", "Japanese", &["ja"]),
    ("spa", "Spanish", &["es"]),
    ("fre", "French", &["fra", "fr"]),
    ("ger", "German", &["deu", "de"]),
    ("ita", "Italian", &["it"]),
    ("por", "Portuguese", &["pt"]),
    ("rus", "Russian", &["ru"]),
    ("kor", "Korean", &["ko"]),
    ("chi", "Chinese", &["zho", "zh"]),
    ("hin", "Hindi", &["hi"]),
    ("ben", "Bengali", &["bn"]),
    ("ara", "Arabic", &["ar"]),
];

/// Whether two track language codes name the same language (`eng` = `en`, `fre` = `fra`).
pub fn same_language(a: &str, b: &str) -> bool {
    let canonical = |code: &str| {
        let code = code.to_ascii_lowercase();
        LANGUAGES
            .iter()
            .find(|(c, _, aliases)| *c == code || aliases.contains(&code.as_str()))
            .map_or(code, |(c, _, _)| c.to_string())
    };
    canonical(a) == canonical(b)
}

const NO_PREFERENCE: &str = "";
/// Pause in typing before mpv.conf is written.
const MPV_CONF_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(500);
const OFF: &str = "off";

/// Everything the Settings pages edit, as one value.
#[derive(Clone, PartialEq)]
pub struct Settings {
    pub language: LanguagePref,
    pub subtitles: SubtitleStyle,
    pub max_bitrate_mbps: Option<u32>,
    pub shaders: ShaderProfile,
    pub video_quality: VideoQuality,
    pub seek: SeekSteps,
    pub theme: String,
    pub hide_spoilers: HideSpoilers,
    pub backdrops: ShowBackdrops,
    pub home_layout: HomeLayout,
    pub library_grouping: LibraryGrouping,
    pub audio: AudioStyle,
    pub auto_skip: AutoSkip,
}

impl Settings {
    pub fn from_config(config: &Config) -> Self {
        Self {
            language: config.language.clone(),
            subtitles: config.subtitles.clone(),
            max_bitrate_mbps: config.max_bitrate_mbps,
            shaders: config.shaders,
            video_quality: config.video_quality,
            seek: config.seek,
            theme: theme::active(config).to_string(),
            hide_spoilers: config.hide_spoilers,
            backdrops: config.backdrops,
            home_layout: config.home_layout,
            library_grouping: config.library_grouping,
            audio: config.audio,
            auto_skip: config.auto_skip,
        }
    }

    pub fn write_to(&self, config: &mut Config) {
        config.language = self.language.clone();
        config.subtitles = self.subtitles.clone();
        config.max_bitrate_mbps = self.max_bitrate_mbps;
        config.shaders = self.shaders;
        config.video_quality = self.video_quality;
        config.seek = self.seek;
        config.theme = Some(self.theme.clone());
        config.hide_spoilers = self.hide_spoilers;
        config.backdrops = self.backdrops;
        config.home_layout = self.home_layout;
        config.library_grouping = self.library_grouping;
        config.audio = self.audio;
        config.auto_skip = self.auto_skip;
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: LanguagePref::default(),
            subtitles: SubtitleStyle::default(),
            max_bitrate_mbps: None,
            shaders: ShaderProfile::default(),
            video_quality: VideoQuality::default(),
            seek: SeekSteps::default(),
            theme: theme::NAMES[0].into(),
            hide_spoilers: HideSpoilers::default(),
            backdrops: ShowBackdrops::default(),
            home_layout: HomeLayout::default(),
            library_grouping: LibraryGrouping::default(),
            audio: AudioStyle::default(),
            auto_skip: AutoSkip::default(),
        }
    }
}

/// A setting changed; the app saves it.
pub struct SettingsChanged(pub Settings);

pub struct SettingsView {
    settings: Entity<Settings>,
    shown: Settings,
    mpv_conf: Entity<TextareaState>,
    /// Reset was clicked once; a second click within a few seconds resets.
    confirm_reset: bool,
    _confirm_reset: Task<()>,
    _observe: Subscription,
    _save_mpv_conf: Subscription,
    /// Text not yet written; saved on drop so leaving Settings mid-debounce keeps it.
    mpv_conf_pending: Option<String>,
    /// Debounced mpv.conf write; replacing it restarts the wait.
    _write_mpv_conf: Task<()>,
}

impl Drop for SettingsView {
    fn drop(&mut self) {
        self.flush_mpv_conf();
    }
}

fn save_mpv_conf(text: &str) {
    if let Err(err) = mpv::save_user_config(text) {
        crate::log::write(&format!("saving mpv.conf failed: {err}"));
    }
}

impl EventEmitter<Nav> for SettingsView {}
impl EventEmitter<SettingsChanged> for SettingsView {}

impl SettingsView {
    pub fn new(config: &Config, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let shown = Settings::from_config(config);
        let settings = cx.new(|_| shown.clone());
        let mpv_conf = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(6, 20)
                .default_value(mpv::user_config())
        });
        let _save_mpv_conf = cx.subscribe(&mpv_conf, |this, state, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                let text = state.read(cx).value().to_string();
                this.mpv_conf_pending = Some(text.clone());
                this._write_mpv_conf = cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(MPV_CONF_DEBOUNCE).await;
                    save_mpv_conf(&text);
                    this.update(cx, |this, _| this.mpv_conf_pending = None).ok();
                });
            }
        });
        cx.on_app_quit(|this, _| {
            this.flush_mpv_conf();
            async {}
        })
        .detach();
        let _observe = cx.observe(&settings, |this, settings, cx| {
            let new = settings.read(cx).clone();
            if new.theme != this.shown.theme {
                theme::apply(&new.theme, cx);
            }
            this.shown = new.clone();
            cx.emit(SettingsChanged(new));
        });
        Self {
            settings,
            shown,
            mpv_conf,
            confirm_reset: false,
            _confirm_reset: Task::ready(()),
            _observe,
            _save_mpv_conf,
            mpv_conf_pending: None,
            _write_mpv_conf: Task::ready(()),
        }
    }

    fn flush_mpv_conf(&mut self) {
        if let Some(text) = self.mpv_conf_pending.take() {
            save_mpv_conf(&text);
        }
    }

    /// Seconds dropdown over one `SeekSteps` field.
    fn seek_item(
        &self,
        title: &'static str,
        choices: &[u32],
        field: fn(&mut SeekSteps) -> &mut u32,
    ) -> SettingItem {
        let (read, write) = (self.settings.clone(), self.settings.clone());
        SettingItem::new(
            title,
            SettingField::dropdown(
                choices
                    .iter()
                    .map(|s| (s.to_string().into(), format!("{s} s").into()))
                    .collect(),
                move |cx| field(&mut read.read(cx).seek.clone()).to_string().into(),
                move |value, cx| {
                    write.update(cx, |steps, cx| {
                        if let Ok(seconds) = value.parse() {
                            *field(&mut steps.seek) = seconds;
                            cx.notify();
                        }
                    })
                },
            ),
        )
    }

    fn reset(&mut self, cx: &mut Context<Self>) {
        self.settings.update(cx, |settings, cx| {
            *settings = Settings::default();
            cx.notify();
        });
    }

    /// First click arms the reset for a few seconds; a second click within them resets.
    fn reset_clicked(&mut self, cx: &mut Context<Self>) {
        if self.confirm_reset {
            self.confirm_reset = false;
            self._confirm_reset = Task::ready(());
            self.reset(cx);
        } else {
            self.confirm_reset = true;
            self._confirm_reset = cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(4))
                    .await;
                this.update(cx, |this, cx| {
                    this.confirm_reset = false;
                    cx.notify();
                })
                .ok();
            });
        }
        cx.notify();
    }

    fn general_page(&self, cx: &Context<Self>) -> SettingPage {
        let label = if self.confirm_reset {
            "Click again to reset"
        } else {
            "Reset"
        };
        let view = cx.entity().downgrade();
        let reset = SettingItem::new(
            "Reset all settings",
            SettingField::action(label, move |cx| {
                view.update(cx, |this, cx| this.reset_clicked(cx)).ok();
            }),
        )
        .description(
            "Restores everything on these pages. Your sign-in, window size and mpv.conf are kept.",
        );
        SettingPage::new("General").group(SettingGroup::new().items([reset]))
    }

    fn seek_group(&self) -> SettingGroup {
        let (read, write) = (self.settings.clone(), self.settings.clone());
        let auto_skip = SettingItem::new(
            "Skip intros and credits automatically",
            SettingField::switch(
                move |cx| read.read(cx).auto_skip.0,
                move |on, cx| {
                    write.update(cx, |value, cx| {
                        value.auto_skip = AutoSkip(on);
                        cx.notify();
                    })
                },
            ),
        )
        .description("Seeking back into a skipped intro plays it.");
        SettingGroup::new().title("Seeking").items([
            auto_skip,
            self.seek_item("Short seek (\u{2190} \u{2192})", &[5, 10, 15, 30], |s| {
                &mut s.short
            })
            .description("Also used by media keys and Control Center."),
            self.seek_item(
                "Long seek (\u{2325}\u{2190} \u{2325}\u{2192})",
                &[30, 60, 120, 300],
                |s| &mut s.long,
            ),
        ])
    }

    fn appearance_page(&self) -> SettingPage {
        let (read, write) = (self.settings.clone(), self.settings.clone());
        let theme = SettingItem::new(
            "Theme",
            SettingField::dropdown(
                theme::NAMES
                    .iter()
                    .map(|n| (SharedString::from(*n), SharedString::from(*n)))
                    .collect(),
                move |cx| read.read(cx).theme.clone().into(),
                move |value, cx| {
                    write.update(cx, |theme, cx| {
                        theme.theme = value.to_string();
                        cx.notify();
                    })
                },
            ),
        );
        let (read, write) = (self.settings.clone(), self.settings.clone());
        let spoilers = SettingItem::new(
            "Hide episode spoilers",
            SettingField::switch(
                move |cx| read.read(cx).hide_spoilers.0,
                move |hide, cx| {
                    write.update(cx, |value, cx| {
                        value.hide_spoilers = HideSpoilers(hide);
                        cx.notify();
                    })
                },
            ),
        )
        .description("Blur the images and hide the descriptions of episodes you haven't watched.");
        let (read, write) = (self.settings.clone(), self.settings.clone());
        let backdrops = SettingItem::new(
            "Backdrops",
            SettingField::switch(
                move |cx| read.read(cx).backdrops.0,
                move |show, cx| {
                    write.update(cx, |value, cx| {
                        value.backdrops = ShowBackdrops(show);
                        cx.notify();
                    })
                },
            ),
        )
        .description("Show a dimmed backdrop from your library behind Home, Movies and Series.");
        let (read, write) = (self.settings.clone(), self.settings.clone());
        let layout = SettingItem::new(
            "Layout",
            SettingField::dropdown(
                HomeLayout::ALL
                    .iter()
                    .map(|l| (SharedString::from(l.label()), SharedString::from(l.label())))
                    .collect(),
                move |cx| read.read(cx).home_layout.label().into(),
                move |value, cx| {
                    write.update(cx, |layout, cx| {
                        layout.home_layout = HomeLayout::ALL
                            .into_iter()
                            .find(|l| l.label() == value.as_ref())
                            .unwrap_or_default();
                        cx.notify();
                    })
                },
            ),
        )
        .description("Modern puts a featured item on top of Home, then a row for each Library.");
        let (read, write) = (self.settings.clone(), self.settings.clone());
        let grouping = SettingItem::new(
            "Libraries",
            SettingField::dropdown(
                LibraryGrouping::ALL
                    .iter()
                    .map(|g| (SharedString::from(g.label()), SharedString::from(g.label())))
                    .collect(),
                move |cx| read.read(cx).library_grouping.label().into(),
                move |value, cx| {
                    write.update(cx, |grouping, cx| {
                        grouping.library_grouping = LibraryGrouping::ALL
                            .into_iter()
                            .find(|g| g.label() == value.as_ref())
                            .unwrap_or_default();
                        cx.notify();
                    })
                },
            ),
        )
        .description("Separate adds a tab for each Library on Movies and Series.");
        SettingPage::new("Appearance")
            .group(SettingGroup::new().items([theme, layout, grouping, backdrops, spoilers]))
    }

    fn video_group(&self) -> SettingGroup {
        let (read, write) = (self.settings.clone(), self.settings.clone());
        let quality = SettingItem::new(
            "Quality",
            SettingField::dropdown(
                VideoQuality::ALL
                    .iter()
                    .map(|q| (SharedString::from(q.label()), SharedString::from(q.label())))
                    .collect(),
                move |cx| read.read(cx).video_quality.label().into(),
                move |value, cx| {
                    write.update(cx, |quality, cx| {
                        quality.video_quality = VideoQuality::ALL
                            .into_iter()
                            .find(|q| q.label() == value.as_ref())
                            .unwrap_or_default();
                        cx.notify();
                    })
                },
            ),
        )
        .description(
            "Fast saves power; High looks best on a capable GPU. Applies to the next video.",
        );
        let (read, write) = (self.settings.clone(), self.settings.clone());
        let upscaling = SettingItem::new(
            "Upscaling",
            SettingField::dropdown(
                ShaderProfile::ALL
                    .iter()
                    .map(|p| (SharedString::from(p.label()), SharedString::from(p.label())))
                    .collect(),
                move |cx| read.read(cx).shaders.label().into(),
                move |value, cx| {
                    write.update(cx, |profile, cx| {
                        profile.shaders = ShaderProfile::ALL
                            .into_iter()
                            .find(|p| p.label() == value.as_ref())
                            .unwrap_or_default();
                        cx.notify();
                    })
                },
            ),
        )
        .description(
            "Anime4K sharpens and upscales animation; not meant for live action. \
             Quality needs a strong GPU. Press U in the Player to cycle.",
        );
        SettingGroup::new()
            .title("Video")
            .items([quality, upscaling])
    }

    fn mpv_group(&self) -> SettingGroup {
        let conf = SettingItem::new("mpv.conf", SettingField::code(self.mpv_conf.clone()))
            .description(
                "Any mpv option, one per line, such as deband=yes or tone-mapping=hable. \
                 Overrides the settings above.",
            );
        SettingGroup::new()
            .title("mpv")
            .description("Applies from the next playback.")
            .items([conf])
    }

    fn audio_group(&self) -> SettingGroup {
        let (read, write) = (self.settings.clone(), self.settings.clone());
        let passthrough = SettingItem::new(
            "Bitstream passthrough",
            SettingField::switch(
                move |cx| read.read(cx).audio.passthrough,
                move |on, cx| {
                    write.update(cx, |style, cx| {
                        style.audio.passthrough = on;
                        cx.notify();
                    })
                },
            ),
        )
        .description("Send AC3/DTS/E-AC3 straight to an AV receiver instead of decoding it here.");
        let (read, write) = (self.settings.clone(), self.settings.clone());
        let loudness = SettingItem::new(
            "Loudness normalization",
            SettingField::switch(
                move |cx| read.read(cx).audio.loudness_norm,
                move |on, cx| {
                    write.update(cx, |style, cx| {
                        style.audio.loudness_norm = on;
                        cx.notify();
                    })
                },
            ),
        )
        .description("Evens out quiet dialogue and loud effects (\"night mode\").");
        SettingGroup::new()
            .title("Audio")
            .description("Applies from the next playback.")
            .items([passthrough, loudness])
    }

    fn streaming_group(&self) -> SettingGroup {
        let (read, write) = (self.settings.clone(), self.settings.clone());
        let cap = SettingItem::new(
            "Maximum bitrate",
            SettingField::dropdown(
                BITRATE_CAPS
                    .iter()
                    .map(|(cap, label)| {
                        let value = cap.map(|m| m.to_string()).unwrap_or_default();
                        (SharedString::from(value), SharedString::from(*label))
                    })
                    .collect(),
                move |cx| {
                    read.read(cx)
                        .max_bitrate_mbps
                        .map(|m| m.to_string())
                        .unwrap_or_default()
                        .into()
                },
                move |value, cx| {
                    write.update(cx, |cap, cx| {
                        cap.max_bitrate_mbps = value.parse().ok();
                        cx.notify();
                    })
                },
            ),
        )
        .description(
            "Files above the limit are transcoded by the server; Original plays the file \
             itself. Applies from the next playback; the Player can change it per session.",
        );
        SettingGroup::new().title("Streaming").items([cap])
    }

    /// Dropdown over `SubtitleStyle` field: `options` as (value, label), with
    /// `get`/`set` mapping the field to and from the option value.
    fn subtitle_item(
        &self,
        title: &'static str,
        options: &[(&str, &str)],
        get: fn(&SubtitleStyle) -> String,
        set: fn(&mut SubtitleStyle, &str),
    ) -> SettingItem {
        let (read, write) = (self.settings.clone(), self.settings.clone());
        SettingItem::new(
            title,
            SettingField::dropdown(
                options
                    .iter()
                    .map(|(v, l)| {
                        (
                            SharedString::from(v.to_string()),
                            SharedString::from(l.to_string()),
                        )
                    })
                    .collect(),
                move |cx| get(&read.read(cx).subtitles).into(),
                move |value, cx| {
                    write.update(cx, |style, cx| {
                        set(&mut style.subtitles, &value);
                        cx.notify();
                    })
                },
            ),
        )
    }

    fn subtitles_group(&self) -> SettingGroup {
        SettingGroup::new()
            .title("Subtitles")
            .description("Text subtitles only; image subtitles keep their own look. Applies from the next playback.")
            .items([
                self.subtitle_item(
                    "Size",
                    &[("75", "Small"), ("100", "Default"), ("125", "Large"), ("150", "Larger"), ("200", "Huge")],
                    |s| s.scale.to_string(),
                    |s, v| s.scale = v.parse().unwrap_or(100),
                ),
                self.subtitle_item(
                    "Colour",
                    &[("#FFFFFF", "White"), ("#FFE66D", "Yellow"), ("#D8D8D8", "Soft grey"), ("#7FDBFF", "Cyan")],
                    |s| s.color.clone(),
                    |s, v| s.color = v.to_string(),
                ),
                self.subtitle_item(
                    "Position",
                    &[("100", "Bottom"), ("95", "Slightly raised"), ("88", "Raised")],
                    |s| s.position.to_string(),
                    |s, v| s.position = v.parse().unwrap_or(100),
                ),
                self.subtitle_item(
                    "Style",
                    &[("outline", "Outline"), ("box", "Dark box")],
                    |s| if s.background { "box" } else { "outline" }.into(),
                    |s, v| s.background = v == "box",
                ),
            ])
    }

    fn language_options(
        extra: &[(&'static str, &'static str)],
    ) -> Vec<(SharedString, SharedString)> {
        extra
            .iter()
            .map(|(value, label)| (SharedString::from(*value), SharedString::from(*label)))
            .chain(
                LANGUAGES.iter().map(|(code, label, _)| {
                    (SharedString::from(*code), SharedString::from(*label))
                }),
            )
            .collect()
    }

    fn playback_page(&self) -> SettingPage {
        let (audio_get, audio_set) = (self.settings.clone(), self.settings.clone());
        let audio = SettingItem::new(
            "Audio language",
            SettingField::dropdown(
                Self::language_options(&[(NO_PREFERENCE, "No preference")]),
                move |cx| {
                    audio_get
                        .read(cx)
                        .language
                        .audio_lang
                        .clone()
                        .unwrap_or_default()
                        .into()
                },
                move |value, cx| {
                    audio_set.update(cx, |pref, cx| {
                        pref.language.audio_lang = (!value.is_empty()).then(|| value.to_string());
                        cx.notify();
                    })
                },
            ),
        )
        .description("Used when an item has no remembered audio track.");

        let (subs_get, subs_set) = (self.settings.clone(), self.settings.clone());
        let subtitles = SettingItem::new(
            "Subtitle language",
            SettingField::dropdown(
                Self::language_options(&[(NO_PREFERENCE, "No preference"), (OFF, "Off")]),
                move |cx| {
                    let pref = &subs_get.read(cx).language;
                    match (pref.subtitles_enabled, &pref.subtitle_lang) {
                        (Some(false), _) => OFF.into(),
                        (Some(true), Some(lang)) => lang.clone().into(),
                        _ => NO_PREFERENCE.into(),
                    }
                },
                move |value, cx| {
                    subs_set.update(cx, |pref, cx| {
                        (pref.language.subtitles_enabled, pref.language.subtitle_lang) =
                            match value.as_ref() {
                                NO_PREFERENCE => (None, None),
                                OFF => (Some(false), None),
                                lang => (Some(true), Some(lang.to_string())),
                            };
                        cx.notify();
                    })
                },
            ),
        )
        .description("Used when an item has no remembered subtitle track.");

        SettingPage::new("Playback").groups([
            SettingGroup::new()
                .title("Language")
                .items([audio, subtitles]),
            self.subtitles_group(),
            self.seek_group(),
            self.video_group(),
            self.audio_group(),
            self.streaming_group(),
            self.mpv_group(),
        ])
    }
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(page_header(
                Some(
                    icon_button("settings-back", IconName::ArrowLeft, "Back", cx)
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(Nav::Back))),
                ),
                "Settings",
            ))
            .child(grow().child(settings("settings").pages([
                self.playback_page(),
                self.appearance_page(),
                self.general_page(cx),
            ])))
    }
}

#[cfg(test)]
mod tests {
    use super::{Settings, same_language};
    use crate::config::Config;
    use crate::theme;
    use serde_json::{Value, json};

    const SETTING_KEYS: [&str; 13] = [
        "language",
        "subtitles",
        "max_bitrate_mbps",
        "shaders",
        "video_quality",
        "seek",
        "theme",
        "hide_spoilers",
        "backdrops",
        "home_layout",
        "library_grouping",
        "audio",
        "auto_skip",
    ];

    fn saved_config() -> Value {
        json!({
            "device_id": "settings-test",
            "session": null,
            "movies_sort": "Year",
            "series_sort": "Rating",
            "volume": 37.0,
            "muted": true,
            "track_prefs": {
                "memory": {
                    "episode": {"audio": 2, "subtitle": 3, "subtitle_off": false}
                },
                "playback": {
                    "episode": {
                        "speed": 1.5,
                        "bitrate_set": true,
                        "max_bitrate_mbps": 10,
                        "subtitle_delay": -5.0
                    }
                },
                "series": {
                    "series": {
                        "audio_lang": "jpn",
                        "subtitle_lang": "eng",
                        "subtitles_enabled": true
                    }
                },
                "global": {
                    "audio_lang": "fre",
                    "subtitle_lang": null,
                    "subtitles_enabled": false
                }
            },
            "recent_searches": ["remember this"],
            "window": null,
            "language": {
                "audio_lang": "jpn",
                "subtitle_lang": "eng",
                "subtitles_enabled": true
            },
            "subtitles": {
                "scale": 150,
                "color": "#FFE66D",
                "position": 88,
                "background": true
            },
            "max_bitrate_mbps": 20,
            "shaders": "Anime4kQuality",
            "video_quality": "Fast",
            "seek": {"short": 5, "long": 300},
            "theme": theme::NAMES[1],
            "hide_spoilers": false,
            "backdrops": false,
            "home_layout": "Simple",
            "library_grouping": "Separate",
            "audio": {"passthrough": true, "loudness_norm": true},
            "auto_skip": false
        })
    }

    fn default_config() -> Config {
        serde_json::from_str(r#"{"device_id":"settings-default","session":null}"#).unwrap()
    }

    #[test]
    fn all_thirteen_settings_round_trip_to_another_config() {
        let source = saved_config();
        let config: Config = serde_json::from_value(source.clone()).unwrap();
        let settings = Settings::from_config(&config);
        let mut target = default_config();
        settings.write_to(&mut target);
        assert!(Settings::from_config(&target) == settings);
        let saved = serde_json::to_value(target).unwrap();
        for key in SETTING_KEYS {
            assert_eq!(saved[key], source[key], "{key}");
        }
    }

    #[test]
    fn existing_config_keeps_its_serialization_shape() {
        let source = saved_config();
        let mut config: Config = serde_json::from_value(source.clone()).unwrap();
        Settings::from_config(&config).write_to(&mut config);
        let saved = serde_json::to_value(config).unwrap();
        assert_eq!(saved, source);
        let reloaded: Config = serde_json::from_value(saved).unwrap();
        assert_eq!(serde_json::to_value(reloaded).unwrap(), source);
    }

    #[test]
    fn reset_restores_all_defaults_and_preserves_unrelated_config() {
        let source = saved_config();
        let mut config: Config = serde_json::from_value(source.clone()).unwrap();
        let mut settings = Settings::from_config(&config);
        assert!(settings != Settings::default());
        settings = Settings::default();
        settings.write_to(&mut config);
        assert!(Settings::from_config(&config) == Settings::default());
        let saved = serde_json::to_value(config).unwrap();
        let expected = json!({
            "language": {
                "audio_lang": null,
                "subtitle_lang": null,
                "subtitles_enabled": null
            },
            "subtitles": {
                "scale": 100,
                "color": "#FFFFFF",
                "position": 100,
                "background": false
            },
            "max_bitrate_mbps": null,
            "shaders": "Off",
            "video_quality": "High",
            "seek": {"short": 10, "long": 60},
            "theme": theme::NAMES[0],
            "hide_spoilers": true,
            "backdrops": true,
            "home_layout": "Modern",
            "library_grouping": "Merged",
            "audio": {"passthrough": false, "loudness_norm": false},
            "auto_skip": true
        });
        assert_eq!(
            saved.as_object().unwrap().len(),
            source.as_object().unwrap().len()
        );
        for (key, value) in source.as_object().unwrap() {
            assert!(saved.get(key).is_some(), "missing key: {key}");
            if SETTING_KEYS.contains(&key.as_str()) {
                assert_eq!(saved[key], expected[key], "{key}");
            } else {
                assert_eq!(&saved[key], value, "{key}");
            }
        }
    }

    #[test]
    fn absent_or_unknown_theme_uses_the_default() {
        let mut config = default_config();
        assert_eq!(Settings::from_config(&config).theme, theme::NAMES[0]);
        config.theme = Some("unknown theme".into());
        assert_eq!(Settings::from_config(&config).theme, theme::NAMES[0]);
    }

    #[test]
    fn language_codes_match_across_forms() {
        assert!(same_language("eng", "en"));
        assert!(same_language("FRA", "fre"));
        assert!(same_language("ger", "de"));
        assert!(!same_language("eng", "jpn"));
        assert!(same_language("xyz", "XYZ"), "unknown codes compare as-is");
    }
}
