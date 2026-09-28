//! Bars, menus, the up-next card, and the `Render` impl.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::slider::{Slider, SliderEvent};
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, Selectable as _, Sizable as _, WindowExt as _,
    h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::config::SeekSteps;
use crate::jellyfin::SegmentKind;
use crate::mpv::{Track, TrackKind};
use crate::now_playing::{Info, NowPlaying, RemoteCommand};
use crate::shaders::ShaderProfile;

use super::clock::{clock, format_time};
use super::{
    AudioDelayEarlier, AudioDelayLater, CONTEXT, ChapterNext, ChapterPrev, CycleAudio,
    CycleShaders, CycleSubtitle, DELAY_STEP, Escape, HIDE_CONTROLS_AFTER, NextEpisode, PlayNext,
    PlayerView, PreviousEpisode, Screenshot, SeekBack, SeekBackLong, SeekForward, SeekForwardLong,
    SkipSegment, SpeedDown, SpeedUp, SubDelayEarlier, SubDelayLater, ToggleFullscreen, ToggleMute,
    TogglePause, TogglePip, TogglePlaybackInfo, VOLUME_STEP, VolumeDown, VolumeUp, hide_cursor,
};

const INFO_EVERY: std::time::Duration = std::time::Duration::from_secs(1);
const SPEEDS: [f64; 7] = [0.5, 0.75, 1., 1.25, 1.5, 1.75, 2.];
/// Displayed width of the scrub-preview thumbnail; the sprite sheet scales to fit.
const TRICKPLAY_WIDTH: f32 = 160.;

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
        let steps = cx.global::<SeekSteps>();
        let seconds = f64::from(if long { steps.long } else { steps.short });
        if let Some(mpv) = self.active_mpv() {
            let _ = mpv.seek_by(if forward { seconds } else { -seconds });
        }
        self.show_controls(window, cx);
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
    }

    fn volume_down(&mut self, _: &VolumeDown, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mpv) = self.active_mpv() {
            let _ = mpv.set_volume((self.volume - VOLUME_STEP).max(0.));
        }
        self.show_controls(window, cx);
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

    /// Keeps Now Playing in step with the Player; called every render.
    fn sync_now_playing(&mut self, cx: &App) {
        let wanted =
            self.pip.is_none() && self.playback.item.is_some() && self.playback.error.is_none();
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
        let dir = std::path::Path::new(&std::env::var_os("HOME").unwrap_or_default())
            .join("Pictures")
            .join("tbis");
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

    /// Opens PiP at the current position and pauses; again closes it (resume follows `Ended`).
    fn toggle_pip(&mut self, _: &TogglePip, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(pip) = &mut self.pip {
            pip.stop();
            self.refocus(window, cx);
            return;
        }
        let (Some(mpv), Some(item)) = (&self.mpv, &self.playback.item) else {
            return;
        };
        let selected = |kind: TrackKind| {
            self.playback
                .tracks
                .iter()
                .find(|t| t.kind == kind && t.selected)
                .map(|t| t.id)
        };
        let (events_tx, mut events_rx) = futures::channel::mpsc::unbounded();
        let mut options = self.subtitles.mpv_options().to_vec();
        if let Ok(shaders) = self.shaders.mpv_value() {
            options.push(("glsl-shaders", shaders));
        }
        let start = crate::pip::PipStart {
            url: &item.stream.url,
            auth_header: &self.api.auth_header(),
            start_seconds: self.playback.time,
            speed: self.speed,
            volume: self.volume,
            muted: self.muted,
            audio_track: selected(TrackKind::Audio),
            subtitle_track: selected(TrackKind::Subtitle),
            options: &options,
        };
        match crate::pip::Pip::start(start, events_tx) {
            Ok(pip) => {
                let _ = mpv.set_pause(true);
                self.pip = Some(pip);
                self._pip = cx.spawn_in(window, async move |this, cx| {
                    use futures::StreamExt as _;
                    while let Some(event) = events_rx.next().await {
                        if this
                            .update_in(cx, |this, window, cx| this.on_pip(event, window, cx))
                            .is_err()
                        {
                            break;
                        }
                    }
                });
            }
            // not fatal: in-window mpv keeps playing, only the standalone window failed
            Err(err) => window.push_notification(Notification::error(err), cx),
        }
        self.refocus(window, cx);
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

    /// Scrub-preview thumbnail for `seconds`, cropped from the server's sprite sheet.
    fn trickplay_preview(&self, seconds: f64) -> Option<AnyElement> {
        let item = self.playback.item.as_ref()?;
        let info = self
            .api
            .trickplay(item)
            .filter(|i| i.width > 0 && i.height > 0)?;
        let (tile, x, y) = info.thumbnail_at(seconds);
        let scale = TRICKPLAY_WIDTH / info.width as f32;
        let (tile_w, tile_h) = (TRICKPLAY_WIDTH, info.height as f32 * scale);
        let sprite_w = tile_w * info.tile_width as f32;
        let sprite_h = tile_h * info.tile_height as f32;
        let url = self.api.trickplay_tile_url(item, &info, tile);
        Some(
            div()
                .w(px(tile_w))
                .h(px(tile_h))
                .rounded(px(8.))
                .overflow_hidden()
                .relative()
                .shadow(vec![
                    BoxShadow::new(px(0.), px(4.), video_black(0.5)).blur_radius(px(12.)),
                ])
                .child(
                    img(url)
                        .absolute()
                        .top(px(-(y as f32) * scale))
                        .left(px(-(x as f32) * scale))
                        .w(px(sprite_w))
                        .h(px(sprite_h)),
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
        Some(
            icon_button(id, icon, cx)
                .dropdown_menu(move |menu, _, _| {
                    // long track lists overflow the window otherwise
                    let mut menu = menu.scrollable(true);
                    if kind == TrackKind::Subtitle {
                        let this = this.clone();
                        menu = menu.item(
                            PopupMenuItem::new("Off")
                                .checked(!tracks.iter().any(|t| t.selected))
                                .on_click(move |_, _, cx| {
                                    this.update(cx, |this, _| this.select_track(kind, None))
                                        .ok();
                                }),
                        );
                    }
                    for track in &tracks {
                        let (this, track_id) = (this.clone(), track.id);
                        menu = menu.item(
                            PopupMenuItem::new(track.label.clone())
                                .checked(track.selected)
                                .on_click(move |_, _, cx| {
                                    this.update(cx, |this, _| {
                                        this.select_track(kind, Some(track_id))
                                    })
                                    .ok();
                                }),
                        );
                    }
                    menu
                })
                .on_open_change(self.on_menu_open(cx))
                .into_any_element(),
        )
    }

    /// Jellyfin's own chapter markers, not mpv's embedded ones (see `chapter_index`).
    fn chapter_menu(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let chapters = self.playback.item.as_ref()?.chapters.clone();
        if chapters.is_empty() {
            return None;
        }
        let current = self.chapter_index();
        let this = cx.entity().downgrade();
        Some(
            icon_button("player-chapters", "icons/chapters.svg", cx)
                .dropdown_menu(move |menu, _, _| {
                    let mut menu = menu.scrollable(true);
                    for (i, chapter) in chapters.iter().enumerate() {
                        let (this, seconds) = (this.clone(), chapter.start_seconds());
                        let label = chapter
                            .name
                            .clone()
                            .filter(|n| !n.is_empty())
                            .unwrap_or_else(|| format!("Chapter {}", i + 1));
                        menu = menu.item(
                            PopupMenuItem::new(label)
                                .checked(current == Some(i))
                                .on_click(move |_, _, cx| {
                                    this.update(cx, |this, _| {
                                        if let Some(mpv) = this.active_mpv() {
                                            let _ = mpv.seek(seconds);
                                        }
                                    })
                                    .ok();
                                }),
                        );
                    }
                    menu
                })
                .on_open_change(self.on_menu_open(cx))
                .into_any_element(),
        )
    }

    fn speed_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity().downgrade();
        let speed = self.speed;
        Button::new("player-speed")
            .ghost()
            .rounded(cx.theme().radius_full())
            .label(format!("{speed}\u{d7}"))
            .font_family(cx.theme().mono_font_family.clone())
            .text_xs()
            .dropdown_menu(move |mut menu, _, _| {
                for s in SPEEDS {
                    let this = this.clone();
                    menu = menu.item(
                        PopupMenuItem::new(format!("{s}\u{d7}"))
                            .checked(s == speed)
                            .on_click(move |_, _, cx| {
                                this.update(cx, |this, cx| {
                                    this.set_speed(s);
                                    cx.notify();
                                })
                                .ok();
                            }),
                    );
                }
                menu
            })
            .on_open_change(self.on_menu_open(cx))
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

/// Round ghost icon button shared by the Player's bars.
fn icon_button(id: &'static str, icon: &'static str, cx: &App) -> Button {
    Button::new(id)
        .ghost()
        .rounded(cx.theme().radius_full())
        .icon(Icon::empty().path(icon))
}

/// One overlay banner centered on the video, shared by the fatal error and the
/// "playing in PiP" notice.
fn overlay_banner(content: impl IntoElement) -> AnyElement {
    let label = div().p_4().rounded_md().bg(overlay_scrim());
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .child(label.child(content))
        .into_any_element()
}

/// Video overlays sit on top of mpv's own layer, so they need their own dark scrim
/// rather than the theme's `overlay` (meant for panels over the app's own background).
fn overlay_scrim() -> Hsla {
    video_black(0.6)
}

/// Black tint at `alpha` over the video, for gradients and scrims that must stay black
/// regardless of theme -- the controls sit over live video, not the app's own background.
fn video_black(alpha: f32) -> Hsla {
    hsla(0., 0., 0., alpha)
}

/// White tint at `alpha` over the video, for text/icons that must stay legible on
/// any frame; same theme-independence reasoning as `video_black`.
fn video_white(alpha: f32) -> Hsla {
    hsla(0., 0., 1., alpha)
}

impl Render for PlayerView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // every pause/PiP/error change notifies, so render is the one sync point
        let playing = self.playback.started
            && self.playback.error.is_none()
            && (!self.paused || self.pip.is_some());
        if playing != self.awake.is_some() {
            self.awake = playing.then(super::Awake::new);
        }
        self.sync_now_playing(cx);
        if let Some(mpv) = &self.mpv {
            let size = window.viewport_size();
            let scale = window.scale_factor();
            mpv.set_size(
                (size.width.as_f32() * scale).round() as i32,
                (size.height.as_f32() * scale).round() as i32,
            );
        }

        let clear = video_black(0.);
        let time = self.playback.scrubbing.unwrap_or(self.playback.time);

        let mono = cx.theme().mono_font_family.clone();
        let dim = video_white;

        let top = h_flex()
            .pt(px(36.)) // below traffic lights
            .px_6()
            .pb_5()
            .gap_3()
            .bg(linear_gradient(
                180.,
                linear_color_stop(video_black(0.65), 0.),
                linear_color_stop(clear, 1.),
            ))
            .child(
                icon_button("player-back", "icons/caret-left.svg", cx)
                    .on_click(cx.listener(|this, _, window, cx| this.close(window, cx))),
            )
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .text_size(px(17.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .truncate()
                            .child(self.playback.title.clone()),
                    )
                    .children(self.playback.subtitle.clone().map(|subtitle| {
                        div()
                            .font_family(mono.clone())
                            .text_xs()
                            .text_color(dim(0.55))
                            .truncate()
                            .child(subtitle)
                    })),
            )
            .child(
                h_flex()
                    .flex_none()
                    .gap_3()
                    .children(
                        self.playback
                            .item
                            .iter()
                            .flat_map(|item| item.media_tags())
                            .map(|tag| {
                                div()
                                    .px_2()
                                    .rounded_full()
                                    .border_1()
                                    .border_color(dim(0.35))
                                    .font_family(mono.clone())
                                    .text_xs()
                                    .text_color(dim(0.75))
                                    .child(tag)
                            }),
                    )
                    .child(
                        div()
                            .font_family(mono.clone())
                            .text_sm()
                            .text_color(dim(0.75))
                            .child(clock(0.)),
                    ),
            );

        let scrub_preview = self
            .playback
            .scrubbing
            .and_then(|seconds| self.trickplay_preview(seconds))
            .map(|preview| {
                let fraction = (time / self.playback.duration.max(1.)) as f32;
                div()
                    .absolute()
                    .bottom(px(28.))
                    .left(relative(fraction))
                    .ml(px(-TRICKPLAY_WIDTH / 2.))
                    .child(preview)
            });

        let timeline = h_flex()
            .gap_3()
            .font_family(mono.clone())
            .text_sm()
            .text_color(dim(0.85))
            .child(format_time(time))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .child(
                        Slider::new(&self.seek)
                            .bg(cx.theme().primary)
                            .text_color(white())
                            .disabled(self.playback.duration <= 0. || self.pip.is_some()),
                    )
                    .children(scrub_preview),
            )
            .child(format_time(self.playback.duration));

        let volume_icon = if self.muted || self.volume <= 0. {
            "icons/mute.svg"
        } else {
            "icons/volume.svg"
        };
        let ends_at = (self.playback.duration > 0.).then(|| {
            div()
                .ml_2p5()
                .font_family(mono.clone())
                .text_size(px(13.))
                .text_color(dim(0.5))
                .whitespace_nowrap()
                .child(format!(
                    "ends at {}",
                    clock((self.playback.duration - time) / self.speed)
                ))
        });
        let is_episode = self
            .playback
            .item
            .as_ref()
            .is_some_and(|item| item.series_id.is_some());
        let controls = h_flex()
            .mt(px(14.))
            .gap_0p5()
            .child(
                icon_button(
                    "player-play",
                    if self.paused {
                        "icons/play.svg"
                    } else {
                        "icons/pause.svg"
                    },
                    cx,
                )
                .on_click(
                    cx.listener(|this, _, window, cx| this.toggle_pause(&TogglePause, window, cx)),
                ),
            )
            // Episodes only; disabled at the Series' first/last Episode
            .when(is_episode, |this| {
                this.child(
                    icon_button("player-previous", "icons/backward-step.svg", cx)
                        .disabled(self.playback.previous.is_none())
                        .tooltip_with_action("Previous Episode", &PreviousEpisode, Some(CONTEXT))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.previous_episode(&PreviousEpisode, window, cx)
                        })),
                )
                .child(
                    icon_button("player-next", "icons/forward-step.svg", cx)
                        .disabled(self.playback.next.is_none())
                        .tooltip_with_action("Next Episode", &NextEpisode, Some(CONTEXT))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.next_episode(&NextEpisode, window, cx)
                        })),
                )
            })
            .child(
                // slider unfolds from the mute button on hover
                h_flex()
                    .group("player-volume")
                    .child(icon_button("player-mute", volume_icon, cx).on_click(
                        cx.listener(|this, _, window, cx| {
                            this.toggle_mute(&ToggleMute, window, cx)
                        }),
                    ))
                    .child(
                        div()
                            .w_0()
                            .overflow_hidden()
                            .group_hover("player-volume", |s| s.w(px(96.)))
                            .child(
                                div()
                                    .w(px(96.))
                                    .px_2()
                                    .child(Slider::new(&self.volume_slider).text_color(white())),
                            ),
                    ),
            )
            .children(ends_at)
            .child(div().flex_1())
            .child(self.speed_menu(cx))
            .children(self.chapter_menu(cx))
            .children(self.track_menu(TrackKind::Audio, cx))
            .children(self.track_menu(TrackKind::Subtitle, cx))
            .child(
                Button::new("player-info")
                    .ghost()
                    .rounded(cx.theme().radius_full())
                    .icon(Icon::new(assets::IconName::Info))
                    .selected(self.info.is_some())
                    .tooltip_with_action("Playback info", &TogglePlaybackInfo, Some(CONTEXT))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.toggle_playback_info(&TogglePlaybackInfo, window, cx)
                    })),
            )
            .child(icon_button("player-pip", "icons/pip.svg", cx).on_click(
                cx.listener(|this, _, window, cx| this.toggle_pip(&TogglePip, window, cx)),
            ))
            .child(
                icon_button(
                    "player-fullscreen",
                    if window.is_fullscreen() {
                        "icons/minimize.svg"
                    } else {
                        "icons/maximize.svg"
                    },
                    cx,
                )
                .on_click(cx.listener(|this, _, window, cx| {
                    this.toggle_fullscreen(&ToggleFullscreen, window, cx)
                })),
            );

        let bottom = div()
            .pt(px(56.))
            .px_6()
            .pb_5()
            .bg(linear_gradient(
                0.,
                linear_color_stop(video_black(0.78), 0.),
                linear_color_stop(clear, 1.),
            ))
            .child(
                v_flex()
                    .w_full()
                    .max_w(px(1088.))
                    .mx_auto()
                    .child(timeline)
                    .child(controls),
            );

        let error = self.playback.error.clone().map(|error| {
            let back = Button::new("error-back")
                .label("Go back")
                .on_click(cx.listener(|this, _, window, cx| this.close(window, cx)))
                .into_any_element();
            overlay_banner(crate::status::error_panel("Can't play this", error, [back], cx).p_4())
        });

        let pip_notice = self
            .pip
            .is_some()
            .then(|| overlay_banner("Playing in Picture-in-Picture \u{b7} press P to return"));
        // up-next card's Play now already covers skipping credits
        let skip = self
            .active_segment()
            .filter(|s| !(s.kind == SegmentKind::Outro && self.up_next().is_some()))
            .map(|segment| {
                let label = match segment.kind {
                    SegmentKind::Intro => "Skip Intro",
                    SegmentKind::Outro => "Skip Credits",
                };
                div()
                    .absolute()
                    .right_6()
                    .bottom(px(112.)) // above timeline, where up-next sits
                    .child(
                        Button::new("skip-segment")
                            .large()
                            .label(label)
                            .tooltip_with_action(label, &SkipSegment, Some(CONTEXT))
                            .on_click(
                                cx.listener(|this, _, window, cx| this.skip_segment(window, cx)),
                            ),
                    )
            });
        let info = self.info.as_ref().map(|groups| {
            v_flex()
                .absolute()
                .top(px(96.)) // below the title bar
                .left_6()
                .w(px(280.))
                .p_4()
                .gap_4()
                .rounded(cx.theme().radius_lg)
                .bg(video_black(0.72))
                .border_1()
                .border_color(video_white(0.08))
                .text_xs()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::BOLD)
                        .child("Playback info"),
                )
                .children(groups.iter().map(|(heading, rows)| {
                    v_flex()
                        .gap_1p5()
                        .child(
                            div()
                                .pb_1()
                                .border_b_1()
                                .border_color(video_white(0.1))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(cx.theme().primary)
                                .child(*heading),
                        )
                        .children(rows.iter().map(|(label, value)| {
                            h_flex()
                                .gap_3()
                                .justify_between()
                                .child(div().flex_none().text_color(video_white(0.6)).child(*label))
                                .child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(value.clone()),
                                )
                        }))
                }))
        });
        let up_next = self.up_next().map(|next| {
            let left = (self.playback.duration - self.playback.time).max(0.).ceil() as u64;
            v_flex()
                .absolute()
                .right_6()
                .bottom(px(112.)) // above timeline
                .w(px(320.))
                .p_4()
                .gap_3()
                .rounded(cx.theme().radius_lg)
                .bg(video_black(0.8))
                .border_1()
                .border_color(video_white(0.08))
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .truncate()
                        .child(format!("Next: {}", next.episode_label())),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("Playing in {left}s")),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            Button::new("up-next-play")
                                .primary()
                                .label("Play now")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.play_next_now(&PlayNext, window, cx)
                                })),
                        )
                        .child(
                            Button::new("up-next-cancel")
                                .ghost()
                                .label("Cancel")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.playback.next_cancelled = true;
                                    this.refocus(window, cx);
                                })),
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
            .text_color(white())
            .on_mouse_move(cx.listener(|this, _, window, cx| {
                if !this.controls_visible {
                    this.show_controls(window, cx);
                }
            }))
            .children(error)
            .children(pip_notice)
            .children(up_next)
            .children(skip)
            // info hides with the controls, like the rest of the chrome
            .when(
                self.controls_visible || self.playback.error.is_some(),
                |this| this.children(info).child(top).child(bottom),
            )
    }
}
