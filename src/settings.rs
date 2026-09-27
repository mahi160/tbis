//! Settings screen: opened from the title-bar user menu as a detail page. One
//! `SettingPage` per area; add a setting by adding a `SettingItem` to a group.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::setting::{
    SettingField, SettingGroup, SettingItem, SettingPage, Settings,
};
use gpui_kit::component::{IconName, h_flex, v_flex};
use gpui_kit::*;

use crate::config::LanguagePref;
use crate::detail::PAD;
use crate::nav::Nav;

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

/// Preferred languages changed; the app saves them.
pub struct LanguageChanged(pub LanguagePref);

pub struct SettingsView {
    language: Entity<LanguagePref>,
    _observe: Subscription,
}

impl EventEmitter<Nav> for SettingsView {}
impl EventEmitter<LanguageChanged> for SettingsView {}

impl SettingsView {
    pub fn new(language: LanguagePref, cx: &mut Context<Self>) -> Self {
        let language = cx.new(|_| language);
        let _observe = cx.observe(&language, |_, language, cx| {
            cx.emit(LanguageChanged(language.read(cx).clone()))
        });
        Self { language, _observe }
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

        SettingPage::new("Playback").default_open(true).group(
            SettingGroup::new()
                .title("Language")
                .items([audio, subtitles]),
        )
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
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Settings"),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(Settings::new("settings").page(self.playback_page())),
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
