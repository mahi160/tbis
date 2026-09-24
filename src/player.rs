use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt as _;
use futures::channel::mpsc::{self, UnboundedSender};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState};
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::jellyfin::{Api, Item, PlaybackItem, Report};
use crate::mpv::{Mpv, MpvEvent, Track, TrackKind};

actions!(
    player,
    [
        TogglePause,
        SeekBack,
        SeekForward,
        ToggleFullscreen,
        ExitFullscreen,
        ToggleMute
    ]
);

const CONTEXT: &str = "Player";
const SEEK_STEP: f64 = 10.;

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("space", TogglePause, Some(CONTEXT)),
        KeyBinding::new("left", SeekBack, Some(CONTEXT)),
        KeyBinding::new("right", SeekForward, Some(CONTEXT)),
        KeyBinding::new("f", ToggleFullscreen, Some(CONTEXT)),
        KeyBinding::new("escape", ExitFullscreen, Some(CONTEXT)),
        KeyBinding::new("m", ToggleMute, Some(CONTEXT)),
    ]);
}

/// Player left; carries volume so the app can remember it.
pub struct Closed {
    pub volume: f64,
    pub muted: bool,
}

const HIDE_CONTROLS_AFTER: Duration = Duration::from_secs(3);
const PROGRESS_EVERY: Duration = Duration::from_secs(10);

/// Server update queued by the Player; sent in order, even after it closes.
enum Queued {
    Report(Report, Arc<PlaybackItem>, f64, bool),
    Played(Arc<PlaybackItem>),
}

pub struct PlayerView {
    title: SharedString,
    mpv: Option<Mpv>,
    item: Option<Arc<PlaybackItem>>,
    reports: UnboundedSender<Queued>,
    error: Option<SharedString>,
    time: f64,
    /// Time last drawn; throttles redraws to 0.25s steps.
    shown_time: f64,
    duration: f64,
    paused: bool,
    volume: f64,
    muted: bool,
    tracks: Vec<Track>,
    started: bool,
    finished: bool,
    scrubbing: Option<f64>,
    seek: Entity<SliderState>,
    volume_slider: Entity<SliderState>,
    focus: FocusHandle,
    controls_visible: bool,
    menu_open: bool,
    _hide: Task<()>,
    _tasks: Vec<Task<()>>,
    _subscriptions: [Subscription; 2],
}

impl EventEmitter<Closed> for PlayerView {}

