//! Picture-in-Picture (ADR-0006): the Player's own mpv keeps playing; only its video
//! surface moves into a small always-on-top window, so entering and leaving is instant.

use gpui_kit::component::notification::Notification;
use gpui_kit::component::slider::{Slider, SliderState};
use gpui_kit::component::{Root, WindowExt as _};
use gpui_kit::*;

use crate::components::button::round_icon_button;
use crate::components::video::pip_frame;
use crate::design::{self, color, overlay};

use crate::mpv::{Mpv, VideoSize};

use super::{
    CONTEXT, Escape, PlayerView, SeekBack, SeekBackLong, SeekForward, SeekForwardLong,
    TogglePause, TogglePip,
};

/// mpv sizes subtitles to the window, so they shrink to unreadable in PiP; this enlarges them.
const PIP_SUB_SCALE: f64 = 2.;

/// Multiplies mpv's current `sub-scale` by `factor`; reading it back keeps the user's style.
fn scale_subs(mpv: &Mpv, factor: f64) {
    if let Some(scale) = mpv
        .property("sub-scale")
        .and_then(|s| s.parse::<f64>().ok())
    {
        let _ = mpv.set_property("sub-scale", &(scale * factor).to_string());
    }
}

/// Content of the PiP window: play/pause and back-to-Player over the video.
/// Never reads the Player while rendering: the window first renders while the
/// Player is still mid-update opening it.
pub(super) struct PipView {
    player: WeakEntity<PlayerView>,
    video: VideoSize,
    paused: bool,
    /// The Player's own seek bar state: scrubbing here goes through its `on_seek`.
    seek: Entity<SliderState>,
    focus: FocusHandle,
    _observe: Subscription,
}

impl PipView {
    fn new(
        player: Entity<PlayerView>,
        video: VideoSize,
        paused: bool,
        seek: Entity<SliderState>,
        cx: &mut Context<Self>,
    ) -> Self {
        let _observe = cx.observe(&player, |this, player, cx| {
            this.paused = player.read(cx).paused;
            cx.notify();
        });
        Self {
            player: player.downgrade(),
            video,
            paused,
            seek,
            focus: cx.focus_handle(),
            _observe,
        }
    }

    fn toggle_pause(&mut self, _: &TogglePause, _: &mut Window, cx: &mut Context<Self>) {
        let _ = self.player.update(cx, |player, _| {
            if let Some(mpv) = &player.mpv {
                let _ = mpv.set_pause(!player.paused);
            }
        });
    }

    fn seek(&self, forward: bool, long: bool, cx: &mut App) {
        let _ = self
            .player
            .update(cx, |player, cx| player.seek_by_step(forward, long, cx));
    }

    fn back(&mut self, cx: &mut App) {
        request_leave(self.player.clone(), cx);
    }
}

impl Render for PipView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        super::fit_video(&self.video, window);
        let seek_bar = Slider::new(&self.seek)
            .bg(color::primary(cx))
            .text_color(overlay::WHITE);
        pip_frame(
            [
                round_icon_button(
                    "pip-play",
                    if self.paused {
                        "icons/play.svg"
                    } else {
                        "icons/pause.svg"
                    },
                )
                .on_click(
                    cx.listener(|this, _, window, cx| this.toggle_pause(&TogglePause, window, cx)),
                ),
                round_icon_button("pip-return", "icons/pip.svg")
                    .tooltip("Back to Player")
                    .on_click(cx.listener(|this, _, _, cx| this.back(cx))),
            ],
            seek_bar,
        )
        // same keys as the Player, once PiP is clicked
        .key_context(CONTEXT)
        .track_focus(&self.focus)
        .on_any_mouse_down(cx.listener(|this, _, window, cx| this.focus.focus(window, cx)))
        .on_action(cx.listener(Self::toggle_pause))
        .on_action(cx.listener(|this, _: &SeekBack, _, cx| this.seek(false, false, cx)))
        .on_action(cx.listener(|this, _: &SeekForward, _, cx| this.seek(true, false, cx)))
        .on_action(cx.listener(|this, _: &SeekBackLong, _, cx| this.seek(false, true, cx)))
        .on_action(cx.listener(|this, _: &SeekForwardLong, _, cx| this.seek(true, true, cx)))
        .on_action(cx.listener(|this, _: &TogglePip, _, cx| this.back(cx)))
        .on_action(cx.listener(|this, _: &Escape, _, cx| this.back(cx)))
        .on_action(cx.listener(|this, _: &crate::CloseWindow, _, cx| this.back(cx)))
    }
}

