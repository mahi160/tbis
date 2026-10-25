//! Video Player. Split by concern: this file is lifecycle (open/load/close/quit) and
//! the `PlayerView` type itself; [`playback`] is the current item's state and its
//! server reports; [`controls`] is the actions, overlays and `Render` impl, with the
//! top and bottom bars and their menus in `controls::chrome`;
//! [`clock`] is the small pure time-formatting helpers; [`pip`] is Picture-in-Picture.

mod clock;
mod controls;
mod pip;
mod playback;

use std::future::Future;
use std::time::Duration;

use futures::StreamExt as _;
use futures::channel::mpsc;
use gpui_kit::component::slider::{SliderEvent, SliderState};
use gpui_kit::*;

use crate::components::video::InfoGroup;
use crate::config::{Config, LanguagePref, TrackPrefs};
use crate::jellyfin::{Api, Item, PlaybackItem, Report};
use crate::mpv::{Mpv, VideoSize};
use crate::now_playing::{NowPlaying, RemoteCommand};
use crate::platform::cursor::hide as hide_cursor;
use crate::platform::power::Awake;
use crate::shaders::ShaderProfile;
use playback::Playback;

actions!(
    player,
    [
        TogglePause,
        SeekBack,
        SeekForward,
        SeekBackLong,
        SeekForwardLong,
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
        Screenshot,
        CycleShaders
    ]
);

const CONTEXT: &str = "Player";
const VOLUME_STEP: f64 = 5.;
const DELAY_STEP: f64 = 0.1;

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("space", TogglePause, Some(CONTEXT)),
        KeyBinding::new("left", SeekBack, Some(CONTEXT)),
        KeyBinding::new("right", SeekForward, Some(CONTEXT)),
        // not ctrl: macOS switches Spaces on ctrl-arrows
        KeyBinding::new("alt-left", SeekBackLong, Some(CONTEXT)),
        KeyBinding::new("alt-right", SeekForwardLong, Some(CONTEXT)),
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
        KeyBinding::new("u", CycleShaders, Some(CONTEXT)),
    ]);
}

/// Player left; carries volume and track memory so the app can remember them.
pub struct Closed {
    pub volume: f64,
    pub muted: bool,
    pub track_prefs: TrackPrefs,
}

const HIDE_CONTROLS_AFTER: Duration = Duration::from_secs(3);
const VOLUME_HUD_AFTER: Duration = Duration::from_millis(1200);
const PROGRESS_EVERY: Duration = Duration::from_secs(10);
const CLOCK_EVERY: Duration = Duration::from_secs(30);

pub struct PlayerView {
    api: Api,
    mpv: Option<Mpv>,
    playback: Playback,
    /// Picture-in-Picture window currently showing this Player's video.
    pip: Option<AnyWindowHandle>,
    /// Window the Player lives in; PiP hands the video back to it.
    main_window: AnyWindowHandle,
    paused: bool,
    /// mpv not advancing; with `!paused`, means buffering.
    core_idle: bool,
    speed: f64,
    volume: f64,
    muted: bool,
    seek: Entity<SliderState>,
    volume_slider: Entity<SliderState>,
    focus: FocusHandle,
    controls_visible: bool,
    /// Volume slider forced open by VolumeUp/VolumeDown, independent of mute-button hover.
    volume_hud: bool,
    menu_open: bool,
    track_prefs: TrackPrefs,
    /// Settings' preferred languages (read-only here).
    language: LanguagePref,
    /// Settings' streaming cap; over it the server may transcode.
    max_bitrate_mbps: Option<u32>,
    /// Upscaling shaders active in mpv.
    shaders: ShaderProfile,
    /// Held while video actually plays; synced in `render`.
    awake: Option<Awake>,
    /// Media keys + Control Center.
    now_playing: Option<NowPlaying>,
    remote: mpsc::UnboundedSender<RemoteCommand>,
    /// Playback Info overlay rows while open; refreshed every second by `_info`.
    info: Option<Vec<InfoGroup>>,
    _info: Task<()>,
    _hide: Task<()>,
    _volume_hud: Task<()>,
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
        let defaults: Vec<(&str, String)> = config
            .subtitles
            .mpv_options()
            .into_iter()
            .chain(config.audio.mpv_options())
            .collect();
        let (mpv, mpv_error) = match Mpv::new(window, &api.auth_header(), &defaults, events_tx) {
            Ok(mpv) => {
                let _ = mpv.set_volume(volume);
                let _ = mpv.set_mute(muted);
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
                        if !this.paused {
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
            main_window: window.window_handle(),
            paused: false,
            core_idle: true,
            speed: 1.,
            volume,
            muted,
            seek,
            volume_slider,
            focus,
            controls_visible: true,
            volume_hud: false,
            menu_open: false,
            track_prefs: config.track_prefs.clone(),
            language: config.language.clone(),
            max_bitrate_mbps: config.max_bitrate_mbps,
            shaders: ShaderProfile::Off,
            awake: None,
            now_playing: None,
            remote,
            info: None,
            _info: Task::ready(()),
            _hide: Task::ready(()),
            _volume_hud: Task::ready(()),
            _tasks: tasks,
            _subscriptions,
        };
        if config.shaders != ShaderProfile::Off {
            this.apply_shaders(config.shaders, false, window, cx);
        }
        this.load(item.id.clone(), None, cx);
        this.show_controls(window, cx);
        this
    }

    /// Fetches fresh play details, starts playback at `start_at` (else the server's
    /// resume point), then looks up the next Episode.
    fn load(&mut self, item_id: String, start_at: Option<f64>, cx: &mut Context<Self>) {
        if self.mpv.is_none() {
            return;
        }
        self.playback.start_at = start_at;
        let api = self.api.clone();
        let artwork_url = self.playback.artwork_url.clone();
        let max_mbps = self.max_bitrate_mbps;
        self.playback._load = cx.spawn(async move |this, cx| {
            let result = api.playback_item(&item_id, max_mbps).await;
            let loaded = result.is_ok();
            let series_id = result.as_ref().ok().and_then(|i| i.series_id.clone());
            if this
                .update(cx, |this, cx| this.start(result, start_at, cx))
                .is_err()
                || !loaded
            {
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

    fn start(
        &mut self,
        result: anyhow::Result<PlaybackItem>,
        start_at: Option<f64>,
        cx: &mut Context<Self>,
    ) {
        let (Some(mpv), Ok(item)) = (&self.mpv, result.as_ref()) else {
            if let Err(err) = result {
                self.playback.error = Some(format!("Cannot load item: {err}").into());
            }
            cx.notify();
            return;
        };
        let start = start_at.unwrap_or_else(|| item.resume_seconds());
        if let Err(err) = mpv.load(&item.stream.url, start) {
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
}

/// Sizes mpv's video to `window`'s viewport in device pixels.
fn fit_video(video: &VideoSize, window: &Window) {
    let size = window.viewport_size();
    let scale = window.scale_factor();
    video.set(
        (size.width.as_f32() * scale).round() as i32,
        (size.height.as_f32() * scale).round() as i32,
        scale,
    );
}

impl PlayerView {
    fn active_mpv(&self) -> Option<&Mpv> {
        self.mpv.as_ref()
    }

    /// Reports stop and asks the app to leave the Player. Tears down mpv on drop.
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.is_fullscreen() {
            window.toggle_fullscreen();
        }
        self.end_current();
        self.drop_pip(cx);
        cx.emit(Closed {
            volume: self.volume,
            muted: self.muted,
            track_prefs: self.track_prefs.clone(),
        });
    }
}
