//! Settings screen: opened from the title-bar user menu as a detail page. One
//! `SettingPage` per area; add a setting by adding a `SettingItem` to a group.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::setting::{
    SettingField, SettingGroup, SettingItem, SettingPage, Settings,
};
use gpui_kit::component::{IconName, Sizable as _, h_flex, v_flex};
use gpui_kit::*;

use crate::config::{Config, HideSpoilers, LanguagePref, SeekSteps, SubtitleStyle};
use crate::detail::PAD;
use crate::jellyfin::Api;
use crate::nav::Nav;
use crate::shaders::ShaderProfile;
use crate::stats::StatsView;
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
const OFF: &str = "off";

/// A setting changed; the app saves it.
pub enum SettingsChanged {
    Language(LanguagePref),
    Subtitles(SubtitleStyle),
    MaxBitrate(Option<u32>),
    Shaders(ShaderProfile),
    Seek(SeekSteps),
    Theme(String),
    HideSpoilers(HideSpoilers),
}

pub struct SettingsView {
    language: Entity<LanguagePref>,
    subtitles: Entity<SubtitleStyle>,
    max_bitrate: Entity<Option<u32>>,
    shaders: Entity<ShaderProfile>,
    seek: Entity<SeekSteps>,
    theme: Entity<SharedString>,
    hide_spoilers: Entity<HideSpoilers>,
    stats: Entity<StatsView>,
    _observe: [Subscription; 7],
}

impl EventEmitter<Nav> for SettingsView {}
impl EventEmitter<SettingsChanged> for SettingsView {}

impl SettingsView {
    pub fn new(config: &Config, api: Api, cx: &mut Context<Self>) -> Self {
        let stats = cx.new(|cx| StatsView::new(api, cx));
        let language = cx.new(|_| config.language.clone());
        let subtitles = cx.new(|_| config.subtitles.clone());
        let max_bitrate = cx.new(|_| config.max_bitrate_mbps);
        let shaders = cx.new(|_| config.shaders);
        let seek = cx.new(|_| config.seek);
        let theme = cx.new(|_| SharedString::from(theme::active(config)));
        let hide_spoilers = cx.new(|_| config.hide_spoilers);
        let _observe = [
            cx.observe(&language, |_, language, cx| {
                cx.emit(SettingsChanged::Language(language.read(cx).clone()))
            }),
            cx.observe(&subtitles, |_, subtitles, cx| {
                cx.emit(SettingsChanged::Subtitles(subtitles.read(cx).clone()))
            }),
            cx.observe(&max_bitrate, |_, max_bitrate, cx| {
                cx.emit(SettingsChanged::MaxBitrate(*max_bitrate.read(cx)))
            }),
            cx.observe(&shaders, |_, shaders, cx| {
                cx.emit(SettingsChanged::Shaders(*shaders.read(cx)))
            }),
            cx.observe(&seek, |_, seek, cx| {
                cx.emit(SettingsChanged::Seek(*seek.read(cx)))
            }),
            cx.observe(&theme, |_, theme, cx| {
                let name = theme.read(cx).to_string();
                theme::apply(&name, cx);
                cx.emit(SettingsChanged::Theme(name))
            }),
            cx.observe(&hide_spoilers, |_, hide, cx| {
                cx.emit(SettingsChanged::HideSpoilers(*hide.read(cx)))
            }),
        ];
        Self {
            language,
            subtitles,
            max_bitrate,
            shaders,
            seek,
            theme,
            hide_spoilers,
            stats,
            _observe,
        }
    }

    /// Seconds dropdown over one `SeekSteps` field.
    fn seek_item(
        &self,
        title: &'static str,
        choices: &[u32],
        field: fn(&mut SeekSteps) -> &mut u32,
    ) -> SettingItem {
        let (read, write) = (self.seek.clone(), self.seek.clone());
        SettingItem::new(
            title,
            SettingField::dropdown(
                choices
                    .iter()
                    .map(|s| (s.to_string().into(), format!("{s} s").into()))
                    .collect(),
                move |cx| field(&mut read.read(cx).clone()).to_string().into(),
                move |value, cx| {
                    write.update(cx, |steps, cx| {
                        if let Ok(seconds) = value.parse() {
                            *field(steps) = seconds;
                            cx.notify();
                        }
                    })
                },
            ),
        )
    }

