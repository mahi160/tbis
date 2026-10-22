//! Player actions, overlays (error, PiP notice, info, up-next, skip), and the `Render`
//! impl; the top and bottom bars live in [`chrome`].

use gpui_kit::component::notification::Notification;
use gpui_kit::component::slider::SliderEvent;
use gpui_kit::component::{Icon, WindowExt as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::components::button::{VegaSize as _, button, ghost_button, primary_button, round_icon};
use crate::components::status::{Tone, error_panel};
use crate::components::typography::{caption, strong};
use crate::components::video::{
    info_panel, overlay_banner, overlay_card, overlay_corner, overlay_spinner,
};
use crate::config::SeekSteps;
use crate::design::{self, gap, overlay};
use crate::jellyfin::SegmentKind;
use crate::mpv::TrackKind;
use crate::now_playing::{Info, NowPlaying, RemoteCommand};
use crate::shaders::ShaderProfile;

use super::clock::format_time;
mod chrome;

use super::{
    AudioDelayEarlier, AudioDelayLater, CONTEXT, ChapterNext, ChapterPrev, CycleAudio,
    CycleShaders, CycleSubtitle, DELAY_STEP, Escape, HIDE_CONTROLS_AFTER, NextEpisode, PlayNext,
    PlayerView, PreviousEpisode, Screenshot, SeekBack, SeekBackLong, SeekForward, SeekForwardLong,
    SkipSegment, SpeedDown, SpeedUp, SubDelayEarlier, SubDelayLater, ToggleFullscreen, ToggleMute,
    TogglePause, TogglePip, TogglePlaybackInfo, VOLUME_STEP, VolumeDown, VolumeUp, hide_cursor,
};

const INFO_EVERY: std::time::Duration = std::time::Duration::from_secs(1);
const SPEEDS: [f64; 7] = [0.5, 0.75, 1., 1.25, 1.5, 1.75, 2.];

impl PlayerView {
    pub(super) fn sync_seek_bar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.playback.scrubbing.is_none() && self.playback.duration > 0. {
            let fraction = (self.playback.time / self.playback.duration) as f32;
            self.seek
                .update(cx, |s, cx| s.set_value(fraction, window, cx));
        }
    }

    pub(super) fn on_seek(
        &mut self,
        event: &SliderEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            SliderEvent::Change(value) => {
                self.playback.scrubbing = Some(value.end() as f64 * self.playback.duration)
            }
            SliderEvent::Release(value) => {
                self.playback.scrubbing = None;
                if let Some(mpv) = &self.mpv {
                    let _ = mpv.seek(value.end() as f64 * self.playback.duration);
                }
            }
        }
        self.show_controls(window, cx);
    }

    /// Controls keep focus on the Player so Space etc. never re-trigger a clicked button.
    pub(super) fn refocus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus.focus(window, cx);
        self.show_controls(window, cx);
    }

    fn toggle_pause(&mut self, _: &TogglePause, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mpv) = self.active_mpv() {
            let _ = mpv.set_pause(!self.paused);
        }
        self.refocus(window, cx);
    }

    /// Relative seek by the configured step; `long` picks the Option+arrow one.
    fn seek_step(
        &mut self,
        forward: bool,
        long: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.seek_by_step(forward, long, cx);
        self.show_controls(window, cx);
    }

    /// `seek_step` without the Player window, for PiP's keys.
    pub(super) fn seek_by_step(&self, forward: bool, long: bool, cx: &App) {
        let steps = cx.global::<SeekSteps>();
        let seconds = f64::from(if long { steps.long } else { steps.short });
        if let Some(mpv) = self.active_mpv() {
            let _ = mpv.seek_by(if forward { seconds } else { -seconds });
        }
    }

    fn seek_back(&mut self, _: &SeekBack, window: &mut Window, cx: &mut Context<Self>) {
        self.seek_step(false, false, window, cx);
    }

    fn seek_forward(&mut self, _: &SeekForward, window: &mut Window, cx: &mut Context<Self>) {
        self.seek_step(true, false, window, cx);
    }

    fn seek_back_long(&mut self, _: &SeekBackLong, window: &mut Window, cx: &mut Context<Self>) {
        self.seek_step(false, true, window, cx);
    }

    fn seek_forward_long(
        &mut self,
        _: &SeekForwardLong,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.seek_step(true, true, window, cx);
    }

    fn toggle_mute(&mut self, _: &ToggleMute, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mpv) = self.active_mpv() {
            let _ = mpv.set_mute(!self.muted);
        }
        self.refocus(window, cx);
    }

    /// Actual `self.volume` update comes back through `MpvEvent::Volume`, same as the
    /// volume slider's own drag handler.
    fn volume_up(&mut self, _: &VolumeUp, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mpv) = self.active_mpv() {
            let _ = mpv.set_volume((self.volume + VOLUME_STEP).min(100.));
        }
        self.show_controls(window, cx);
        self.show_volume_hud(window, cx);
    }

    fn volume_down(&mut self, _: &VolumeDown, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mpv) = self.active_mpv() {
            let _ = mpv.set_volume((self.volume - VOLUME_STEP).max(0.));
        }
        self.show_controls(window, cx);
        self.show_volume_hud(window, cx);
    }

    fn cycle_audio(&mut self, _: &CycleAudio, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_track(TrackKind::Audio);
        self.refocus(window, cx);
    }

    fn cycle_subtitle(&mut self, _: &CycleSubtitle, window: &mut Window, cx: &mut Context<Self>) {
        self.cycle_track(TrackKind::Subtitle);
        self.refocus(window, cx);
    }

    fn chapter_prev(&mut self, _: &ChapterPrev, window: &mut Window, cx: &mut Context<Self>) {
        self.seek_chapter(-1);
        self.show_controls(window, cx);
    }

    fn chapter_next(&mut self, _: &ChapterNext, window: &mut Window, cx: &mut Context<Self>) {
        self.seek_chapter(1);
        self.show_controls(window, cx);
    }

    fn sub_delay_later(&mut self, _: &SubDelayLater, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mpv) = self.active_mpv() {
            let _ = mpv.adjust_sub_delay(DELAY_STEP);
        }
        self.show_controls(window, cx);
    }

    fn sub_delay_earlier(
        &mut self,
        _: &SubDelayEarlier,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(mpv) = self.active_mpv() {
            let _ = mpv.adjust_sub_delay(-DELAY_STEP);
        }
        self.show_controls(window, cx);
    }

    fn audio_delay_later(
        &mut self,
        _: &AudioDelayLater,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(mpv) = self.active_mpv() {
            let _ = mpv.adjust_audio_delay(DELAY_STEP);
        }
        self.show_controls(window, cx);
    }

    fn audio_delay_earlier(
        &mut self,
        _: &AudioDelayEarlier,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(mpv) = self.active_mpv() {
            let _ = mpv.adjust_audio_delay(-DELAY_STEP);
        }
        self.show_controls(window, cx);
    }

    fn speed_up(&mut self, _: &SpeedUp, window: &mut Window, cx: &mut Context<Self>) {
        self.step_speed(1, window, cx);
    }

    fn speed_down(&mut self, _: &SpeedDown, window: &mut Window, cx: &mut Context<Self>) {
        self.step_speed(-1, window, cx);
    }

    /// `SPEEDS[2]` is `1.` -- the fallback if `self.speed` somehow isn't in the list.
    fn step_speed(&mut self, direction: i32, window: &mut Window, cx: &mut Context<Self>) {
        let current = SPEEDS.iter().position(|&s| s == self.speed).unwrap_or(2);
        let next = (current as i32 + direction).clamp(0, SPEEDS.len() as i32 - 1) as usize;
        self.set_speed(SPEEDS[next]);
        self.show_controls(window, cx);
    }

    fn toggle_fullscreen(
        &mut self,
        _: &ToggleFullscreen,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.toggle_fullscreen();
        self.refocus(window, cx);
    }

    /// Cancels Autoplay while its card shows, else leaves fullscreen.
    fn escape(&mut self, _: &Escape, window: &mut Window, cx: &mut Context<Self>) {
        if self.up_next().is_some() {
            self.playback.next_cancelled = true;
        } else if window.is_fullscreen() {
            window.toggle_fullscreen();
        }
        self.show_controls(window, cx);
    }

    pub(super) fn on_remote(
        &mut self,
        command: RemoteCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match command {
            RemoteCommand::Play | RemoteCommand::Pause => {
                if let Some(mpv) = self.active_mpv() {
                    let _ = mpv.set_pause(matches!(command, RemoteCommand::Pause));
                }
            }
            RemoteCommand::Toggle => self.toggle_pause(&TogglePause, window, cx),
            RemoteCommand::SkipForward => self.seek_forward(&SeekForward, window, cx),
            RemoteCommand::SkipBackward => self.seek_back(&SeekBack, window, cx),
            RemoteCommand::Seek(seconds) => {
                if let Some(mpv) = self.active_mpv() {
                    let _ = mpv.seek(seconds);
                }
            }
            RemoteCommand::Previous => self.previous_episode(&PreviousEpisode, window, cx),
            RemoteCommand::Next => self.next_episode(&NextEpisode, window, cx),
        }
    }

    /// Keeps Now Playing in step with the Player; called every render and time update.
    pub(super) fn sync_now_playing(&mut self, cx: &App) {
        let wanted = self.playback.item.is_some() && self.playback.error.is_none();
        if wanted != self.now_playing.is_some() {
            self.now_playing = wanted.then(|| {
                let step = f64::from(cx.global::<SeekSteps>().short);
                let now_playing = NowPlaying::new(self.remote.clone(), step);
                if let Some(bytes) = &self.playback.artwork {
                    now_playing.set_artwork(bytes);
                }
                now_playing
            });
        }
        let Some(now_playing) = &self.now_playing else {
            return;
        };
        now_playing.set_episode_nav(
            self.playback.previous.is_some(),
            self.playback.next.is_some(),
        );
        // Episode: "S01E06 · Name" over the Series title; Movie: its name alone
        let (title, subtitle) = match &self.playback.subtitle {
            Some(episode) => (episode.to_string(), Some(self.playback.title.to_string())),
            None => (self.playback.title.to_string(), None),
        };
        now_playing.update(&Info {
            title,
            subtitle,
            duration: self.playback.duration,
            elapsed: self.playback.time,
            rate: if self.paused { 0. } else { self.speed },
        });
    }

    /// Switches to the Episode before this one; this one is reported, not marked played.
    fn previous_episode(
        &mut self,
        _: &PreviousEpisode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(previous) = self.playback.previous.clone() {
            self.play_next(previous, window, cx);
        }
    }

    /// Like the skip button: switches without counting this Episode watched.
    fn next_episode(&mut self, _: &NextEpisode, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(next) = self.playback.next.clone() {
            self.play_next(next, window, cx);
        }
    }

    fn cycle_shaders(&mut self, _: &CycleShaders, window: &mut Window, cx: &mut Context<Self>) {
        self.apply_shaders(self.shaders.next(), true, window, cx);
    }

    /// Switches the in-window mpv to `profile`; on failure falls back to no shaders
    /// with an error toast, playback unaffected. `announce` toasts the new profile.
    pub(super) fn apply_shaders(
        &mut self,
        profile: ShaderProfile,
        announce: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(mpv) = &self.mpv else {
            return;
        };
        let result = profile
            .mpv_value()
            .and_then(|value| mpv.set_property("glsl-shaders", &value));
        let note = match result {
            Ok(()) => {
                self.shaders = profile;
                announce.then(|| Notification::info(format!("Upscaling: {}", profile.label())))
            }
            Err(err) => {
                let _ = mpv.set_property("glsl-shaders", "");
                self.shaders = ShaderProfile::Off;
                Some(Notification::error(format!("Upscaling unavailable: {err}")))
            }
        };
        if let Some(note) = note {
            window.push_notification(note, cx);
        }
    }

    /// Saves the current frame to `~/Pictures/tbis`, named after the item and position.
    fn screenshot(&mut self, _: &Screenshot, window: &mut Window, cx: &mut Context<Self>) {
        let Some(mpv) = self.active_mpv() else {
            return;
        };
        let name = match &self.playback.subtitle {
            Some(episode) => format!("{} {episode}", self.playback.title),
            None => self.playback.title.to_string(),
        };
        // position as h-mm-ss: `:` and `/` are not filename-safe on macOS
        let at = format_time(self.playback.time).replace(':', "-");
        let file = format!("{name} {at}.png").replace(['/', ':'], "-");
        let dir = dirs::picture_dir().unwrap_or_default().join("tbis");
        let result = std::fs::create_dir_all(&dir)
            .map_err(|err| err.to_string())
            .and_then(|()| mpv.screenshot(&dir.join(&file).to_string_lossy()));
        let note = match result {
            Ok(()) => Notification::success(format!("Screenshot saved to Pictures/tbis/{file}")),
            Err(err) => Notification::error(format!("Screenshot failed: {err}")),
        };
        window.push_notification(note, cx);
    }

    fn toggle_playback_info(
        &mut self,
        _: &TogglePlaybackInfo,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.info.take().is_some() {
            self._info = Task::ready(()); // stops polling
        } else {
            self.info = Some(self.playback_info());
            self._info = cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor().timer(INFO_EVERY).await;
                    let Ok(()) = this.update(cx, |this, cx| {
                        this.info = Some(this.playback_info());
                        cx.notify();
                    }) else {
                        break;
                    };
                }
            });
        }
        self.refocus(window, cx);
    }

    /// Live stats from mpv, grouped; "—" for whatever it can't report right now.
    fn playback_info(&self) -> Vec<super::InfoGroup> {
        let mpv = self.active_mpv();
        let get = |name: &str| {
            mpv.and_then(|mpv| mpv.property(name))
                .filter(|v| !v.is_empty())
        };
        let num = |name: &str| get(name).and_then(|v| v.parse::<f64>().ok());
        let or_dash = |v: Option<String>| v.unwrap_or_else(|| "\u{2014}".into());
        let resolution = get("video-params/w")
            .zip(get("video-params/h"))
            .map(|(w, h)| format!("{w}\u{d7}{h}"));
        let fps = num("container-fps").map(|fps| format!("{fps:.3} fps"));
        let gamma = get("video-params/gamma");
        let hdr = gamma.as_deref().and_then(|g| match g {
            "pq" => Some("PQ"),
            "hlg" => Some("HLG"),
            _ => None,
        });
        let colour = match hdr {
            Some(kind) => format!("HDR ({kind}) \u{2192} tone-mapped to SDR"),
            None => [get("video-params/primaries"), gamma]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" / "),
        };
        let bitrate = match (num("video-bitrate"), num("audio-bitrate")) {
            (None, None) => None,
            (video, audio) => Some(format!(
                "{:.1} Mb/s",
                (video.unwrap_or(0.) + audio.unwrap_or(0.)) / 1e6
            )),
        };
        let dropped = [num("frame-drop-count"), num("decoder-frame-drop-count")]
            .into_iter()
            .flatten()
            .map(|n| n as u64)
            .reduce(|a, b| a + b)
            .map(|n| n.to_string());
        let method = self.playback.item.as_ref().map(|item| {
            if item.stream.transcode.is_some() {
                "Transcode"
            } else {
                "Direct play"
            }
            .into()
        });
        // "Opus (Opus Interactive Audio Codec)" -> "Opus"
        let codec =
            |name: &str| get(name).map(|c| c.split(" (").next().unwrap_or_default().to_string());
        vec![
            (
                "Stream",
                vec![
                    ("Play method", or_dash(method)),
                    ("Container", or_dash(get("file-format"))),
                    ("Bitrate", or_dash(bitrate)),
                    (
                        "Buffered",
                        or_dash(num("demuxer-cache-duration").map(|s| format!("{s:.0} s ahead"))),
                    ),
                    ("Dropped frames", or_dash(dropped)),
                ],
            ),
            (
                "Video",
                vec![
                    ("Codec", or_dash(codec("video-codec"))),
                    ("Resolution", or_dash(resolution)),
                    ("Frame rate", or_dash(fps)),
                    ("Colour", or_dash((!colour.is_empty()).then_some(colour))),
                    (
                        "Hardware decoding",
                        or_dash(
                            get("hwdec-current").map(|h| if h == "no" { "off".into() } else { h }),
                        ),
                    ),
                ],
            ),
            (
                "Audio",
                vec![
                    ("Codec", or_dash(codec("audio-codec"))),
                    ("Channels", or_dash(get("audio-params/hr-channels"))),
                ],
            ),
        ]
    }

    fn skip_segment_action(
        &mut self,
        _: &SkipSegment,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.skip_segment(window, cx);
    }

    fn play_next_now(&mut self, _: &PlayNext, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(next) = self.up_next().cloned() {
            self.playback.finished = true; // Play now / Enter counts the current Episode watched
            self.play_next(next, window, cx);
        }
    }

    /// Moves the video into the PiP window; again moves it back.
    fn toggle_pip(&mut self, _: &TogglePip, window: &mut Window, cx: &mut Context<Self>) {
        if self.pip.is_some() {
            self.leave_pip(window, cx);
        } else {
            self.enter_pip(window, cx);
        }
    }

    /// Pops the volume slider open (normally hidden behind a hover on the mute button)
    /// so VolumeUp/VolumeDown give visible feedback, then folds it back after a beat.
    fn show_volume_hud(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.volume_hud = true;
        self._volume_hud = cx.spawn_in(window, async move |this, cx| {
            cx.background_executor()
                .timer(super::VOLUME_HUD_AFTER)
                .await;
            this.update(cx, |this, cx| {
                this.volume_hud = false;
                cx.notify();
            })
            .ok();
        });
        cx.notify();
    }

    pub(super) fn show_controls(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.controls_visible = true;
        self._hide = cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(HIDE_CONTROLS_AFTER).await;
            this.update(cx, |this, cx| {
                let busy = this.paused || this.menu_open || this.playback.scrubbing.is_some();
                if !busy && this.playback.error.is_none() {
                    this.controls_visible = false;
                    hide_cursor();
                    cx.notify();
                }
            })
            .ok();
        });
        cx.notify();
    }
}

