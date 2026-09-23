use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt as _;
use futures::channel::mpsc::{self, UnboundedSender};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState};
use gpui_kit::component::{ActiveTheme as _, IconName, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::jellyfin::{Api, Item, PlaybackItem, Report};
use crate::mpv::{Mpv, MpvEvent};

pub struct Closed;

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
    duration: f64,
    paused: bool,
    started: bool,
    finished: bool,
    scrubbing: Option<f64>,
    seek: Entity<SliderState>,
    controls_visible: bool,
    _hide: Task<()>,
    _tasks: Vec<Task<()>>,
    _subscription: Subscription,
}

impl EventEmitter<Closed> for PlayerView {}

impl PlayerView {
    pub fn new(api: Api, item: &Item, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let seek = cx.new(|_| SliderState::new().min(0.).max(1.).step(0.0001));
        let _subscription = cx.subscribe_in(&seek, window, |this, _, event, _, cx| {
            this.on_seek(event, cx)
        });

        let reports = spawn_reporter(api.clone(), uuid::Uuid::new_v4().simple().to_string(), cx);

        let (events_tx, mut events_rx) = mpsc::unbounded();
        let (mpv, error) = match Mpv::new(window, &api.auth_header(), events_tx) {
            Ok(mpv) => (Some(mpv), None),
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

        let mut this = Self {
            title: item.name.clone().into(),
            mpv,
            item: None,
            reports,
            error,
            time: 0.,
            duration: 0.,
            paused: false,
            started: false,
            finished: false,
            scrubbing: None,
            seek,
            controls_visible: true,
            _hide: Task::ready(()),
            _tasks: tasks,
            _subscription,
        };
        this.show_controls(cx);
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
                let redraw = (time - self.time).abs() >= 0.25;
                self.time = time;
                if !redraw {
                    return;
                }
                if self.scrubbing.is_none() && self.duration > 0. {
                    let fraction = (time / self.duration) as f32;
                    self.seek
                        .update(cx, |s, cx| s.set_value(fraction, window, cx));
                }
            }
            MpvEvent::Duration(duration) => self.duration = duration,
            MpvEvent::Pause(paused) => {
                self.paused = paused;
                if self.started {
                    self.report(Report::Progress);
                }
                self.show_controls(cx);
            }
            MpvEvent::FileLoaded => {
                if !self.started {
                    self.started = true;
                    self.report(Report::Start);
                }
            }
            MpvEvent::EndFile(Ok(true)) => {
                self.finished = true;
                self.time = self.duration;
                self.close(cx);
                return;
            }
            MpvEvent::EndFile(Ok(false)) => {}
            MpvEvent::EndFile(Err(err)) => {
                self.error = Some(format!("Playback failed: {err}").into())
            }
        }
        cx.notify();
    }

    fn on_seek(&mut self, event: &SliderEvent, cx: &mut Context<Self>) {
        match event {
            SliderEvent::Change(value) => self.scrubbing = Some(value.end() as f64 * self.duration),
            SliderEvent::Release(value) => {
                self.scrubbing = None;
                if let Some(mpv) = &self.mpv {
                    let _ = mpv.seek(value.end() as f64 * self.duration);
                }
            }
        }
        self.show_controls(cx);
        cx.notify();
    }

    fn toggle_pause(&mut self, cx: &mut Context<Self>) {
        if let Some(mpv) = &self.mpv {
            let _ = mpv.set_pause(!self.paused);
        }
        self.show_controls(cx);
    }

    /// Reports stop and asks the app to leave the Player. Tears down mpv on drop.
    fn close(&mut self, cx: &mut Context<Self>) {
        if self.started {
            self.report(Report::Stopped);
        }
        // explicit: Stopped near end alone depends on server's resume thresholds
        if let (true, Some(item)) = (self.finished, &self.item) {
            let _ = self.reports.unbounded_send(Queued::Played(item.clone()));
        }
        cx.emit(Closed);
    }

    fn show_controls(&mut self, cx: &mut Context<Self>) {
        self.controls_visible = true;
        self._hide = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(HIDE_CONTROLS_AFTER).await;
            this.update(cx, |this, cx| {
                if !this.paused && this.scrubbing.is_none() && this.error.is_none() {
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
                    .on_click(cx.listener(|this, _, _, cx| this.close(cx))),
            )
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .truncate()
                    .child(self.title.clone()),
            );

        let bottom = h_flex()
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
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_pause(cx))),
            )
            .child(div().text_sm().child(format_time(time)))
            .child(
                div()
                    .flex_1()
                    .child(Slider::new(&self.seek).disabled(self.duration <= 0.)),
            )
            .child(div().text_sm().child(format_time(self.duration)));

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
            .size_full()
            .relative()
            .justify_between()
            .text_color(white())
            .on_mouse_move(cx.listener(|this, _, _, cx| {
                if !this.controls_visible {
                    this.show_controls(cx);
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
