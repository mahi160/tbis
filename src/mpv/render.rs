//! mpv's render thread: drives the render loop and, with `TBIS_FRAME_STATS=1`, logs
//! frame pacing and render pass timings.

use super::{Extent, Handle, get_string, skip_frame};
use crate::platform::video;

use std::ffi::{CStr, c_void};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use libmpv_sys::*;

/// Frame pacing and drop counts, logged every 5 seconds with `TBIS_FRAME_STATS=1`.
struct FrameStats {
    since: Instant,
    frames: u32,
    draw_total: Duration,
    draw_max: Duration,
    last_frame: Option<Instant>,
    gap_max: Duration,
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
        }
    }

    /// A frame was presented; its draw began at `start`.
    fn frame(&mut self, start: Instant) {
        let now = Instant::now();
        let draw = now - start;
        self.frames += 1;
        self.draw_total += draw;
        self.draw_max = self.draw_max.max(draw);
        if let Some(last) = self.last_frame {
            self.gap_max = self.gap_max.max(now - last);
        }
        self.last_frame = Some(now);
    }

    /// Timings only: mpv properties are read by `spawn_pass_logger`, since the render
    /// thread must never wait on the mpv core (render.h, advanced control).
    fn maybe_log(&mut self, size: Extent) {
        let elapsed = self.since.elapsed();
        if elapsed < Duration::from_secs(5) {
            return;
        }
        if self.frames > 0 {
            let ms = |d: Duration| d.as_secs_f64() * 1000.;
            eprintln!(
                "frame stats: {}x{} | {} frames, {:.1} fps | draw avg {:.2}ms max {:.2}ms | longest gap {:.1}ms",
                size.width,
                size.height,
                self.frames,
                f64::from(self.frames) / elapsed.as_secs_f64(),
                ms(self.draw_total) / f64::from(self.frames),
                ms(self.draw_max),
                ms(self.gap_max),
            );
        }
        let last_frame = self.last_frame;
        *self = Self::new();
        self.last_frame = last_frame;
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
    let mut node: mpv_node = unsafe { std::mem::zeroed() };
    let rc = unsafe {
        mpv_get_property(
            mpv.0,
            c"vo-passes".as_ptr(),
            mpv_format_MPV_FORMAT_NODE,
            (&raw mut node).cast(),
        )
    };
    let fresh = if rc < 0 {
        Vec::new()
    } else {
        let fresh = unsafe { fresh_passes(&node) };
        unsafe { mpv_free_node_contents(&raw mut node) };
        fresh
    };
    let total: f64 = fresh.iter().map(|(_, ms)| ms).sum();
    let passes: Vec<String> = fresh
        .iter()
        .map(|(desc, ms)| format!("{desc} {ms:.2}"))
        .collect();
    eprintln!(
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
    );
}

/// `(description, average ms)` of each pass in a `vo-passes` node's `fresh` list.
///
/// # Safety
/// `node` must be a node filled by `mpv_get_property` and not yet freed.
unsafe fn fresh_passes(node: &mpv_node) -> Vec<(String, f64)> {
    let Some(fresh) = (unsafe { map_get(node, "fresh") }) else {
        return Vec::new();
    };
    unsafe { list_values(fresh) }
        .iter()
        .map(|pass| {
            let desc = unsafe { map_get(pass, "desc") }
                .filter(|n| n.format == mpv_format_MPV_FORMAT_STRING)
                .map(|n| {
                    unsafe { CStr::from_ptr(n.u.string) }
                        .to_string_lossy()
                        .into_owned()
                })
                .unwrap_or_default();
            let avg_ns = unsafe { map_get(pass, "avg") }
                .filter(|n| n.format == mpv_format_MPV_FORMAT_INT64)
                .map_or(0, |n| unsafe { n.u.int64 });
            (desc, avg_ns as f64 / 1e6)
        })
        .collect()
}

/// Items of an array or map node; empty for any other node.
///
/// # Safety
/// `node` must be a valid mpv node.
unsafe fn list_values(node: &mpv_node) -> &[mpv_node] {
    let is_list = node.format == mpv_format_MPV_FORMAT_NODE_ARRAY
        || node.format == mpv_format_MPV_FORMAT_NODE_MAP;
    let list = if is_list {
        unsafe { node.u.list.as_ref() }
    } else {
        None
    };
    match list {
        Some(l) if l.num > 0 && !l.values.is_null() => unsafe {
            std::slice::from_raw_parts(l.values, l.num as usize)
        },
        _ => &[],
    }
}

/// Value under `key` in a map node.
///
/// # Safety
/// `node` must be a valid mpv node.
unsafe fn map_get<'a>(node: &'a mpv_node, key: &str) -> Option<&'a mpv_node> {
    if node.format != mpv_format_MPV_FORMAT_NODE_MAP {
        return None;
    }
    let list = unsafe { node.u.list.as_ref() }?;
    let values = unsafe { list_values(node) };
    values.iter().enumerate().find_map(|(i, value)| {
        let k = unsafe { CStr::from_ptr(*list.keys.add(i)) };
        (k.to_bytes() == key.as_bytes()).then_some(value)
    })
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
    failure: Arc<Mutex<Option<String>>>,
) {
    let ctx = match renderer.init(mpv) {
        Ok(ctx) => ctx,
        Err(err) => {
            crate::platform::log::write(&format!("video initialization failed: {err}"));
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
        let start = Instant::now();
        let drawn = if new_frame || drawn_size != extent {
            match renderer.draw(ctx, extent) {
                Ok(drawn) => drawn,
                Err(error) => {
                    skip_frame(ctx);
                    crate::platform::log::write(&error);
                    *failure.lock().unwrap_or_else(PoisonError::into_inner) = Some(error);
                    unsafe { mpv_wakeup(mpv.0) }; // event thread forwards it
                    break;
                }
            }
        } else {
            false
        };
        if drawn {
            drawn_size = extent;
            unsafe { mpv_render_context_report_swap(ctx) };
            if let Some(stats) = &mut stats {
                stats.frame(start);
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
