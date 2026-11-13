//! mpv's event thread: forwards property changes, file events and log lines to the UI.

use super::{FileEvent, Handle, MpvEvent, PlayerEvent, Track, TrackKind, get_string, node};

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use futures::channel::mpsc::UnboundedSender;
use libmpv_sys::*;

/// mpv reports `time-pos` about once per frame; the UI needs a few updates a second.
const TIME_EVERY: Duration = Duration::from_millis(250);
/// A jump this large (a seek) passes the throttle at once.
const TIME_JUMP: f64 = 1.;

/// Lets a `time-pos` through every `TIME_EVERY`, or at once after a seek. A held-back
/// value is flushed once mpv goes quiet (paused, small paused seek), so the last
/// position always arrives.
#[derive(Default)]
struct TimeThrottle {
    last: Option<(Instant, f64)>,
    held: Option<f64>,
}

impl TimeThrottle {
    fn pass(&mut self, seconds: f64, now: Instant) -> bool {
        let pass = self.last.is_none_or(|(at, shown)| {
            now.duration_since(at) >= TIME_EVERY || (seconds - shown).abs() >= TIME_JUMP
        });
        if pass {
            self.last = Some((now, seconds));
            self.held = None;
        } else {
            self.held = Some(seconds);
        }
        pass
    }

    /// `mpv_wait_event` timeout: wake to flush a held value, else block.
    fn wait(&self) -> f64 {
        if self.held.is_some() {
            TIME_EVERY.as_secs_f64()
        } else {
            -1.
        }
    }

    fn flush(&mut self, now: Instant) -> Option<f64> {
        let seconds = self.held.take()?;
        self.last = Some((now, seconds));
        Some(seconds)
    }

    fn reset(&mut self) {
        *self = Self::default();
    }
}

#[allow(non_upper_case_globals)] // bindgen constant names
pub(super) fn event_thread(
    mpv: Handle,
    stop: Arc<AtomicBool>,
    tx: UnboundedSender<MpvEvent>,
    secrets: Vec<String>,
) {
    let send = |event| {
        let _ = tx.unbounded_send(event);
    };
    // file events only exist between START_FILE and END_FILE
    let send_file = |load: Option<i64>, event| {
        if let Some(load) = load {
            send(MpvEvent::File { load, event });
        }
    };
    let mut log = LogDedup::default();
    let mut time = TimeThrottle::default();
    let mut load = None;
    while !stop.load(Ordering::SeqCst) {
        let event = unsafe { &*mpv_wait_event(mpv.0, time.wait()) };
        match event.event_id {
            mpv_event_id_MPV_EVENT_NONE => {
                if let Some(seconds) = time.flush(Instant::now()) {
                    send_file(load, FileEvent::TimePos(seconds));
                }
            }
            mpv_event_id_MPV_EVENT_SHUTDOWN => return,
            mpv_event_id_MPV_EVENT_START_FILE => {
                time.reset();
                let start = unsafe { &*(event.data as *const mpv_event_start_file) };
                load = Some(start.playlist_entry_id);
                if let Some(paused) = get_string(mpv, "pause") {
                    send_file(load, FileEvent::Pause(paused == "yes"));
                }
            }
            mpv_event_id_MPV_EVENT_PROPERTY_CHANGE => {
                let prop = unsafe { &*(event.data as *const mpv_event_property) };
                let name = unsafe { CStr::from_ptr(prop.name) }.to_bytes();
                // track-list is observed without a payload; every other value needs one
                if prop.data.is_null() && name != b"track-list" {
                    continue;
                }
                let double = || unsafe { *(prop.data as *const f64) };
                let flag = || unsafe { *(prop.data as *const c_int) } != 0;
                match name {
                    b"volume" => send(MpvEvent::Player(PlayerEvent::Volume(double()))),
                    b"mute" => send(MpvEvent::Player(PlayerEvent::Mute(flag()))),
                    b"hwdec-current" => {
                        let value = unsafe { CStr::from_ptr(*(prop.data as *const *const c_char)) };
                        crate::log::write(&format!(
                            "mpv: hwdec-current={}",
                            value.to_string_lossy()
                        ));
                    }
                    _ if load.is_none() => {}
                    b"track-list" => send_file(load, FileEvent::Tracks(read_tracks(mpv))),
                    b"time-pos" => {
                        let seconds = double();
                        if time.pass(seconds, Instant::now()) {
                            send_file(load, FileEvent::TimePos(seconds));
                        }
                    }
                    b"duration" => send_file(load, FileEvent::Duration(double())),
                    b"pause" => send_file(load, FileEvent::Pause(flag())),
                    b"core-idle" => send_file(load, FileEvent::CoreIdle(flag())),
                    _ => {}
                }
            }
            mpv_event_id_MPV_EVENT_COMMAND_REPLY => {
                // sub-add is the only async command; its userdata is the load it was for
                let load = Some(event.reply_userdata as i64);
                let result = if event.error < 0 {
                    let msg = unsafe { CStr::from_ptr(mpv_error_string(event.error)) };
                    Err(msg.to_string_lossy().into_owned())
                } else {
                    Ok(())
                };
                send_file(load, FileEvent::Tracks(read_tracks(mpv)));
                send_file(load, FileEvent::SubtitleAdded(result));
            }
            mpv_event_id_MPV_EVENT_FILE_LOADED => send_file(load, FileEvent::FileLoaded),
            mpv_event_id_MPV_EVENT_END_FILE => {
                let end = unsafe { &*(event.data as *const mpv_event_end_file) };
                let result = if end.reason == mpv_end_file_reason_MPV_END_FILE_REASON_ERROR as c_int
                {
                    Err(unsafe { CStr::from_ptr(mpv_error_string(end.error)) }
                        .to_string_lossy()
                        .into_owned())
                } else {
                    Ok(end.reason == mpv_end_file_reason_MPV_END_FILE_REASON_EOF as c_int)
                };
                send(MpvEvent::File {
                    load: end.playlist_entry_id,
                    event: FileEvent::EndFile(result),
                });
                if load == Some(end.playlist_entry_id) {
                    time.reset();
                    load = None;
                }
            }
            mpv_event_id_MPV_EVENT_LOG_MESSAGE => {
                let msg = unsafe { &*(event.data as *const mpv_event_log_message) };
                let prefix = unsafe { CStr::from_ptr(msg.prefix) }.to_string_lossy();
                let text = unsafe { CStr::from_ptr(msg.text) }.to_string_lossy();
                let prefix = crate::log::redact(&prefix, &secrets);
                let text = crate::log::redact(text.trim_end(), &secrets);
                log.print(format!("mpv [{prefix}] {text}"));
            }
            _ => {}
        }
    }
}