/// Leaves PiP from the PiP window: the main window is needed to re-host the video,
/// and it can't be borrowed from inside this window's update, so this runs deferred.
fn request_leave(player: WeakEntity<PlayerView>, cx: &mut App) {
    cx.defer(move |cx| {
        let Some(player) = player.upgrade() else {
            return;
        };
        let main = player.read(cx).main_window;
        let _ = main.update(cx, |_, window, cx| {
            player.update(cx, |player, cx| player.leave_pip(window, cx))
        });
    });
}

impl PlayerView {
    /// Opens the PiP window bottom-right of the Player's screen and moves the video there.
    pub(super) fn enter_pip(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(mpv) = &self.mpv else {
            return;
        };
        if self.pip.is_some() {
            return;
        }
        let video = mpv.video_size();
        let dimension = |name| mpv.property(name)?.parse::<f32>().ok().filter(|v| *v > 0.);
        // 16:9 until the first frame reports its size
        let (width, height) = dimension("dwidth")
            .zip(dimension("dheight"))
            .unwrap_or((16., 9.));
        let fit = |w: Pixels| size(w, w * (height / width));
        let size = fit(design::window::PIP_W);
        let margin = design::window::PIP_MARGIN;
        let bounds = window.display(cx).map_or_else(
            || Bounds::centered(None, size, cx),
            |display| {
                let area = display.visible_bounds();
                let corner = area.bottom_right() - point(size.width + margin, size.height + margin);
                Bounds::new(corner, size)
            },
        );
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            // transparent bar keeps it resizable; buttons hidden below
            titlebar: Some(TitlebarOptions {
                title: None,
                appears_transparent: true,
                traffic_light_position: None,
            }),
            kind: WindowKind::PopUp,
            focus: false,
            is_minimizable: false,
            window_background: WindowBackgroundAppearance::Transparent,
            window_min_size: Some(fit(design::window::PIP_MIN_W)),
            ..Default::default()
        };
        let player = cx.entity();
        let paused = self.paused;
        let seek = self.seek.clone();
        let opened = cx.open_window(options, |pip_window, cx| {
            crate::platform::window::hide_title_buttons(pip_window);
            crate::platform::window::lock_aspect(pip_window, width, height);
            let on_close = player.downgrade();
            pip_window.on_window_should_close(cx, move |_, cx| {
                request_leave(on_close.clone(), cx);
                false // leave_pip removes the window once the video is back
            });
            let view = cx.new(|cx| PipView::new(player, video, paused, seek, cx));
            let root = cx.new(|cx| Root::new(view, pip_window, cx));
            root.update(cx, |root, _| root.style().background = Some(overlay::CLEAR.into()));
            root
        });
        let Some(mpv) = &mut self.mpv else {
            return;
        };
        let moved = opened.map_err(|e| e.to_string()).and_then(|handle| {
            let handle = AnyWindowHandle::from(handle);
            let result = handle
                .update(cx, |_, pip_window, _| mpv.move_to(pip_window))
                .map_err(|e| e.to_string())
                .and_then(|moved| moved);
            if result.is_err() {
                let _ = handle.update(cx, |_, pip_window, _| pip_window.remove_window());
            }
            result.map(|()| handle)
        });
        match moved {
            Ok(handle) => {
                scale_subs(mpv, PIP_SUB_SCALE);
                self.pip = Some(handle);
            }
            // not fatal: video stays in the Player
            Err(err) => window.push_notification(
                Notification::error(format!("Cannot start PiP: {err}")),
                cx,
            ),
        }
        cx.notify();
    }

    /// Moves the video back into the Player's `window` and closes the PiP window.
    pub(super) fn leave_pip(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(handle) = self.pip.take() else {
            return;
        };
        if let Some(mpv) = &mut self.mpv {
            scale_subs(mpv, 1. / PIP_SUB_SCALE);
            if let Err(err) = mpv.move_to(window) {
                eprintln!("PiP: moving video back failed: {err}");
            }
        }
        let _ = handle.update(cx, |_, pip_window, _| pip_window.remove_window());
        self.refocus(window, cx);
    }

    /// Closes the PiP window without moving the video back; the Player is closing.
    pub(super) fn drop_pip(&mut self, cx: &mut App) {
        if let Some(handle) = self.pip.take() {
            let _ = handle.update(cx, |_, pip_window, _| pip_window.remove_window());
        }
    }
}
