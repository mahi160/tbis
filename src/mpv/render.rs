//! mpv's render thread: drives the render loop and, with `TBIS_FRAME_STATS=1`, logs
//! frame pacing and render pass timings.

use super::{Extent, Handle, MpvEvent, PlayerEvent, get_string, node, skip_frame};
use crate::platform::video;

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use futures::channel::mpsc::UnboundedSender;
use libmpv_sys::*;

/// Milliseconds a renderer spent in each named phase of the frame it just drew, filled
/// only while frame stats are on.
pub type Phases = Vec<(&'static str, f64)>;

/// Frame pacing, drop counts and renderer phases, logged every 5 seconds with
/// `TBIS_FRAME_STATS=1`.
struct FrameStats {
    since: Instant,
    frames: u32,
    draw_total: Duration,
    draw_max: Duration,
    last_frame: Option<Instant>,
    gap_max: Duration,
    /// Per-phase totals, in the order the renderer first reported them.
    phases: Vec<(&'static str, Timing)>,
}

#[derive(Default)]
struct Timing {
    total: f64,
    max: f64,
    count: u32,
}

impl std::fmt::Display for Timing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let avg = self.total / f64::from(self.count.max(1));
        write!(f, "avg {avg:.2}ms max {:.2}ms", self.max)
    }
}

impl FrameStats {
    fn from_env() -> Option<Self> {
        frame_stats_enabled().then(Self::new)
    }

    fn new() -> Self {
        Self {
            since: Instant::now(),
            frames: 0,
            draw_total: Duration::ZERO,
            draw_max: Duration::ZERO,
            last_frame: None,
            gap_max: Duration::ZERO,
            phases: Vec::new(),
        }
    }

    /// A frame was presented; its draw began at `start` and spent `phases`.
    fn frame(&mut self, start: Instant, phases: &Phases) {
        let now = Instant::now();
        let draw = now - start;
        self.frames += 1;
        self.draw_total += draw;
        self.draw_max = self.draw_max.max(draw);
        if let Some(last) = self.last_frame {
            self.gap_max = self.gap_max.max(now - last);
        }
        self.last_frame = Some(now);
        for &(name, ms) in phases {
            let index = match self.phases.iter().position(|(n, _)| *n == name) {
                Some(index) => index,
                None => {
                    self.phases.push((name, Timing::default()));
                    self.phases.len() - 1
                }
            };
            let timing = &mut self.phases[index].1;
            timing.total += ms;
            timing.max = timing.max.max(ms);
            timing.count += 1;
        }
    }

    /// Timings only: mpv properties are read by `spawn_pass_logger`, since the render
    /// thread must never wait on the mpv core (render.h, advanced control).
    fn maybe_log(&mut self, size: Extent) {
        let elapsed = self.since.elapsed();
        if elapsed < Duration::from_secs(5) {
            return;
        }
        if self.frames > 0 {
            crate::log::write(&self.summary(size, elapsed));
        }
        let last_frame = self.last_frame;
        *self = Self::new();
        self.last_frame = last_frame;
    }

    fn summary(&self, size: Extent, elapsed: Duration) -> String {
        let ms = |d: Duration| d.as_secs_f64() * 1000.;
        let phases: String = self
            .phases
            .iter()
            .map(|(name, timing)| format!(" | {name} {timing}"))
            .collect();
        format!(
            "frame stats: {}x{} | {} frames, {:.1} fps | draw avg {:.2}ms max {:.2}ms | longest gap {:.1}ms{phases}",
            size.width,
            size.height,
            self.frames,
            f64::from(self.frames) / elapsed.as_secs_f64(),
            ms(self.draw_total) / f64::from(self.frames),
            ms(self.draw_max),
            ms(self.gap_max),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Extent, FrameStats};
    use std::time::{Duration, Instant};

    #[test]
    fn renderer_phases_join_the_frame_stats_line() {
        let mut stats = FrameStats::new();
        let start = Instant::now();
        let (render, wait) = ("mpv GL render", "wait for drawable");
        stats.frame(start, &vec![(render, 2.), (wait, 1.)]);
        // a phase first reported later (copy GPU of the previous frame) is still kept
        stats.frame(start, &vec![(render, 4.), (wait, 3.), ("copy GPU", 0.5)]);
        let size = Extent {
            width: 1920,
            height: 1080,
            ..Extent::default()
        };
        let line = stats.summary(size, Duration::from_secs(5));
        let phases = line.split_once("longest gap").unwrap().1;
        let head = "frame stats: 1920x1080 | 2 frames, 0.4 fps";
        assert!(line.starts_with(head), "{line}");
        assert!(
            phases.ends_with(concat!(
                " | mpv GL render avg 3.00ms max 4.00ms",
                " | wait for drawable avg 2.00ms max 3.00ms",
                " | copy GPU avg 0.50ms max 0.50ms",
            )),
            "{line}"
        );
    }
}

pub(super) fn frame_stats_enabled() -> bool {
    std::env::var_os("TBIS_FRAME_STATS").is_some()
}

/// Logs `log_render_passes` every 5 seconds until `stop`. Off the render thread: with
/// advanced control, mpv answers `vo-passes` on the render thread, so asking from it
/// would deadlock.
fn spawn_pass_logger(mpv: Handle, stop: Arc<AtomicBool>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let tick = Duration::from_millis(100);
        let mut waited = Duration::ZERO;
        while !stop.load(Ordering::SeqCst) {
            std::thread::sleep(tick);
            waited += tick;
            if waited >= Duration::from_secs(5) {
                waited = Duration::ZERO;
                log_render_passes(mpv);
            }
        }
    })
}