impl PlayerView {
    pub fn new(
        api: Api,
        item: &Item,
        (volume, muted): (f64, bool),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
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
                if let Some(mpv) = &this.mpv {
                    let _ = mpv.set_volume(value.end() as f64);
                }
                this.show_controls(window, cx);
            }),
        ];

        let reports = spawn_reporter(api.clone(), uuid::Uuid::new_v4().simple().to_string(), cx);

        let (events_tx, mut events_rx) = mpsc::unbounded();
        let (mpv, error) = match Mpv::new(window, &api.auth_header(), events_tx) {
            Ok(mpv) => {
                let _ = mpv.set_volume(volume);
                let _ = mpv.set_mute(muted);
                (Some(mpv), None)
            }
            Err(err) => (None, Some(format!("Cannot start player: {err}").into())),
        };

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
            let item_id = item.id.clone();
            tasks.push(cx.spawn(async move |this, cx| {
                let result = api.playback_item(&item_id).await;
                this.update(cx, |this, cx| this.start(result, &api, cx))
                    .ok();
            }));
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

        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let mut this = Self {
            title: item.name.clone().into(),
            mpv,
            item: None,
            reports,
            error,
            time: 0.,
            shown_time: 0.,
            duration: 0.,
            paused: false,
            volume,
            muted,
            tracks: Vec::new(),
            started: false,
            finished: false,
            scrubbing: None,
            seek,
            volume_slider,
            focus,
            controls_visible: true,
            menu_open: false,
            _hide: Task::ready(()),
            _tasks: tasks,
            _subscriptions,
        };
        this.show_controls(window, cx);
        this
    }

    fn start(&mut self, result: anyhow::Result<PlaybackItem>, api: &Api, cx: &mut Context<Self>) {
        let (Some(mpv), Ok(item)) = (&self.mpv, result.as_ref()) else {
            if let Err(err) = result {
                self.error = Some(format!("Cannot load item: {err}").into());
            }
            cx.notify();
            return;
        };
        if let Err(err) = mpv.load(&api.stream_url(item), item.resume_seconds()) {
            self.error = Some(err.into());
        }
        self.item = result.ok().map(Arc::new);
        cx.notify();
    }

    fn report(&self, report: Report) {
        if let Some(item) = &self.item {
            let _ = self.reports.unbounded_send(Queued::Report(
                report,
                item.clone(),
                self.time,
                self.paused,
            ));
        }
    }

    fn on_mpv(&mut self, event: MpvEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            MpvEvent::TimePos(time) => {
                self.time = time;
                if (time - self.shown_time).abs() < 0.25 {
                    return;
                }
                self.shown_time = time;
                self.sync_seek_bar(window, cx);
            }
            // resume jump arrives before duration is known
            MpvEvent::Duration(duration) => {
                self.duration = duration;
                self.sync_seek_bar(window, cx);
            }
            MpvEvent::Pause(paused) => {
                self.paused = paused;
                if self.started {
                    self.report(Report::Progress);
                }
                self.show_controls(window, cx);
            }
            MpvEvent::Volume(volume) => {
                self.volume = volume;
                let shown = self.volume_slider.read(cx).value().end() as f64;
                if (shown - volume).abs() >= 0.5 {
                    self.volume_slider
                        .update(cx, |s, cx| s.set_value(volume as f32, window, cx));
                }
            }
            MpvEvent::Mute(muted) => self.muted = muted,
            MpvEvent::Tracks(tracks) => self.tracks = tracks,
            MpvEvent::FileLoaded => {
                if !self.started {
                    self.started = true;
                    self.report(Report::Start);
                }
            }
            MpvEvent::EndFile(Ok(true)) => {
                self.finished = true;
                self.time = self.duration;
                self.close(window, cx);
                return;
            }
            MpvEvent::EndFile(Ok(false)) => {}
            MpvEvent::EndFile(Err(err)) => {
                self.error = Some(format!("Playback failed: {err}").into())
            }
        }
        cx.notify();
    }

    fn sync_seek_bar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.scrubbing.is_none() && self.duration > 0. {
            let fraction = (self.time / self.duration) as f32;
            self.seek
                .update(cx, |s, cx| s.set_value(fraction, window, cx));
        }
    }

    fn on_seek(&mut self, event: &SliderEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            SliderEvent::Change(value) => self.scrubbing = Some(value.end() as f64 * self.duration),
            SliderEvent::Release(value) => {
                self.scrubbing = None;
                if let Some(mpv) = &self.mpv {
                    let _ = mpv.seek(value.end() as f64 * self.duration);
                }
            }
        }
        self.show_controls(window, cx);
    }

    /// Controls keep focus on the Player so Space etc. never re-trigger a clicked button.
    fn refocus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus.focus(window, cx);
        self.show_controls(window, cx);
    }

    fn toggle_pause(&mut self, _: &TogglePause, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mpv) = &self.mpv {
            let _ = mpv.set_pause(!self.paused);
        }
        self.refocus(window, cx);
    }

    fn seek_back(&mut self, _: &SeekBack, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mpv) = &self.mpv {
            let _ = mpv.seek_by(-SEEK_STEP);
        }
        self.show_controls(window, cx);
    }

    fn seek_forward(&mut self, _: &SeekForward, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mpv) = &self.mpv {
            let _ = mpv.seek_by(SEEK_STEP);
        }
        self.show_controls(window, cx);
    }

    fn toggle_mute(&mut self, _: &ToggleMute, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mpv) = &self.mpv {
            let _ = mpv.set_mute(!self.muted);
        }
        self.refocus(window, cx);
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

    fn exit_fullscreen(&mut self, _: &ExitFullscreen, window: &mut Window, cx: &mut Context<Self>) {
        if window.is_fullscreen() {
            window.toggle_fullscreen();
        }
        self.show_controls(window, cx);
    }

    fn select_track(&mut self, kind: TrackKind, id: Option<i64>) {
        if let Some(mpv) = &self.mpv {
            let _ = mpv.select_track(kind, id);
        }
    }

    /// Reports stop and asks the app to leave the Player. Tears down mpv on drop.
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.is_fullscreen() {
            window.toggle_fullscreen();
        }
        if self.started {
            self.report(Report::Stopped);
        }
        // explicit: Stopped near end alone depends on server's resume thresholds
        if let (true, Some(item)) = (self.finished, &self.item) {
            let _ = self.reports.unbounded_send(Queued::Played(item.clone()));
        }
        cx.emit(Closed {
            volume: self.volume,
            muted: self.muted,
        });
    }

    fn show_controls(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.controls_visible = true;
        self._hide = cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(HIDE_CONTROLS_AFTER).await;
            this.update(cx, |this, cx| {
                let busy = this.paused || this.menu_open || this.scrubbing.is_some();
                if !busy && this.error.is_none() {
                    this.controls_visible = false;
                    hide_cursor();
                    cx.notify();
                }
            })
            .ok();
        });
        cx.notify();
    }

    fn track_menu(&self, kind: TrackKind, cx: &mut Context<Self>) -> Option<AnyElement> {
        let tracks: Vec<Track> = self
            .tracks
            .iter()
            .filter(|t| t.kind == kind)
            .cloned()
            .collect();
        if tracks.is_empty() {
            return None;
        }
        let (id, icon) = match kind {
            TrackKind::Audio => ("player-audio", "icons/audio-lines.svg"),
            TrackKind::Subtitle => ("player-subtitles", "icons/captions.svg"),
        };
        let this = cx.entity().downgrade();
        let on_open = cx.listener(|this, open: &bool, window, cx| {
            this.menu_open = *open;
            if !*open {
                this.refocus(window, cx);
            }
        });
        Some(
            Button::new(id)
                .ghost()
                .icon(Icon::empty().path(icon))
                .dropdown_menu(move |mut menu, _, _| {
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
                .on_open_change(on_open)
                .into_any_element(),
        )
    }
}

/// Sends reports one by one so the server sees them in order.
fn spawn_reporter(
    api: Api,
    play_session_id: String,
    cx: &mut Context<PlayerView>,
) -> UnboundedSender<Queued> {
    let (tx, mut rx) = mpsc::unbounded::<Queued>();
    cx.background_spawn(async move {
        while let Some(queued) = rx.next().await {
            let result = match queued {
                Queued::Report(report, item, seconds, paused) => {
                    api.report(report, &item, &play_session_id, seconds, paused)
                        .await
                }
                Queued::Played(item) => api.mark_played(&item.id).await,
            };
            if let Err(err) = result {
                eprintln!("playback report failed: {err}");
            }
        }
    })
    .detach();
    tx
}

#[cfg(target_os = "macos")]
fn hide_cursor() {
    objc2_app_kit::NSCursor::setHiddenUntilMouseMoves(true);
}

#[cfg(not(target_os = "macos"))]
fn hide_cursor() {}

fn format_time(seconds: f64) -> String {
    let total = if seconds.is_finite() {
        seconds.max(0.) as u64
    } else {
        0
    };
    let (h, m, s) = (total / 3600, total / 60 % 60, total % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

impl Render for PlayerView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(mpv) = &self.mpv {
            let size = window.viewport_size();
            let scale = window.scale_factor();
            mpv.set_size(
                (size.width.as_f32() * scale).round() as i32,
                (size.height.as_f32() * scale).round() as i32,
            );
        }

        let shade = hsla(0., 0., 0., 0.6);
        let clear = hsla(0., 0., 0., 0.);
        let time = self.scrubbing.unwrap_or(self.time);

        let top = h_flex()
            .pt(px(36.)) // below traffic lights
            .px_4()
            .pb_6()
            .gap_3()
            .bg(linear_gradient(
                180.,
                linear_color_stop(shade, 0.),
                linear_color_stop(clear, 1.),
            ))
            .child(
                Button::new("player-back")
                    .ghost()
                    .icon(IconName::ArrowLeft)
                    .on_click(cx.listener(|this, _, window, cx| this.close(window, cx))),
            )
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .truncate()
                    .child(self.title.clone()),
            );

        let volume_icon = if self.muted || self.volume <= 0. {
            "icons/volume-x.svg"
        } else {
            "icons/volume-2.svg"
        };
        let bottom =
            h_flex()
                .px_4()
                .pt_6()
                .pb_4()
                .gap_3()
                .bg(linear_gradient(
                    0.,
                    linear_color_stop(shade, 0.),
                    linear_color_stop(clear, 1.),
                ))
                .child(
                    Button::new("player-play")
                        .ghost()
                        .icon(if self.paused {
                            IconName::Play
                        } else {
                            IconName::Pause
                        })
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.toggle_pause(&TogglePause, window, cx)
                        })),
                )
                .child(div().text_sm().child(format_time(time)))
                .child(
                    div()
                        .flex_1()
                        .child(Slider::new(&self.seek).disabled(self.duration <= 0.)),
                )
                .child(div().text_sm().child(format_time(self.duration)))
                .child(
                    Button::new("player-mute")
                        .ghost()
                        .icon(Icon::empty().path(volume_icon))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.toggle_mute(&ToggleMute, window, cx)
                        })),
                )
                .child(div().w(px(96.)).child(Slider::new(&self.volume_slider)))
                .children(self.track_menu(TrackKind::Audio, cx))
                .children(self.track_menu(TrackKind::Subtitle, cx))
                .child(
                    Button::new("player-fullscreen")
                        .ghost()
                        .icon(if window.is_fullscreen() {
                            IconName::Minimize
                        } else {
                            IconName::Maximize
                        })
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.toggle_fullscreen(&ToggleFullscreen, window, cx)
                        })),
                );

        let error = self.error.clone().map(|error| {
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .p_4()
                        .rounded_md()
                        .bg(shade)
                        .text_color(cx.theme().danger)
                        .child(error),
                )
        });

        v_flex()
            .id("player")
            .key_context(CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::toggle_pause))
            .on_action(cx.listener(Self::seek_back))
            .on_action(cx.listener(Self::seek_forward))
            .on_action(cx.listener(Self::toggle_mute))
            .on_action(cx.listener(Self::toggle_fullscreen))
            .on_action(cx.listener(Self::exit_fullscreen))
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
            .when(self.controls_visible || self.error.is_some(), |this| {
                this.child(top).child(bottom)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::format_time;

    #[::core::prelude::v1::test]
    fn formats_time() {
        assert_eq!(format_time(0.), "0:00");
        assert_eq!(format_time(65.9), "1:05");
        assert_eq!(format_time(3725.), "1:02:05");
        assert_eq!(format_time(f64::NAN), "0:00");
    }
}
