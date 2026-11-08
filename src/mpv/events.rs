//! mpv's event thread: forwards property changes, file events and log lines to the UI.

use super::{Handle, MpvEvent, Track, TrackKind, get_string};

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
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
}

#[allow(non_upper_case_globals)] // bindgen constant names
pub(super) fn event_thread(
    mpv: Handle,
    stop: Arc<AtomicBool>,
    tx: UnboundedSender<MpvEvent>,
    failure: Arc<Mutex<Option<String>>>,
    secrets: Vec<String>,
) {
    let send = |event| {
        let _ = tx.unbounded_send(event);
    };
    let mut log = LogDedup::default();
    let mut time = TimeThrottle::default();
    while !stop.load(Ordering::SeqCst) {
        let event = unsafe { &*mpv_wait_event(mpv.0, time.wait()) };
        match event.event_id {
            mpv_event_id_MPV_EVENT_NONE => {
                let failed = failure
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .take();
                if let Some(error) = failed {
                    send(MpvEvent::EndFile(Err(error)));
                }
                if let Some(seconds) = time.flush(Instant::now()) {
                    send(MpvEvent::TimePos(seconds));
                }
            }
            mpv_event_id_MPV_EVENT_SHUTDOWN => return,
            mpv_event_id_MPV_EVENT_PROPERTY_CHANGE => {
                let prop = unsafe { &*(event.data as *const mpv_event_property) };
                let name = unsafe { CStr::from_ptr(prop.name) }.to_bytes();
                if name == b"track-list" {
                    send(MpvEvent::Tracks(read_tracks(mpv)));
                    continue;
                }
                if prop.data.is_null() {
                    continue;
                }
                match name {
                    b"volume" => send(MpvEvent::Volume(unsafe { *(prop.data as *const f64) })),
                    b"mute" => send(MpvEvent::Mute(unsafe { *(prop.data as *const c_int) } != 0)),
                    b"time-pos" => {
                        let seconds = unsafe { *(prop.data as *const f64) };
                        if time.pass(seconds, Instant::now()) {
                            send(MpvEvent::TimePos(seconds));
                        }
                    }
                    b"duration" => send(MpvEvent::Duration(unsafe { *(prop.data as *const f64) })),
                    b"pause" => send(MpvEvent::Pause(
                        unsafe { *(prop.data as *const c_int) } != 0,
                    )),
                    b"core-idle" => send(MpvEvent::CoreIdle(
                        unsafe { *(prop.data as *const c_int) } != 0,
                    )),
                    b"hwdec-current" => {
                        let value = unsafe { CStr::from_ptr(*(prop.data as *const *const c_char)) };
                        crate::platform::log::write(&format!(
                            "mpv: hwdec-current={}",
                            value.to_string_lossy()
                        ));
                    }
                    _ => {}
                }
            }
            mpv_event_id_MPV_EVENT_COMMAND_REPLY if event.error < 0 => {
                let msg = unsafe { CStr::from_ptr(mpv_error_string(event.error)) };
                crate::platform::log::write(&format!(
                    "mpv async command: {}",
                    msg.to_string_lossy()
                ));
            }
            mpv_event_id_MPV_EVENT_FILE_LOADED => send(MpvEvent::FileLoaded),
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
                send(MpvEvent::EndFile(result));
            }
            mpv_event_id_MPV_EVENT_LOG_MESSAGE => {
                let msg = unsafe { &*(event.data as *const mpv_event_log_message) };
                let prefix = unsafe { CStr::from_ptr(msg.prefix) }.to_string_lossy();
                let text = unsafe { CStr::from_ptr(msg.text) }.to_string_lossy();
                let prefix = crate::platform::log::redact(&prefix, &secrets);
                let text = crate::platform::log::redact(text.trim_end(), &secrets);
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
            crate::platform::log::write(&format!(
                "{} (repeated {} more times)",
                self.last, self.repeats
            ));
        }
        crate::platform::log::write(&line);
        self.last = line;
        self.repeats = 0;
    }
}

/// Reads `track-list/N/*` sub-properties; video tracks are skipped.
fn read_tracks(mpv: Handle) -> Vec<Track> {
    let count: usize = get_string(mpv, "track-list/count")
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    (0..count)
        .filter_map(|i| {
            let field =
                |f: &str| get_string(mpv, &format!("track-list/{i}/{f}")).filter(|v| !v.is_empty());
            let kind = match field("type")?.as_str() {
                "audio" => TrackKind::Audio,
                "sub" => TrackKind::Subtitle,
                _ => return None,
            };
            let id = field("id")?.parse().ok()?;
            let lang = field("lang");
            // main-selection: 0 primary, 1 secondary (secondary-sid)
            let secondary = field("main-selection").as_deref() == Some("1");
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
                && let Some(channels) = field("demux-channel-count")
            {
                parts.push(format!("{channels}ch"));
            }
            let label = if parts.is_empty() {
                format!("Track {id}")
            } else {
                parts.join(" \u{b7} ")
            };
            Some(Track {
                id,
                kind,
                label,
                selected: field("selected").as_deref() == Some("yes") && !secondary,
                secondary,
                lang,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Instant, TIME_EVERY, TimeThrottle};
    use std::time::Duration;

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
