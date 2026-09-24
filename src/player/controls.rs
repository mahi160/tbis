//! Bars, menus, the up-next card, and the `Render` impl.

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::slider::{Slider, SliderEvent};
use gpui_kit::component::{ActiveTheme as _, Icon, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::mpv::{Track, TrackKind};

use super::clock::{clock, format_time};
use super::{
    CONTEXT, Escape, HIDE_CONTROLS_AFTER, PlayNext, PlayerView, SEEK_STEP, SeekBack, SeekForward,
    ToggleFullscreen, ToggleMute, TogglePause, TogglePip, hide_cursor,
};

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

    fn seek_back(&mut self, _: &SeekBack, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mpv) = self.active_mpv() {
            let _ = mpv.seek_by(-SEEK_STEP);
        }
        self.show_controls(window, cx);
    }

    fn seek_forward(&mut self, _: &SeekForward, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mpv) = self.active_mpv() {
            let _ = mpv.seek_by(SEEK_STEP);
        }
        self.show_controls(window, cx);
    }

    fn toggle_mute(&mut self, _: &ToggleMute, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mpv) = self.active_mpv() {
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

    /// Cancels Autoplay while its card shows, else leaves fullscreen.
    fn escape(&mut self, _: &Escape, window: &mut Window, cx: &mut Context<Self>) {
        if self.up_next().is_some() {
            self.playback.next_cancelled = true;
        } else if window.is_fullscreen() {
            window.toggle_fullscreen();
        }
        self.show_controls(window, cx);
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
        let start = crate::pip::PipStart {
            url: &self.api.stream_url(item),
            auth_header: &self.api.auth_header(),
            start_seconds: self.playback.time,
            speed: self.speed,
            volume: self.volume,
            muted: self.muted,
            audio_track: selected(TrackKind::Audio),
            subtitle_track: selected(TrackKind::Subtitle),
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
            Err(err) => {
                self.pip_error = Some(err.into());
                self._pip_error = cx.spawn_in(window, async move |this, cx| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_secs(4))
                        .await;
                    this.update(cx, |this, cx| {
                        this.pip_error = None;
                        cx.notify();
                    })
                    .ok();
                });
            }
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
/// "playing in PiP" notice (the PiP-start `pip_error` toast uses its own smaller one).
fn overlay_banner(color: Option<Hsla>, content: impl IntoElement) -> AnyElement {
    let mut label = div().p_4().rounded_md().bg(overlay_scrim());
    if let Some(color) = color {
        label = label.text_color(color);
    }
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
                div()
                    .flex_none()
                    .font_family(mono.clone())
                    .text_size(px(17.))
                    .text_color(dim(0.75))
                    .child(clock(0.)),
            );

        let timeline = h_flex()
            .gap_3()
            .font_family(mono.clone())
            .text_sm()
            .text_color(dim(0.85))
            .child(format_time(time))
            .child(
                div().flex_1().child(
                    Slider::new(&self.seek)
                        .bg(cx.theme().primary)
                        .text_color(white())
                        .disabled(self.playback.duration <= 0. || self.pip.is_some()),
                ),
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
            .when(self.playback.next.is_some(), |this| {
                this.child(
                    icon_button("player-next", "icons/forward-step.svg", cx).on_click(cx.listener(
                        |this, _, window, cx| {
                            if let Some(next) = this.playback.next.clone() {
                                this.play_next(next, window, cx);
                            }
                        },
                    )),
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
            .children(self.track_menu(TrackKind::Audio, cx))
            .children(self.track_menu(TrackKind::Subtitle, cx))
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

        let error = self
            .playback
            .error
            .clone()
            .map(|error| overlay_banner(Some(cx.theme().danger), error));

        let pip_notice = self.pip.is_some().then(|| {
            overlay_banner(
                None,
                "Playing in Picture-in-Picture \u{b7} press P to return",
            )
        });
        let pip_error = self.pip_error.clone().map(|error| {
            div()
                .absolute()
                .top(px(96.))
                .left_0()
                .right_0()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .px_4()
                        .py_2()
                        .rounded_md()
                        .bg(overlay_scrim())
                        .text_color(cx.theme().danger)
                        .child(error),
                )
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
                .rounded_lg()
                .bg(video_black(0.8))
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
            .on_action(cx.listener(Self::toggle_mute))
            .on_action(cx.listener(Self::toggle_fullscreen))
            .on_action(cx.listener(Self::escape))
            .on_action(cx.listener(Self::play_next_now))
            .on_action(cx.listener(Self::toggle_pip))
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
            .children(pip_error)
            .children(up_next)
            .when(
                self.controls_visible || self.playback.error.is_some(),
                |this| this.child(top).child(bottom),
            )
    }
}
