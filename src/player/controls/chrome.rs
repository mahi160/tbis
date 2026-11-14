//! Player chrome: the top bar (back, title, tags, clock) and the bottom bar
//! (timeline, transport, menus), shown together while the controls are up.

use gpui_kit::component::slider::Slider;
use gpui_kit::component::{Disableable as _, Icon, Selectable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::components::button::{round_ghost_button, round_icon, round_icon_button};
use crate::components::layout::spacer;
use crate::components::menu::{choice_menu, choice_menu_with_submenu};
use crate::components::video::{
    bottom_bar, hover_reveal, sprite_thumb, timeline, top_bar, video_meta, video_tag, video_title,
};
use crate::config::BITRATE_CAPS;
use crate::design::{self, color, font, gap, overlay, text};
use crate::mpv::{Track, TrackKind};
use crate::platform::video::SUPPORTS_PIP;

use super::super::clock::{clock, format_time};
use super::super::{
    CONTEXT, NextEpisode, PlayerView, PreviousEpisode, ToggleFullscreen, ToggleMute, TogglePause,
    TogglePip, TogglePlaybackInfo,
};
use super::SPEEDS;

/// Menu rows for `tracks`: (mpv track id, label, checked); `with_off` leads with "Off" (id None).
fn track_choices(
    tracks: &[Track],
    with_off: bool,
    checked: impl Fn(&Track) -> bool,
) -> Vec<(Option<i64>, SharedString, bool)> {
    let mut choices = Vec::new();
    if with_off {
        choices.push((None, "Off".into(), !tracks.iter().any(&checked)));
    }
    choices.extend(
        tracks
            .iter()
            .map(|t| (Some(t.id), t.label.clone().into(), checked(t))),
    );
    choices
}

impl PlayerView {
    /// Top bar: back, title, media tags, wall clock.
    pub(super) fn top_chrome(&self, cx: &mut Context<Self>) -> AnyElement {
        let mono = font::mono(cx);

        top_bar()
            .child(
                round_icon_button("player-back", "icons/caret-left.svg", "Back")
                    .on_click(cx.listener(|this, _, window, cx| this.close(window, cx))),
            )
            .child(video_title(
                self.playback.title.clone(),
                self.playback.subtitle.clone(),
                mono.clone(),
            ))
            .child(
                h_flex()
                    .flex_none()
                    .gap(gap::item())
                    .children(
                        self.playback
                            .item
                            .iter()
                            .flat_map(|item| item.media_tags())
                            .map(|tag| video_tag(tag, mono.clone())),
                    )
                    .child(
                        video_meta(clock(0.), mono.clone(), overlay::TEXT_MUTED)
                            .text_size(text::body()),
                    ),
            )
            .into_any_element()
    }

    /// Bottom bar: timeline with scrub preview at `time`, transport, menus.
    pub(super) fn bottom_chrome(
        &self,
        time: f64,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mono = font::mono(cx);
        let scrub_preview = self
            .playback
            .scrubbing
            .and_then(|seconds| self.trickplay_preview(seconds))
            .map(|preview| ((time / self.playback.duration.max(1.)) as f32, preview));

        let timeline = timeline(
            format_time(time),
            Slider::new(&self.seek)
                .accessibility_label("Seek")
                .bg(color::primary(cx))
                .text_color(overlay::WHITE)
                .disabled(self.playback.duration <= 0.),
            (self.playback.buffered_until / self.playback.duration.max(1.)) as f32,
            scrub_preview,
            format_time(self.playback.duration),
            mono.clone(),
        );

        let volume_icon = if self.muted || self.volume <= 0. {
            "icons/mute.svg"
        } else {
            "icons/volume.svg"
        };
        let ends_at = (self.playback.duration > 0.).then(|| {
            video_meta(
                format!(
                    "ends at {}",
                    clock((self.playback.duration - time) / self.speed)
                ),
                mono.clone(),
                overlay::TEXT_FAINT,
            )
            .ml(gap::item())
            .text_size(text::meta())
        });
        let is_episode = self
            .playback
            .item
            .as_ref()
            .is_some_and(|item| item.series_id.is_some());
        let play_label = if self.paused { "Play" } else { "Pause" };
        let mute_label = if self.muted { "Unmute" } else { "Mute" };
        let fullscreen_label = if window.is_fullscreen() {
            "Exit Full Screen"
        } else {
            "Full Screen"
        };
        let controls = h_flex()
            .mt(gap::item())
            .gap(gap::tight())
            .child(
                round_icon_button(
                    "player-play",
                    if self.paused {
                        "icons/play.svg"
                    } else {
                        "icons/pause.svg"
                    },
                    play_label,
                )
                .tooltip_with_action(play_label, &TogglePause, Some(CONTEXT))
                .on_click(
                    cx.listener(|this, _, window, cx| this.toggle_pause(&TogglePause, window, cx)),
                ),
            )
            // Episodes only; disabled at the Series' first/last Episode
            .when(is_episode, |this| {
                this.child(
                    round_icon_button(
                        "player-previous",
                        "icons/backward-step.svg",
                        "Previous Episode",
                    )
                    .disabled(self.playback.previous.is_none())
                    .tooltip_with_action("Previous Episode", &PreviousEpisode, Some(CONTEXT))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.previous_episode(&PreviousEpisode, window, cx)
                    })),
                )
                .child(
                    round_icon_button("player-next", "icons/forward-step.svg", "Next Episode")
                        .disabled(self.playback.next.is_none())
                        .tooltip_with_action("Next Episode", &NextEpisode, Some(CONTEXT))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.next_episode(&NextEpisode, window, cx)
                        })),
                )
            })
            .child(
                // slider unfolds from the mute button on hover
                hover_reveal(
                    "player-volume",
                    round_icon_button("player-mute", volume_icon, mute_label)
                        .tooltip_with_action(mute_label, &ToggleMute, Some(CONTEXT))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.toggle_mute(&ToggleMute, window, cx)
                        })),
                    Slider::new(&self.volume_slider)
                        .accessibility_label("Volume")
                        .text_color(overlay::WHITE),
                    self.volume_hud,
                ),
            )
            .children(ends_at)
            .child(spacer())
            .child(self.speed_menu(cx))
            .child(self.bitrate_menu(cx))
            .children(self.chapter_menu(cx))
            .children(self.track_menu(TrackKind::Audio, cx))
            .children(self.track_menu(TrackKind::Subtitle, cx))
            .child(
                round_icon(
                    "player-info",
                    Icon::new(assets::IconName::Info),
                    "Playback info",
                )
                .selected(self.info.is_some())
                .tooltip_with_action("Playback info", &TogglePlaybackInfo, Some(CONTEXT))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.toggle_playback_info(&TogglePlaybackInfo, window, cx)
                })),
            );
        let controls = if SUPPORTS_PIP {
            controls.child(
                round_icon_button("player-pip", "icons/pip.svg", "Picture in Picture")
                    .tooltip_with_action("Picture in Picture", &TogglePip, Some(CONTEXT))
                    .on_click(
                        cx.listener(|this, _, window, cx| this.toggle_pip(&TogglePip, window, cx)),
                    ),
            )
        } else {
            controls
        };
        let controls = controls.child(
            round_icon_button(
                "player-fullscreen",
                if window.is_fullscreen() {
                    "icons/minimize.svg"
                } else {
                    "icons/maximize.svg"
                },
                fullscreen_label,
            )
            .tooltip_with_action(fullscreen_label, &ToggleFullscreen, Some(CONTEXT))
            .on_click(cx.listener(|this, _, window, cx| {
                this.toggle_fullscreen(&ToggleFullscreen, window, cx)
            })),
        );

        bottom_bar(v_flex().w_full().child(timeline).child(controls)).into_any_element()
    }

    /// Scrub-preview thumbnail for `seconds`, cropped from the server's sprite sheet.
    fn trickplay_preview(&self, seconds: f64) -> Option<AnyElement> {
        let item = self.playback.item.as_ref()?;
        let info = self
            .api
            .trickplay(item)
            .filter(|i| i.width > 0 && i.height > 0)?;
        let (tile, x, y) = info.thumbnail_at(seconds);
        let tile_w = f32::from(design::size::trickplay_w());
        let scale = tile_w / info.width as f32;
        let tile_h = info.height as f32 * scale;
        let sprite_w = tile_w * info.tile_width as f32;
        let sprite_h = tile_h * info.tile_height as f32;
        let url = self.api.trickplay_tile_url(item, &info, tile);
        Some(
            sprite_thumb(
                url,
                size(tile_w, tile_h),
                size(sprite_w, sprite_h),
                point(x as f32 * scale, y as f32 * scale),
            )
            .into_any_element(),
        )
    }

    fn track_menu(&self, kind: TrackKind, cx: &mut Context<Self>) -> Option<AnyElement> {
        let tracks: Vec<Track> = self
            .playback
            .tracks
            .iter()
            .filter(|t| t.kind == kind)
            .cloned()
            .collect();
        if tracks.is_empty() {
            return None;
        }
        let (id, icon, tooltip) = match kind {
            TrackKind::Audio => ("player-audio", "icons/headphones.svg", "Audio"),
            TrackKind::Subtitle if tracks.iter().any(|t| t.selected || t.secondary) => {
                ("player-subtitles", "icons/cc-filled.svg", "Subtitles")
            }
            TrackKind::Subtitle => ("player-subtitles", "icons/cc.svg", "Subtitles"),
        };
        let this = cx.entity().downgrade();
        // subtitles lead with "Off" (id None)
        let choices = track_choices(&tracks, kind == TrackKind::Subtitle, |t| t.selected);
        let options = choices.iter().map(|(_, l, c)| (l.clone(), *c)).collect();
        if kind == TrackKind::Audio {
            return Some(choice_menu(
                round_icon_button(id, icon, tooltip),
                options,
                // long track lists overflow the window otherwise
                true,
                move |ix, cx| {
                    let track_id = choices[ix].0;
                    this.update(cx, |this, _| this.select_track(kind, track_id))
                        .ok();
                },
                self.on_menu_open(cx),
            ));
        }
        // submenu: same tracks, mpv's `secondary-sid`
        let secondary_choices = track_choices(&tracks, true, |t| t.secondary);
        let secondary_options = secondary_choices
            .iter()
            .map(|(_, l, c)| (l.clone(), *c))
            .collect();
        Some(choice_menu_with_submenu(
            round_icon_button(id, icon, tooltip),
            options,
            ("Secondary subtitles".into(), secondary_options),
            true,
            move |list, ix, cx| {
                this.update(cx, |this, _| {
                    if list == 0 {
                        this.select_track(kind, choices[ix].0)
                    } else {
                        this.select_secondary_subtitle(secondary_choices[ix].0)
                    }
                })
                .ok();
            },
            self.on_menu_open(cx),
        ))
    }

    /// Jellyfin's own chapter markers, not mpv's embedded ones (see `chapter_index`).
    fn chapter_menu(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let chapters = self.playback.item.as_ref()?.chapters.clone();
        if chapters.is_empty() {
            return None;
        }
        let current = self.chapter_index();
        let this = cx.entity().downgrade();
        let options = chapters
            .iter()
            .enumerate()
            .map(|(i, chapter)| {
                let label = chapter
                    .name
                    .clone()
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| format!("Chapter {}", i + 1));
                (label.into(), current == Some(i))
            })
            .collect();
        Some(choice_menu(
            round_icon_button("player-chapters", "icons/chapters.svg", "Chapters"),
            options,
            true,
            move |ix, cx| {
                let seconds = chapters[ix].start_seconds();
                this.update(cx, |this, _| {
                    if let Some(mpv) = this.active_mpv() {
                        let _ = mpv.seek(seconds);
                    }
                })
                .ok();
            },
            self.on_menu_open(cx),
        ))
    }

    fn speed_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        let speed = self.speed;
        choice_menu(
            round_ghost_button("player-speed", format!("{speed}\u{d7}"), cx).tooltip("Speed"),
            SPEEDS
                .iter()
                .map(|s| (format!("{s}\u{d7}").into(), *s == speed))
                .collect(),
            false,
            move |ix, cx| {
                this.update(cx, |this, cx| {
                    this.set_speed(SPEEDS[ix]);
                    cx.notify();
                })
                .ok();
            },
            self.on_menu_open(cx),
        )
    }

    /// Streaming cap for this Player; picking one reloads the item from where it is.
    fn bitrate_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        let current = self.max_bitrate_mbps;
        let label = BITRATE_CAPS
            .iter()
            .find(|(cap, _)| *cap == current)
            .map_or_else(
                || current.map(|m| format!("{m} Mbps")).unwrap_or_default(),
                |(_, label)| label.to_string(),
            );
        choice_menu(
            round_icon_button(
                "player-bitrate",
                "icons/speedometer.svg",
                format!("Quality: {label}"),
            ),
            BITRATE_CAPS
                .iter()
                .map(|(cap, label)| (SharedString::from(*label), *cap == current))
                .collect(),
            false,
            move |ix, cx| {
                let cap = BITRATE_CAPS[ix].0;
                this.update(cx, |this, cx| this.set_max_bitrate(cap, cx))
                    .ok();
            },
            self.on_menu_open(cx),
        )
    }

    /// Keeps controls up while a menu is open; refocuses the Player on close.
    fn on_menu_open(
        &self,
        cx: &mut Context<Self>,
    ) -> impl Fn(&bool, &mut Window, &mut App) + 'static {
        cx.listener(|this, open: &bool, window, cx| {
            this.menu_open = *open;
            if !*open {
                this.refocus(window, cx);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::track_choices;
    use crate::mpv::{Track, TrackKind};

    fn sub(id: i64, selected: bool, secondary: bool) -> Track {
        Track {
            id,
            kind: TrackKind::Subtitle,
            label: format!("Sub {id}"),
            selected,
            secondary,
            lang: None,
        }
    }

    fn rows(tracks: &[Track], checked: impl Fn(&Track) -> bool) -> Vec<(Option<i64>, bool)> {
        let choices = track_choices(tracks, true, checked);
        choices.into_iter().map(|(id, _, c)| (id, c)).collect()
    }

    #[test]
    fn primary_and_secondary_lists_each_check_their_own_track() {
        let tracks = [sub(1, true, false), sub(2, false, true)];
        // Off, then each track once
        let primary = [(None, false), (Some(1), true), (Some(2), false)];
        let secondary = [(None, false), (Some(1), false), (Some(2), true)];
        assert_eq!(rows(&tracks, |t| t.selected), primary);
        assert_eq!(rows(&tracks, |t| t.secondary), secondary);
    }

    #[test]
    fn off_is_checked_when_nothing_is_on() {
        // only a secondary subtitle: primary Off is checked, secondary's track is
        let tracks = [sub(1, false, true)];
        let primary = [(None, true), (Some(1), false)];
        let secondary = [(None, false), (Some(1), true)];
        assert_eq!(rows(&tracks, |t| t.selected), primary);
        assert_eq!(rows(&tracks, |t| t.secondary), secondary);
    }
}
