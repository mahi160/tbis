//! Video Player. Split by concern: this file is lifecycle (open/load/close/quit) and
//! the `PlayerView` type itself; [`playback`] is the current item's state and its
//! server reports; [`controls`] is the bars, menus, up-next card and `Render` impl;
//! [`clock`] is the small pure time-formatting helpers.

mod clock;
mod controls;
mod playback;

use std::future::Future;
use std::time::Duration;

use futures::StreamExt as _;
use futures::channel::mpsc;
use gpui_kit::component::slider::{SliderEvent, SliderState};
use gpui_kit::*;

use crate::config::{Config, LanguagePref, SubtitleStyle, TrackPrefs};
use crate::jellyfin::{Api, Item, PlaybackItem, Report};
use crate::mpv::Mpv;
use crate::now_playing::{NowPlaying, RemoteCommand};
use crate::pip::Pip;
use playback::Playback;

actions!(
    player,
    [
        TogglePause,
        SeekBack,
        SeekForward,
        ToggleFullscreen,
        Escape,
        ToggleMute,
        PlayNext,
        TogglePip,
        VolumeUp,
        VolumeDown,
        CycleAudio,
        CycleSubtitle,
        SpeedUp,
        SpeedDown,
        ChapterPrev,
        ChapterNext,
        SubDelayLater,
        SubDelayEarlier,
        AudioDelayLater,
        AudioDelayEarlier,
        SkipSegment,
        PreviousEpisode,
        NextEpisode,
        TogglePlaybackInfo,
        Screenshot
    ]
);

const CONTEXT: &str = "Player";
const SEEK_STEP: f64 = 10.;
const VOLUME_STEP: f64 = 5.;
const DELAY_STEP: f64 = 0.1;

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("space", TogglePause, Some(CONTEXT)),
        KeyBinding::new("left", SeekBack, Some(CONTEXT)),
        KeyBinding::new("right", SeekForward, Some(CONTEXT)),
        KeyBinding::new("f", ToggleFullscreen, Some(CONTEXT)),
        KeyBinding::new("escape", Escape, Some(CONTEXT)),
        KeyBinding::new("m", ToggleMute, Some(CONTEXT)),
        KeyBinding::new("enter", PlayNext, Some(CONTEXT)),
        KeyBinding::new("p", TogglePip, Some(CONTEXT)),
        KeyBinding::new("up", VolumeUp, Some(CONTEXT)),
        KeyBinding::new("down", VolumeDown, Some(CONTEXT)),
        KeyBinding::new("a", CycleAudio, Some(CONTEXT)),
        KeyBinding::new("c", CycleSubtitle, Some(CONTEXT)),
        KeyBinding::new("shift-.", SpeedUp, Some(CONTEXT)), // '>'
        KeyBinding::new("shift-,", SpeedDown, Some(CONTEXT)), // '<'
        KeyBinding::new("shift-left", ChapterPrev, Some(CONTEXT)),
        KeyBinding::new("shift-right", ChapterNext, Some(CONTEXT)),
        KeyBinding::new("]", SubDelayLater, Some(CONTEXT)),
        KeyBinding::new("[", SubDelayEarlier, Some(CONTEXT)),
        KeyBinding::new("shift-]", AudioDelayLater, Some(CONTEXT)),
        KeyBinding::new("shift-[", AudioDelayEarlier, Some(CONTEXT)),
        KeyBinding::new("s", SkipSegment, Some(CONTEXT)),
        KeyBinding::new("shift-p", PreviousEpisode, Some(CONTEXT)),
        KeyBinding::new("shift-n", NextEpisode, Some(CONTEXT)),
        KeyBinding::new("i", TogglePlaybackInfo, Some(CONTEXT)),
        KeyBinding::new("shift-s", Screenshot, Some(CONTEXT)),
    ]);
}

/// Player left; carries volume and track memory so the app can remember them.
pub struct Closed {
    pub volume: f64,
    pub muted: bool,
    pub track_prefs: TrackPrefs,
}

const HIDE_CONTROLS_AFTER: Duration = Duration::from_secs(3);
const PROGRESS_EVERY: Duration = Duration::from_secs(10);
const CLOCK_EVERY: Duration = Duration::from_secs(30);

