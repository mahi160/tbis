//! Picture-in-Picture (ADR-0006): the Player's own mpv keeps playing; only its video
//! surface moves into a small always-on-top window, so entering and leaving is instant.

use gpui_kit::component::notification::Notification;
use gpui_kit::component::{Root, WindowExt as _};
use gpui_kit::*;

use crate::components::button::round_icon_button;
use crate::components::video::pip_frame;
use crate::design::{self, overlay};

use crate::mpv::{Mpv, VideoSize};

use super::PlayerView;

/// Content of the PiP window: play/pause and back-to-Player over the video.
/// Never reads the Player while rendering: the window first renders while the
/// Player is still mid-update opening it.
pub(super) struct PipView {
    player: WeakEntity<PlayerView>,
    video: VideoSize,
    paused: bool,
    _observe: Subscription,
}

impl PipView {
    fn new(
        player: Entity<PlayerView>,
        video: VideoSize,
        paused: bool,
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
            _observe,
        }
    }
}

impl Render for PipView {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        super::fit_video(&self.video, window);
        let paused = self.paused;
        let play = self.player.clone();
        let back = self.player.clone();
        pip_frame([
            round_icon_button(
                "pip-play",
                if paused {
                    "icons/play.svg"
                } else {
                    "icons/pause.svg"
                },
            )
            .on_click(move |_, _, cx| {
                let _ = play.update(cx, |player, _| {
                    if let Some(mpv) = &player.mpv {
                        let _ = mpv.set_pause(!player.paused);
                    }
                });
            }),
            round_icon_button("pip-return", "icons/pip.svg")
                .tooltip("Back to Player")
                .on_click(move |_, _, cx| request_leave(back.clone(), cx)),
        ])
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
        let Some(video) = self.mpv.as_ref().map(Mpv::video_size) else {
            return;
        };
        if self.pip.is_some() {
            return;
        }
        let size = design::window::PIP;
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
            window_min_size: Some(design::window::PIP_MIN),
            ..Default::default()
        };
        let player = cx.entity();
        let paused = self.paused;
        let opened = cx.open_window(options, |pip_window, cx| {
            crate::platform::window::hide_title_buttons(pip_window);
            let on_close = player.downgrade();
            pip_window.on_window_should_close(cx, move |_, cx| {
                request_leave(on_close.clone(), cx);
                false // leave_pip removes the window once the video is back
            });
            let view = cx.new(|cx| PipView::new(player, video, paused, cx));
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
            Ok(handle) => self.pip = Some(handle),
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
        if let Some(mpv) = &mut self.mpv
            && let Err(err) = mpv.move_to(window)
        {
            eprintln!("PiP: moving video back failed: {err}");
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
