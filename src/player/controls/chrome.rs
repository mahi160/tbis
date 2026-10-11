//! Player chrome: the top bar (back, title, tags, clock) and the bottom bar
//! (timeline, transport, menus), shown together while the controls are up.

use gpui_kit::component::slider::Slider;
use gpui_kit::component::{Disableable as _, Icon, Selectable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::components::button::{round_ghost_button, round_icon, round_icon_button};
use crate::components::layout::spacer;
use crate::components::menu::choice_menu;
use crate::components::video::{
    bottom_bar, hover_reveal, sprite_thumb, timeline, top_bar, video_meta, video_tag, video_title,
};
use crate::design::{self, color, font, gap, overlay, text};
use crate::mpv::{Track, TrackKind};

use super::super::clock::{clock, format_time};
use super::super::{
    CONTEXT, NextEpisode, PlayerView, PreviousEpisode, ToggleFullscreen, ToggleMute, TogglePause,
    TogglePip, TogglePlaybackInfo,
};
use super::SPEEDS;

impl PlayerView {
    /// Top bar: back, title, media tags, wall clock.
    pub(super) fn top_chrome(&self, cx: &mut Context<Self>) -> AnyElement {
        let mono = font::mono(cx);

        top_bar()
            .child(
                round_icon_button("player-back", "icons/caret-left.svg")
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
                .bg(color::primary(cx))
                .text_color(overlay::WHITE)
                .disabled(self.playback.duration <= 0. || self.pip.is_some()),
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
                )
                .on_click(
                    cx.listener(|this, _, window, cx| this.toggle_pause(&TogglePause, window, cx)),
                ),
            )
            // Episodes only; disabled at the Series' first/last Episode
            .when(is_episode, |this| {
                this.child(
                    round_icon_button("player-previous", "icons/backward-step.svg")
                        .disabled(self.playback.previous.is_none())
                        .tooltip_with_action("Previous Episode", &PreviousEpisode, Some(CONTEXT))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.previous_episode(&PreviousEpisode, window, cx)
                        })),
                )
                .child(
                    round_icon_button("player-next", "icons/forward-step.svg")
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
                    round_icon_button("player-mute", volume_icon).on_click(
                        cx.listener(|this, _, window, cx| {
                            this.toggle_mute(&ToggleMute, window, cx)
                        }),
                    ),
                    Slider::new(&self.volume_slider).text_color(overlay::WHITE),
                    self.volume_hud,
                ),
            )
            .children(ends_at)
            .child(spacer())
            .child(self.speed_menu(cx))
            .children(self.chapter_menu(cx))
            .children(self.track_menu(TrackKind::Audio, cx))
            .children(self.track_menu(TrackKind::Subtitle, cx))
            .child(
                round_icon("player-info", Icon::new(assets::IconName::Info))
                    .selected(self.info.is_some())
                    .tooltip_with_action("Playback info", &TogglePlaybackInfo, Some(CONTEXT))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.toggle_playback_info(&TogglePlaybackInfo, window, cx)
                    })),
            )
            .child(round_icon_button("player-pip", "icons/pip.svg").on_click(
                cx.listener(|this, _, window, cx| this.toggle_pip(&TogglePip, window, cx)),
            ))
            .child(
                round_icon_button(
                    "player-fullscreen",
                    if window.is_fullscreen() {
                        "icons/minimize.svg"
                    } else {
                        "icons/maximize.svg"
                    },
                )
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
        let (id, icon) = match kind {
            TrackKind::Audio => ("player-audio", "icons/headphones.svg"),
            TrackKind::Subtitle if tracks.iter().any(|t| t.selected) => {
                ("player-subtitles", "icons/cc-filled.svg")
            }
            TrackKind::Subtitle => ("player-subtitles", "icons/cc.svg"),
        };
        let this = cx.entity().downgrade();
        // subtitles lead with "Off" (id None)
        let mut choices: Vec<(Option<_>, SharedString, bool)> = Vec::new();
        if kind == TrackKind::Subtitle {
            choices.push((None, "Off".into(), !tracks.iter().any(|t| t.selected)));
        }
        choices.extend(
            tracks
                .iter()
                .map(|t| (Some(t.id), t.label.clone().into(), t.selected)),
        );
        let options = choices.iter().map(|(_, l, c)| (l.clone(), *c)).collect();
        Some(choice_menu(
            round_icon_button(id, icon),
            options,
            // long track lists overflow the window otherwise
            true,
            move |ix, cx| {
                let track_id = choices[ix].0;
                this.update(cx, |this, _| this.select_track(kind, track_id))
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
            round_icon_button("player-chapters", "icons/chapters.svg"),
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
            round_ghost_button("player-speed", format!("{speed}\u{d7}"), cx),
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