pub struct PlayerView {
    api: Api,
    mpv: Option<Mpv>,
    playback: Playback,
    /// Standalone mpv window playing instead of this Player.
    pip: Option<Pip>,
    paused: bool,
    speed: f64,
    volume: f64,
    muted: bool,
    seek: Entity<SliderState>,
    volume_slider: Entity<SliderState>,
    focus: FocusHandle,
    controls_visible: bool,
    menu_open: bool,
    track_prefs: TrackPrefs,
    /// Settings' preferred languages (read-only here).
    language: LanguagePref,
    /// Settings' subtitle look; PiP gets it too.
    subtitles: SubtitleStyle,
    /// Settings' streaming cap; over it the server may transcode.
    max_bitrate_mbps: Option<u32>,
    /// Held while video actually plays (here or in PiP); synced in `render`.
    awake: Option<Awake>,
    /// Media keys + Control Center; absent while PiP's own mpv owns them.
    now_playing: Option<NowPlaying>,
    remote: mpsc::UnboundedSender<RemoteCommand>,
    /// Playback Info overlay rows while open; refreshed every second by `_info`.
    info: Option<Vec<(&'static str, String)>>,
    _info: Task<()>,
    _hide: Task<()>,
    _pip: Task<()>,
    _tasks: Vec<Task<()>>,
    _subscriptions: [Subscription; 2],
}

impl EventEmitter<Closed> for PlayerView {}

impl PlayerView {
    pub fn new(
        api: Api,
        item: &Item,
        config: &Config,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (volume, muted) = (config.volume, config.muted);
        let seek = cx.new(|_| SliderState::new().min(0.).max(1.).step(0.0001));
        let volume_slider = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(100.)
                .step(1.)
                .default_value(volume as f32)
        });
        let _subscriptions = [
            cx.subscribe_in(&seek, window, |this, _, event, window, cx| {
                this.on_seek(event, window, cx)
            }),
            cx.subscribe_in(&volume_slider, window, |this, _, event, window, cx| {
                let (SliderEvent::Change(value) | SliderEvent::Release(value)) = event;
                if let Some(mpv) = this.active_mpv() {
                    let _ = mpv.set_volume(value.end() as f64);
                }
                this.show_controls(window, cx);
            }),
        ];

        let mut playback = Playback::new(item, &api, cx);

        let (events_tx, mut events_rx) = mpsc::unbounded();
        let (mpv, mpv_error) = match Mpv::new(window, &api.auth_header(), events_tx) {
            Ok(mpv) => {
                let _ = mpv.set_volume(volume);
                let _ = mpv.set_mute(muted);
                for (name, value) in config.subtitles.mpv_options() {
                    let _ = mpv.set_property(name, &value);
                }
                (Some(mpv), None)
            }
            Err(err) => (None, Some(format!("Cannot start player: {err}").into())),
        };
        playback.error = mpv_error;