    fn seek_group(&self) -> SettingGroup {
        SettingGroup::new().title("Seeking").items([
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
        let (read, write) = (self.theme.clone(), self.theme.clone());
        let theme = SettingItem::new(
            "Theme",
            SettingField::dropdown(
                theme::NAMES
                    .iter()
                    .map(|n| (SharedString::from(*n), SharedString::from(*n)))
                    .collect(),
                move |cx| read.read(cx).clone(),
                move |value, cx| {
                    write.update(cx, |theme, cx| {
                        *theme = value;
                        cx.notify();
                    })
                },
            ),
        );
        let (read, write) = (self.hide_spoilers.clone(), self.hide_spoilers.clone());
        let spoilers = SettingItem::new(
            "Hide episode spoilers",
            SettingField::switch(
                move |cx| read.read(cx).0,
                move |hide, cx| {
                    write.update(cx, |value, cx| {
                        *value = HideSpoilers(hide);
                        cx.notify();
                    })
                },
            ),
        )
        .description("Blur the images and hide the descriptions of episodes you haven't watched.");
        SettingPage::new("Appearance").group(SettingGroup::new().items([theme, spoilers]))
    }

    fn stats_page(&self) -> SettingPage {
        let stats = self.stats.clone();
        SettingPage::new("Stats").group(
            SettingGroup::new()
                .title("Watch stats")
                .description("From the server's play history. Recent totals count each item at its last play.")
                .item(SettingItem::render(move |_, _, _| stats.clone())),
        )
    }

    fn video_group(&self) -> SettingGroup {
        let (read, write) = (self.shaders.clone(), self.shaders.clone());
        let upscaling = SettingItem::new(
            "Upscaling",
            SettingField::dropdown(
                ShaderProfile::ALL
                    .iter()
                    .map(|p| (SharedString::from(p.label()), SharedString::from(p.label())))
                    .collect(),
                move |cx| read.read(cx).label().into(),
                move |value, cx| {
                    write.update(cx, |profile, cx| {
                        *profile = ShaderProfile::ALL
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
        SettingGroup::new().title("Video").items([upscaling])
    }

    fn streaming_group(&self) -> SettingGroup {
        let (read, write) = (self.max_bitrate.clone(), self.max_bitrate.clone());
        let options = [
            ("", "No limit (direct play)"),
            ("40", "40 Mbps"),
            ("20", "20 Mbps"),
            ("10", "10 Mbps"),
            ("4", "4 Mbps"),
            ("2", "2 Mbps"),
        ];
        let cap = SettingItem::new(
            "Maximum bitrate",
            SettingField::dropdown(
                options
                    .iter()
                    .map(|(v, l)| (SharedString::from(*v), SharedString::from(*l)))
                    .collect(),
                move |cx| {
                    read.read(cx)
                        .map(|m| m.to_string())
                        .unwrap_or_default()
                        .into()
                },
                move |value, cx| {
                    write.update(cx, |cap, cx| {
                        *cap = value.parse().ok();
                        cx.notify();
                    })
                },
            ),
        )
        .description(
            "Files above the limit are transcoded by the server. Applies from the next playback.",
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
        let (read, write) = (self.subtitles.clone(), self.subtitles.clone());
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
                move |cx| get(read.read(cx)).into(),
                move |value, cx| {
                    write.update(cx, |style, cx| {
                        set(style, &value);
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
        let (audio_get, audio_set) = (self.language.clone(), self.language.clone());
        let audio = SettingItem::new(
            "Audio language",
            SettingField::scrollable_dropdown(
                Self::language_options(&[(NO_PREFERENCE, "No preference")]),
                move |cx| {
                    audio_get
                        .read(cx)
                        .audio_lang
                        .clone()
                        .unwrap_or_default()
                        .into()
                },
                move |value, cx| {
                    audio_set.update(cx, |pref, cx| {
                        pref.audio_lang = (!value.is_empty()).then(|| value.to_string());
                        cx.notify();
                    })
                },
            ),
        )
        .description("Used when an item has no remembered audio track.");

        let (subs_get, subs_set) = (self.language.clone(), self.language.clone());
        let subtitles = SettingItem::new(
            "Subtitle language",
            SettingField::scrollable_dropdown(
                Self::language_options(&[(NO_PREFERENCE, "No preference"), (OFF, "Off")]),
                move |cx| {
                    let pref = subs_get.read(cx);
                    match (pref.subtitles_enabled, &pref.subtitle_lang) {
                        (Some(false), _) => OFF.into(),
                        (Some(true), Some(lang)) => lang.clone().into(),
                        _ => NO_PREFERENCE.into(),
                    }
                },
                move |value, cx| {
                    subs_set.update(cx, |pref, cx| {
                        (pref.subtitles_enabled, pref.subtitle_lang) = match value.as_ref() {
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

        SettingPage::new("Playback").default_open(true).groups([
            SettingGroup::new()
                .title("Language")
                .items([audio, subtitles]),
            self.subtitles_group(),
            self.seek_group(),
            self.video_group(),
            self.streaming_group(),
        ])
    }
}

impl Render for SettingsView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                h_flex()
                    .p(px(PAD))
                    .pb_2()
                    .gap_3()
                    .child(
                        Button::new("settings-back")
                            .ghost()
                            .icon(IconName::ArrowLeft)
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(Nav::Back))),
                    )
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::BOLD)
                            .child("Settings"),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(Settings::new("settings").small().pages([
                        self.playback_page(),
                        self.appearance_page(),
                        self.stats_page(),
                    ])),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::same_language;

    #[test]
    fn language_codes_match_across_forms() {
        assert!(same_language("eng", "en"));
        assert!(same_language("FRA", "fre"));
        assert!(same_language("ger", "de"));
        assert!(!same_language("eng", "jpn"));
        assert!(same_language("xyz", "XYZ"), "unknown codes compare as-is");
    }
}