/// GPU time of each render pass for fresh frames (`vo-passes`), with the source format
/// and the scalers in effect, so render settings can be compared.
fn log_render_passes(mpv: Handle) {
    let get = |name: &str| get_string(mpv, name).unwrap_or_else(|| "?".into());
    // only the whole node resolves; `vo-passes/fresh/N/...` sub-paths fail as strings
    let fresh =
        node::read(mpv, c"vo-passes", |passes| unsafe { fresh_passes(passes) }).unwrap_or_default();
    let total: f64 = fresh.iter().map(|(_, ms)| ms).sum();
    let passes: Vec<String> = fresh
        .iter()
        .map(|(desc, ms)| format!("{desc} {ms:.2}"))
        .collect();
    crate::log::write(&format!(
        "render passes: {}x{} {} {} | fps {} | dropped vo {} decoder {} | hwdec {} | scale {} dscale {} cscale {} | total {total:.2}ms | {}",
        get("video-params/w"),
        get("video-params/h"),
        get("video-params/pixelformat"),
        get("video-params/gamma"),
        get("container-fps"),
        get("frame-drop-count"),
        get("decoder-frame-drop-count"),
        get("hwdec-current"),
        get("scale"),
        get("dscale"),
        get("cscale"),
        passes.join("; "),
    ));
}

/// `(description, average ms)` of each pass in a `vo-passes` node's `fresh` list.
///
/// # Safety
/// `node` must be a node filled by `mpv_get_property` and not yet freed.
unsafe fn fresh_passes(node: &mpv_node) -> Vec<(String, f64)> {
    let Some(fresh) = (unsafe { node::get(node, "fresh") }) else {
        return Vec::new();
    };
    unsafe { node::list(fresh) }
        .iter()
        .map(|pass| {
            let desc = unsafe { node::string(pass, "desc") }.unwrap_or_default();
            let avg_ns = unsafe { node::int(pass, "avg") }.unwrap_or(0);
            (desc, avg_ns as f64 / 1e6)
        })
        .collect()
}

/// Wakes render thread when mpv has a new frame or the size changed.
#[derive(Default)]
pub(super) struct Waker {
    ready: Mutex<bool>,
    cv: Condvar,
}

impl Waker {
    pub(super) fn notify(&self) {
        *self.ready.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.cv.notify_one();
    }

    fn wait(&self, timeout: Duration) {
        let guard = self.ready.lock().unwrap_or_else(PoisonError::into_inner);
        let (mut guard, _) = self
            .cv
            .wait_timeout_while(guard, timeout, |ready| !*ready)
            .unwrap_or_else(PoisonError::into_inner);
        *guard = false;
    }
}

unsafe extern "C" fn on_render_update(ctx: *mut c_void) {
    unsafe { &*(ctx as *const Waker) }.notify();
}

pub(super) fn render_thread(
    mpv: Handle,
    mut renderer: video::Renderer,
    size: Arc<Mutex<Extent>>,
    waker: Arc<Waker>,
    stop: Arc<AtomicBool>,
    ready: mpsc::Sender<Result<(), String>>,
    events: UnboundedSender<MpvEvent>,
) {
    let ctx = match renderer.init(mpv) {
        Ok(ctx) => ctx,
        Err(err) => {
            crate::log::write(&format!("video initialization failed: {err}"));
            let _ = ready.send(Err(err));
            return;
        }
    };
    unsafe {
        mpv_render_context_set_update_callback(
            ctx,
            Some(on_render_update),
            Arc::as_ptr(&waker) as *mut c_void,
        )
    };
    let _ = ready.send(Ok(()));

    let mut drawn_size = Extent::default();
    let mut stats = FrameStats::from_env();
    let mut phases = Phases::new();
    let pass_logger = stats
        .is_some()
        .then(|| spawn_pass_logger(mpv, stop.clone()));
    while !stop.load(Ordering::SeqCst) {
        if let Some(stats) = &mut stats {
            stats.maybe_log(drawn_size);
        }
        waker.wait(Duration::from_millis(250));
        let flags = unsafe { mpv_render_context_update(ctx) };
        let new_frame = flags & mpv_render_update_flag_MPV_RENDER_UPDATE_FRAME as u64 != 0;
        let extent = *size.lock().unwrap_or_else(PoisonError::into_inner);
        if extent.width <= 0 || extent.height <= 0 {
            if new_frame {
                skip_frame(ctx); // drain, else mpv stalls waiting for render
            }
            continue;
        }
        let start = stats.is_some().then(Instant::now);
        let timed = stats.is_some().then(|| {
            phases.clear();
            &mut phases
        });
        let drawn = if new_frame || drawn_size != extent {
            match renderer.draw(ctx, extent, timed) {
                Ok(drawn) => drawn,
                Err(error) => {
                    skip_frame(ctx);
                    crate::log::write(&error);
                    let failed = MpvEvent::Player(PlayerEvent::RenderFailed(error));
                    let _ = events.unbounded_send(failed);
                    break;
                }
            }
        } else {
            false
        };
        if drawn {
            drawn_size = extent;
            unsafe { mpv_render_context_report_swap(ctx) };
            if let (Some(stats), Some(start)) = (&mut stats, start) {
                stats.frame(start, &phases);
            }
        }
    }

    unsafe {
        mpv_render_context_set_update_callback(ctx, None, std::ptr::null_mut());
        mpv_render_context_free(ctx);
    }
    if let Some(logger) = pass_logger {
        let _ = logger.join();
    }
    // renderer drops here, freeing its GL/GPU objects
}