        let mut tasks = Vec::new();
        tasks.push(cx.spawn_in(window, async move |this, cx| {
            while let Some(event) = events_rx.next().await {
                if this
                    .update_in(cx, |this, window, cx| this.on_mpv(event, window, cx))
                    .is_err()
                {
                    break;
                }
            }
        }));
        if mpv.is_some() {
            tasks.push(cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor().timer(PROGRESS_EVERY).await;
                    let Ok(()) = this.update(cx, |this, _| {
                        if !this.paused || this.pip.is_some() {
                            this.report(Report::Progress);
                        }
                    }) else {
                        break;
                    };
                }
            }));
        }

        // clock and "ends at" stay current while paused
        tasks.push(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(CLOCK_EVERY).await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        }));

        let (remote, mut remote_rx) = mpsc::unbounded();
        tasks.push(cx.spawn_in(window, async move |this, cx| {
            while let Some(command) = remote_rx.next().await {
                if this
                    .update_in(cx, |this, window, cx| this.on_remote(command, window, cx))
                    .is_err()
                {
                    break;
                }
            }
        }));

        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let mut this = Self {
            api,
            mpv,
            playback,
            pip: None,
            paused: false,
            speed: 1.,
            volume,
            muted,
            seek,
            volume_slider,
            focus,
            controls_visible: true,
            menu_open: false,
            track_prefs: config.track_prefs.clone(),
            language: config.language.clone(),
            subtitles: config.subtitles.clone(),
            max_bitrate_mbps: config.max_bitrate_mbps,
            awake: None,
            now_playing: None,
            remote,
            info: None,
            _info: Task::ready(()),
            _hide: Task::ready(()),
            _pip: Task::ready(()),
            _tasks: tasks,
            _subscriptions,
        };
        this.load(item.id.clone(), cx);
        this.show_controls(window, cx);
        this
    }

    /// Fetches fresh play details, starts playback, then looks up the next Episode.
    fn load(&mut self, item_id: String, cx: &mut Context<Self>) {
        if self.mpv.is_none() {
            return;
        }
        let api = self.api.clone();
        let artwork_url = self.playback.artwork_url.clone();
        let max_mbps = self.max_bitrate_mbps;
        self.playback._load = cx.spawn(async move |this, cx| {
            let result = api.playback_item(&item_id, max_mbps).await;
            let loaded = result.is_ok();
            let series_id = result.as_ref().ok().and_then(|i| i.series_id.clone());
            if this.update(cx, |this, cx| this.start(result, cx)).is_err() || !loaded {
                return;
            }
            if let Some(url) = artwork_url {
                match api.image_bytes(&url).await {
                    Ok(bytes) => {
                        let Ok(()) = this.update(cx, |this, _| {
                            if let Some(now_playing) = &this.now_playing {
                                now_playing.set_artwork(&bytes);
                            }
                            this.playback.artwork = Some(bytes);
                        }) else {
                            return;
                        };
                    }
                    Err(err) => eprintln!("artwork fetch failed: {err}"),
                }
            }
            match api.segments(&item_id).await {
                Ok(segments) => {
                    let Ok(()) = this.update(cx, |this, cx| {
                        this.playback.segments = segments;
                        cx.notify();
                    }) else {
                        return;
                    };
                }
                // pre-10.10 servers 404 here: just no skip button
                Err(err) => eprintln!("media segments lookup failed: {err}"),
            }
            let Some(series_id) = series_id else {
                return;
            };
            match api.adjacent_episodes(&series_id, &item_id).await {
                Ok((previous, next)) => {
                    this.update(cx, |this, cx| {
                        this.playback.previous = previous;
                        this.playback.next = next;
                        cx.notify();
                    })
                    .ok();
                }
                // no Autoplay; playback unaffected
                Err(err) => eprintln!("next episode lookup failed: {err}"),
            }
        });
    }

    fn start(&mut self, result: anyhow::Result<PlaybackItem>, cx: &mut Context<Self>) {
        let (Some(mpv), Ok(item)) = (&self.mpv, result.as_ref()) else {
            if let Err(err) = result {
                self.playback.error = Some(format!("Cannot load item: {err}").into());
            }
            cx.notify();
            return;
        };
        if let Err(err) = mpv.load(&item.stream.url, item.resume_seconds()) {
            self.playback.error = Some(err.into());
        }
        self.playback.item = result.ok().map(std::sync::Arc::new);
        cx.notify();
    }

    /// Best-effort final report for `cx.on_app_quit`: cmd-Q skips `close`/`end_current`,
    /// so without this Stopped/Played never reaches the server. Sent directly with the
    /// current session id, bypassing the queued reporter, since its background task
    /// isn't guaranteed to run during shutdown.
    pub fn quit_report(&self) -> Option<impl Future<Output = ()> + use<>> {
        let item = self.playback.item.clone()?;
        if !self.playback.started {
            return None;
        }
        let api = self.api.clone();
        let session = self.playback.play_session_id.clone();
        let (time, paused, finished) = (self.playback.time, self.paused, self.playback.finished);
        Some(async move {
            if finished {
                let _ = api.set_played(&item.id, true).await;
            } else {
                let _ = api
                    .report(Report::Stopped, &item, &session, time, paused)
                    .await;
            }
        })
    }

    /// Volume/mute to persist on quit, mirroring what `Closed` carries on a normal close.
    pub fn volume_state(&self) -> (f64, bool) {
        (self.volume, self.muted)
    }

    /// Track memory to persist on quit, mirroring what `Closed` carries on a normal close.
    pub fn track_prefs(&self) -> TrackPrefs {
        self.track_prefs.clone()
    }

    /// mpv driving playback right now: in-window one, unless PiP has taken over.
    fn active_mpv(&self) -> Option<&Mpv> {
        self.mpv.as_ref().filter(|_| self.pip.is_none())
    }

    /// Reports stop and asks the app to leave the Player. Tears down mpv on drop.
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.is_fullscreen() {
            window.toggle_fullscreen();
        }
        self.end_current();
        cx.emit(Closed {
            volume: self.volume,
            muted: self.muted,
            track_prefs: self.track_prefs.clone(),
        });
    }
}

/// Holds off display/idle sleep while alive (macOS power assertion via NSProcessInfo).
#[cfg(target_os = "macos")]
struct Awake(
    objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn objc2_foundation::NSObjectProtocol>>,
);

#[cfg(target_os = "macos")]
impl Awake {
    fn new() -> Self {
        use objc2_foundation::{NSActivityOptions, NSProcessInfo, ns_string};
        Self(
            NSProcessInfo::processInfo().beginActivityWithOptions_reason(
                NSActivityOptions::UserInitiated | NSActivityOptions::IdleDisplaySleepDisabled,
                ns_string!("Playing video"),
            ),
        )
    }
}

#[cfg(target_os = "macos")]
impl Drop for Awake {
    fn drop(&mut self) {
        // SAFETY: token came from beginActivityWithOptions_reason
        unsafe { objc2_foundation::NSProcessInfo::processInfo().endActivity(&self.0) };
    }
}

#[cfg(not(target_os = "macos"))]
struct Awake;

#[cfg(not(target_os = "macos"))]
impl Awake {
    fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "macos")]
fn hide_cursor() {
    objc2_app_kit::NSCursor::setHiddenUntilMouseMoves(true);
}

#[cfg(not(target_os = "macos"))]
fn hide_cursor() {}