impl Render for PlayerView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // every pause/PiP/error change notifies, so render is the one sync point
        let playing = self.playback.started && self.playback.error.is_none() && !self.paused;
        if playing != self.awake.is_some() {
            self.awake = playing.then(super::Awake::new);
        }
        self.sync_now_playing(cx);
        // in PiP, its window sizes the video
        if let Some(mpv) = &self.mpv
            && self.pip.is_none()
        {
            super::fit_video(&mpv.video_size(), window);
        }

        let time = self.playback.scrubbing.unwrap_or(self.playback.time);
        let chrome = (self.controls_visible || self.playback.error.is_some())
            .then(|| (self.top_chrome(cx), self.bottom_chrome(time, window, cx)));

        let error = self.playback.error.clone().map(|error| {
            let back = button("error-back", "Go back")
                .on_click(cx.listener(|this, _, window, cx| this.close(window, cx)))
                .into_any_element();
            overlay_banner(
                error_panel("Can't play this", error, [back], Tone::Video, cx).p(gap::group()),
            )
        });

        let pip_notice = self
            .pip
            .is_some()
            .then(|| overlay_banner("Playing in Picture-in-Picture \u{b7} press P to return"));
        let buffering = (self.core_idle
            && !self.paused
            && self.pip.is_none()
            && self.playback.error.is_none())
        .then(overlay_spinner);
        // up-next card's Play now already covers skipping credits
        let skip = self
            .active_segment()
            .filter(|s| !(s.kind == SegmentKind::Outro && self.up_next().is_some()))
            .map(|segment| {
                let label = match segment.kind {
                    SegmentKind::Intro => "Skip Intro",
                    SegmentKind::Outro => "Skip Credits",
                };
                overlay_corner(
                    button("skip-segment", label)
                        .lg()
                        .tooltip_with_action(label, &SkipSegment, Some(CONTEXT))
                        .on_click(cx.listener(|this, _, window, cx| this.skip_segment(window, cx))),
                )
            });
        let info = self.info.as_ref().map(|groups| {
            let close = round_icon("player-info-close", Icon::new(assets::IconName::Close))
                .icon_xs()
                .tooltip("Close")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.toggle_playback_info(&TogglePlaybackInfo, window, cx)
                }));
            info_panel(groups, close)
        });
        let up_next = self.up_next().map(|next| {
            let left = (self.playback.duration - self.playback.time).max(0.).ceil() as u64;
            overlay_corner(
                overlay_card(overlay::CARD)
                    .w(design::size::up_next_w())
                    .gap(gap::item())
                    .child(strong(format!("Next: {}", next.episode_label())))
                    .child(
                        caption(format!("Playing in {left}s"), cx).text_color(overlay::TEXT_MUTED),
                    )
                    .child(
                        h_flex()
                            .gap(gap::item())
                            .child(primary_button("up-next-play", "Play now").on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.play_next_now(&PlayNext, window, cx)
                                }),
                            ))
                            .child(
                                ghost_button("up-next-cancel", "Cancel", cx)
                                    .text_color(overlay::WHITE)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.playback.next_cancelled = true;
                                        this.refocus(window, cx);
                                    })),
                            ),
                    ),
            )
        });

        v_flex()
            .id("player")
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::toggle_pause))
            .on_action(cx.listener(Self::seek_back))
            .on_action(cx.listener(Self::seek_forward))
            .on_action(cx.listener(Self::seek_back_long))
            .on_action(cx.listener(Self::seek_forward_long))
            .on_action(cx.listener(Self::toggle_mute))
            .on_action(cx.listener(Self::toggle_fullscreen))
            .on_action(cx.listener(Self::escape))
            .on_action(cx.listener(Self::play_next_now))
            .on_action(cx.listener(Self::toggle_pip))
            .on_action(cx.listener(Self::volume_up))
            .on_action(cx.listener(Self::volume_down))
            .on_action(cx.listener(Self::cycle_audio))
            .on_action(cx.listener(Self::cycle_subtitle))
            .on_action(cx.listener(Self::speed_up))
            .on_action(cx.listener(Self::speed_down))
            .on_action(cx.listener(Self::chapter_prev))
            .on_action(cx.listener(Self::chapter_next))
            .on_action(cx.listener(Self::sub_delay_later))
            .on_action(cx.listener(Self::sub_delay_earlier))
            .on_action(cx.listener(Self::audio_delay_later))
            .on_action(cx.listener(Self::audio_delay_earlier))
            .on_action(cx.listener(Self::skip_segment_action))
            .on_action(cx.listener(Self::previous_episode))
            .on_action(cx.listener(Self::next_episode))
            .on_action(cx.listener(Self::toggle_playback_info))
            .on_action(cx.listener(Self::screenshot))
            .on_action(cx.listener(Self::cycle_shaders))
            .size_full()
            .relative()
            .justify_between()
            .text_color(overlay::WHITE)
            // video is in PiP; window is see-through, so cover it
            .when(self.pip.is_some(), |this| this.bg(overlay::black(1.)))
            .on_mouse_move(cx.listener(|this, _, window, cx| {
                if !this.controls_visible {
                    this.show_controls(window, cx);
                }
            }))
            .children(error)
            .children(pip_notice)
            .children(buffering)
            // stays up until closed (x or i), independent of the controls
            .children(info)
            .children(up_next)
            .children(skip)
            .when_some(chrome, |this, (top, bottom)| this.child(top).child(bottom))
    }
}