/// Prints each mpv log line once while it repeats back to back, then a count when
/// another line arrives; some files make a decoder warn on every frame.
#[derive(Default)]
struct LogDedup {
    last: String,
    repeats: u32,
}

impl LogDedup {
    fn print(&mut self, line: String) {
        if line == self.last {
            self.repeats += 1;
            return;
        }
        if self.repeats > 0 {
            crate::log::write(&format!(
                "{} (repeated {} more times)",
                self.last, self.repeats
            ));
        }
        crate::log::write(&line);
        self.last = line;
        self.repeats = 0;
    }
}

/// The current file's audio and subtitle tracks, in one read of `track-list`.
fn read_tracks(mpv: Handle) -> Vec<Track> {
    node::read(mpv, c"track-list", |list| unsafe { tracks(list) }).unwrap_or_default()
}

/// Tracks of a `track-list` node; video tracks are skipped.
///
/// # Safety
/// `list` must be a valid mpv node.
unsafe fn tracks(list: &mpv_node) -> Vec<Track> {
    unsafe { node::list(list) }
        .iter()
        .filter_map(|track| {
            let field = |f: &str| unsafe { node::string(track, f) }.filter(|v| !v.is_empty());
            let int = |f: &str| unsafe { node::int(track, f) };
            let kind = match field("type")?.as_str() {
                "audio" => TrackKind::Audio,
                "sub" => TrackKind::Subtitle,
                _ => return None,
            };
            let id = int("id")?;
            let lang = field("lang");
            // main-selection: 0 primary, 1 secondary (secondary-sid)
            let secondary = int("main-selection") == Some(1);
            let mut parts: Vec<String> = [field("title"), lang.clone()]
                .into_iter()
                .flatten()
                .collect();
            // untagged: title/lang both missing (common for scene-released subs) --
            // codec alone repeats identically across tracks, so tack on the id too.
            let untagged = parts.is_empty();
            if let Some(codec) = field("codec") {
                parts.push(codec);
            }
            if untagged {
                parts.push(format!("#{id}"));
            }
            if kind == TrackKind::Audio
                && let Some(channels) = int("demux-channel-count")
            {
                parts.push(format!("{channels}ch"));
            }
            let label = if parts.is_empty() {
                format!("Track {id}")
            } else {
                parts.join(" \u{b7} ")
            };
            let selected = unsafe { node::flag(track, "selected") } == Some(true);
            Some(Track {
                id,
                kind,
                label,
                selected: selected && !secondary,
                secondary,
                lang,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{FileEvent, Instant, MpvEvent, PlayerEvent, TIME_EVERY, TimeThrottle, TrackKind};
    use crate::mpv::node::build::{array, flag, int, map, string};
    use std::time::Duration;

    #[test]
    fn tracks_come_from_one_track_list_node() {
        let list = array(vec![
            map(vec![
                ("id", int(1)),
                ("type", string("video")),
                ("codec", string("hevc")),
                ("selected", flag(true)),
            ]),
            map(vec![
                ("id", int(1)),
                ("type", string("audio")),
                ("title", string("Surround")),
                ("lang", string("eng")),
                ("codec", string("eac3")),
                ("demux-channel-count", int(6)),
                ("selected", flag(true)),
                ("main-selection", int(0)),
            ]),
            map(vec![
                ("id", int(2)),
                ("type", string("audio")),
                ("codec", string("aac")),
                ("selected", flag(false)),
            ]),
            map(vec![
                ("id", int(1)),
                ("type", string("sub")),
                ("lang", string("eng")),
                ("codec", string("subrip")),
                ("selected", flag(true)),
                ("main-selection", int(0)),
            ]),
            map(vec![
                ("id", int(2)),
                ("type", string("sub")),
                ("title", string("")),
                ("codec", string("ass")),
                ("selected", flag(true)),
                ("main-selection", int(1)),
            ]),
        ]);
        let tracks = unsafe { super::tracks(&list) };
        let shown: Vec<_> = tracks
            .iter()
            .map(|t| {
                let kind = match t.kind {
                    TrackKind::Audio => "audio",
                    TrackKind::Subtitle => "sub",
                };
                let lang = t.lang.as_deref().unwrap_or("-");
                let (id, label, selected, secondary) = (t.id, &t.label, t.selected, t.secondary);
                format!("{kind} {id} [{label}] {selected} {secondary} {lang}")
            })
            .collect();
        assert_eq!(
            shown,
            [
                // tagged, with its channel count; the video track is skipped
                "audio 1 [Surround · eng · eac3 · 6ch] true false eng",
                // untagged: the id tells it apart
                "audio 2 [aac · #2] false false -",
                "sub 1 [eng · subrip] true false eng",
                // secondary is never also the primary selection
                "sub 2 [ass · #2] false true -",
            ]
        );
    }

    #[test]
    fn stale_file_events_are_rejected_but_volume_and_mute_are_player_wide() {
        for event in [
            FileEvent::TimePos(12.),
            FileEvent::Duration(100.),
            FileEvent::Pause(true),
            FileEvent::CoreIdle(true),
            FileEvent::Tracks(Vec::new()),
            FileEvent::FileLoaded,
            FileEvent::EndFile(Ok(true)),
            FileEvent::EndFile(Err("failure".into())),
        ] {
            let event = MpvEvent::File { load: 1, event };
            assert!(event.is_current(Some(1)));
            assert!(!event.is_current(None));
            assert!(!event.is_current(Some(2)));
        }
        for event in [PlayerEvent::Volume(50.), PlayerEvent::Mute(true)] {
            let event = MpvEvent::Player(event);
            assert!(event.is_current(None));
            assert!(event.is_current(Some(2)));
        }
    }

    #[test]
    fn render_failure_before_any_load_still_reaches_the_player() {
        let event = MpvEvent::Player(PlayerEvent::RenderFailed("no drawable".into()));
        assert!(event.is_current(None));
        assert!(event.is_current(Some(3)));
    }

    #[test]
    fn delayed_positions_cannot_change_episode_or_bitrate_reload_positions() {
        let start = Instant::now();
        let mut throttle = TimeThrottle::default();
        let mut position = 20.;
        assert!(throttle.pass(position, start));
        assert!(!throttle.pass(20.1, start + Duration::from_millis(40)));

        for next in [2, 3] {
            throttle.reset();
            let current = Some(next);
            assert_eq!(throttle.flush(start + TIME_EVERY), None);
            let delayed = MpvEvent::File {
                load: next - 1,
                event: FileEvent::TimePos(20.1),
            };
            if delayed.is_current(current)
                && let MpvEvent::File {
                    event: FileEvent::TimePos(seconds),
                    ..
                } = delayed
            {
                position = seconds;
            }
            assert_eq!(position, 20.);
            assert!(throttle.pass(position, start));
            assert!(!throttle.pass(20.1, start + Duration::from_millis(40)));
        }
    }

    #[test]
    fn throttle_discards_held_positions_on_each_load_boundary() {
        let start = Instant::now();
        let mut throttle = TimeThrottle::default();
        for position in [10., 10.1] {
            assert!(throttle.pass(position, start));
            assert!(!throttle.pass(position + 0.04, start + Duration::from_millis(40)));
            throttle.reset();
            assert_eq!(throttle.flush(start + TIME_EVERY), None);
            assert_eq!(throttle.wait(), -1.);
        }
        assert!(throttle.pass(10.2, start + Duration::from_millis(50)));
    }

    #[test]
    fn time_throttle_passes_first_interval_and_seeks() {
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        let mut throttle = TimeThrottle::default();
        assert!(throttle.pass(10., at(0)));
        assert!(!throttle.pass(10.04, at(40)));
        assert!(!throttle.pass(10.2, at(200)));
        assert!(throttle.pass(10.25, at(TIME_EVERY.as_millis() as u64)));
        // seek: big jump passes immediately, backwards too
        assert!(throttle.pass(70., at(260)));
        assert!(throttle.pass(5., at(270)));
        assert!(!throttle.pass(5.04, at(310)));
        // a held value waits for the quiet-time flush
        assert!(throttle.wait() > 0.);
        assert_eq!(throttle.flush(at(560)), Some(5.04));
        assert_eq!(throttle.wait(), -1.);
        assert_eq!(throttle.flush(at(600)), None);
    }
}
