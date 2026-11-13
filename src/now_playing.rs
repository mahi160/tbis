//! Media-key and system "Now Playing" commands for the Player. Registering handlers makes
//! tbis the "now playing" app, so keyboard ⏯ and headset taps reach it. The OS side lives
//! in `platform::now_playing` and publishes whenever asked; this file decides when to ask.

use std::time::Instant;

use futures::channel::mpsc::UnboundedSender;

use crate::platform::now_playing::System;

/// Seconds the system's extrapolated position may drift before republishing.
const DRIFT: f64 = 1.;

/// Media-key / Control Center request, forwarded to the Player.
#[derive(Clone, Copy, Debug)]
pub enum RemoteCommand {
    Play,
    Pause,
    // SMTC sends explicit play/pause; retain the shared command for macOS.
    #[cfg_attr(target_os = "windows", allow(dead_code))]
    Toggle,
    SkipForward,
    SkipBackward,
    Previous,
    Next,
    /// Absolute position in seconds (Control Center scrubber).
    Seek(f64),
}

/// What Now Playing shows; `elapsed` + `rate` let the OS extrapolate the position.
#[derive(Clone, Debug, PartialEq)]
pub struct Info {
    pub title: String,
    pub subtitle: Option<String>,
    pub duration: f64,
    pub elapsed: f64,
    /// 0 while paused, else playback speed.
    pub rate: f64,
}

/// The system Now Playing session; dropping it clears Now Playing.
pub struct NowPlaying {
    system: System,
    /// Last published info and when, to extrapolate its position.
    shown: Option<(Info, Instant)>,
    /// Last published (previous, next) Episode availability.
    episode_nav: Option<(bool, bool)>,
}

impl NowPlaying {
    /// `skip` is the seconds a skip-forward/back command moves.
    pub fn new(window: &gpui_kit::Window, tx: UnboundedSender<RemoteCommand>, skip: f64) -> Self {
        Self {
            system: System::new(window, tx, skip),
            shown: None,
            episode_nav: None,
        }
    }

    /// Publishes only what a user could notice; the system extrapolates position from
    /// elapsed + rate, so steady playback needs no updates, only seeks and drift.
    pub fn update(&mut self, info: &Info) {
        let now = Instant::now();
        if stale(self.shown.as_ref(), info, now) {
            self.system.update(info);
            self.shown = Some((info.clone(), now));
        }
    }

    /// Poster/thumb bytes as downloaded; undecodable data is ignored.
    pub fn set_artwork(&self, bytes: &[u8]) {
        self.system.set_artwork(bytes);
    }

    /// Enables previous/next track for the Episodes that exist.
    pub fn set_episode_nav(&mut self, previous: bool, next: bool) {
        if self.episode_nav != Some((previous, next)) {
            self.system.set_episode_nav(previous, next);
            self.episode_nav = Some((previous, next));
        }
    }
}

/// Whether `info` differs noticeably from `shown`, published at its `Instant`.
fn stale(shown: Option<&(Info, Instant)>, info: &Info, now: Instant) -> bool {
    let Some((shown, at)) = shown else {
        return true;
    };
    let predicted = shown.elapsed + now.duration_since(*at).as_secs_f64() * shown.rate;
    shown.title != info.title
        || shown.subtitle != info.subtitle
        || shown.duration != info.duration
        || shown.rate != info.rate
        || (predicted - info.elapsed).abs() > DRIFT
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{Info, stale};

    fn info(elapsed: f64, rate: f64) -> Info {
        Info {
            title: "S01E01 · Pilot".into(),
            subtitle: Some("Show".into()),
            duration: 1200.,
            elapsed,
            rate,
        }
    }

    #[test]
    fn steady_playback_is_not_republished_until_it_drifts() {
        let at = Instant::now();
        let shown = (info(10., 1.), at);
        let later = |s| at + Duration::from_secs_f64(s);
        assert!(stale(None, &info(10., 1.), at));
        assert!(!stale(Some(&shown), &info(15., 1.), later(5.)));
        assert!(!stale(Some(&shown), &info(15.9, 1.), later(5.)));
        assert!(stale(Some(&shown), &info(16.5, 1.), later(5.)));
        // seek
        assert!(stale(Some(&shown), &info(300., 1.), later(5.)));
        // paused: the system holds the position
        let paused = (info(10., 0.), at);
        assert!(!stale(Some(&paused), &info(10., 0.), later(60.)));
    }

    #[test]
    fn pause_speed_and_item_changes_are_republished() {
        let at = Instant::now();
        let shown = (info(10., 1.), at);
        assert!(stale(Some(&shown), &info(10., 0.), at));
        assert!(stale(Some(&shown), &info(10., 1.5), at));
        let mut next = info(10., 1.);
        next.title = "S01E02 · Second".into();
        assert!(stale(Some(&shown), &next, at));
        let mut longer = info(10., 1.);
        longer.duration = 1300.;
        assert!(stale(Some(&shown), &longer, at));
        let mut movie = info(10., 1.);
        movie.subtitle = None;
        assert!(stale(Some(&shown), &movie, at));
    }
}
