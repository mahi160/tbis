//! In-process libmpv (ADR-0001). This file holds the platform-independent part: mpv handle,
//! options, commands, event thread, and the render loop driving mpv's OpenGL render API.
//! Each platform module supplies the on-screen surface and the GL/present side of a frame.

#[cfg_attr(target_os = "macos", path = "mac.rs")]
#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "windows", path = "windows.rs")]
mod platform;

use std::ffi::{CStr, CString, c_void};
use std::os::raw::{c_char, c_int};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

use futures::channel::mpsc::UnboundedSender;
use libmpv_sys::*;
use raw_window_handle::HasWindowHandle;

pub enum MpvEvent {
    TimePos(f64),
    Duration(f64),
    Pause(bool),
    FileLoaded,
    /// `Ok(true)` reached end of file, `Ok(false)` stopped otherwise.
    EndFile(Result<bool, String>),
    /// 0–100.
    Volume(f64),
    Mute(bool),
    /// Audio and subtitle tracks of the current file.
    Tracks(Vec<Track>),
}

#[derive(Clone, Copy, PartialEq)]
pub enum TrackKind {
    Audio,
    Subtitle,
}

impl TrackKind {
    fn property(self) -> &'static str {
        match self {
            TrackKind::Audio => "aid",
            TrackKind::Subtitle => "sid",
        }
    }
}

#[derive(Clone)]
pub struct Track {
    /// mpv's own track id for `aid`/`sid`.
    pub id: i64,
    pub kind: TrackKind,
    pub label: String,
    pub selected: bool,
}

pub struct Mpv {
    handle: Handle,
    surface: Option<platform::Surface>,
    size: Arc<Mutex<(i32, i32)>>,
    waker: Arc<Waker>,
    stop: Arc<AtomicBool>,
    events: Option<JoinHandle<()>>,
    render: Option<JoinHandle<()>>,
}

impl Mpv {
    /// Must run on the main thread. `auth_header` goes on every HTTP request mpv makes.
    pub fn new(
        window: &impl HasWindowHandle,
        auth_header: &str,
        events: UnboundedSender<MpvEvent>,
    ) -> Result<Self, String> {
        let handle = Handle(unsafe { mpv_create() });
        if handle.0.is_null() {
            return Err("mpv_create failed".into());
        }
        if let Err(err) = handle.init(auth_header) {
            unsafe { mpv_terminate_destroy(handle.0) };
            return Err(err);
        }
        let (surface, renderer) = match platform::Surface::attach(window) {
            Ok(parts) => parts,
            Err(err) => {
                unsafe { mpv_terminate_destroy(handle.0) };
                return Err(err);
            }
        };

        let size = Arc::new(Mutex::new((0, 0)));
        let waker = Arc::new(Waker::default());
        let stop = Arc::new(AtomicBool::new(false));

        let (ready_tx, ready_rx) = mpsc::channel();
        let render = {
            let (size, waker, stop) = (size.clone(), waker.clone(), stop.clone());
            std::thread::spawn(move || render_thread(handle, renderer, size, waker, stop, ready_tx))
        };
        let events = {
            let stop = stop.clone();
            std::thread::spawn(move || event_thread(handle, stop, events))
        };

        let this = Self {
            handle,
            surface: Some(surface),
            size,
            waker,
            stop,
            events: Some(events),
            render: Some(render),
        };
        // on Err, drop tears everything down in order
        ready_rx
            .recv()
            .unwrap_or_else(|_| Err("render thread died".into()))
            .map(|()| this)
    }

    /// Replaces current file; starts at `start_seconds`.
    pub fn load(&self, url: &str, start_seconds: f64) -> Result<(), String> {
        let start = if start_seconds > 0.0 {
            format!("{start_seconds}")
        } else {
            "none".into()
        };
        self.handle.set_property("start", &start)?;
        self.command(&["loadfile", url, "replace"])
    }

    pub fn set_pause(&self, pause: bool) -> Result<(), String> {
        self.handle
            .set_property("pause", if pause { "yes" } else { "no" })
    }

    pub fn seek(&self, seconds: f64) -> Result<(), String> {
        self.command(&["seek", &format!("{seconds}"), "absolute"])
    }

    pub fn seek_by(&self, seconds: f64) -> Result<(), String> {
        self.command(&["seek", &format!("{seconds}"), "relative"])
    }

    /// 0–100.
    pub fn set_volume(&self, volume: f64) -> Result<(), String> {
        self.handle
            .set_property("volume", &format!("{}", volume.clamp(0., 100.)))
    }

    pub fn set_mute(&self, mute: bool) -> Result<(), String> {
        self.handle
            .set_property("mute", if mute { "yes" } else { "no" })
    }

    /// `None` turns the track type off.
    pub fn select_track(&self, kind: TrackKind, id: Option<i64>) -> Result<(), String> {
        let value = id.map_or("no".to_string(), |id| id.to_string());
        self.handle.set_property(kind.property(), &value)
    }

    /// Video size in device pixels.
    pub fn set_size(&self, width: i32, height: i32) {
        *self.size.lock().unwrap() = (width, height);
        self.waker.notify();
    }

    fn command(&self, args: &[&str]) -> Result<(), String> {
        self.handle.command(args)
    }
}

impl Drop for Mpv {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        unsafe { mpv_wakeup(self.handle.0) };
        self.waker.notify();
        if let Some(thread) = self.events.take() {
            let _ = thread.join();
        }
        // render thread frees render context; must precede mpv_terminate_destroy
        if let Some(thread) = self.render.take() {
            let _ = thread.join();
        }
        drop(self.surface.take());
        unsafe { mpv_terminate_destroy(self.handle.0) };
    }
}

#[derive(Clone, Copy)]
struct Handle(*mut mpv_handle);

// mpv client API is thread-safe; Mpv::drop joins every thread before destroying handle
unsafe impl Send for Handle {}

impl Handle {
    fn init(self, auth_header: &str) -> Result<(), String> {
        for (name, value) in [
            ("vo", "libmpv"),
            ("hwdec", "auto-safe"),
            ("terminal", "no"),
            ("input-default-bindings", "no"),
            ("input-vo-keyboard", "no"),
            ("osc", "no"),
            ("ytdl", "no"), // plain Jellyfin URLs; skip youtube-dl hook
        ] {
            let (n, v) = cstrings(name, value)?;
            unsafe { check(mpv_set_option_string(self.0, n.as_ptr(), v.as_ptr()), name)? };
        }
        unsafe {
            mpv_request_log_messages(self.0, c"warn".as_ptr());
            check(mpv_initialize(self.0), "mpv_initialize")?;
        }
        // change-list: one verbatim item; plain option would split header on its commas
        let header = format!("Authorization: {auth_header}");
        self.command(&["change-list", "http-header-fields", "append", &header])?;
        unsafe {
            mpv_observe_property(
                self.0,
                0,
                c"time-pos".as_ptr(),
                mpv_format_MPV_FORMAT_DOUBLE,
            );
            mpv_observe_property(
                self.0,
                0,
                c"duration".as_ptr(),
                mpv_format_MPV_FORMAT_DOUBLE,
            );
            mpv_observe_property(self.0, 0, c"pause".as_ptr(), mpv_format_MPV_FORMAT_FLAG);
            mpv_observe_property(self.0, 0, c"volume".as_ptr(), mpv_format_MPV_FORMAT_DOUBLE);
            mpv_observe_property(self.0, 0, c"mute".as_ptr(), mpv_format_MPV_FORMAT_FLAG);
            // no payload: tracks are re-read from sub-properties on change
            mpv_observe_property(
                self.0,
                0,
                c"track-list".as_ptr(),
                mpv_format_MPV_FORMAT_NONE,
            );
            // known only once decoder is up; first thing any stutter report needs
            mpv_observe_property(
                self.0,
                0,
                c"hwdec-current".as_ptr(),
                mpv_format_MPV_FORMAT_STRING,
            );
        }
        Ok(())
    }

    fn command(self, args: &[&str]) -> Result<(), String> {
        let args: Vec<CString> = args
            .iter()
            .map(|a| CString::new(*a).map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()?;
        let mut ptrs: Vec<*const c_char> = args.iter().map(|a| a.as_ptr()).collect();
        ptrs.push(std::ptr::null());
        unsafe { check(mpv_command(self.0, ptrs.as_mut_ptr()), "mpv_command") }
    }

    fn set_property(self, name: &str, value: &str) -> Result<(), String> {
        let (n, v) = cstrings(name, value)?;
        unsafe {
            check(
                mpv_set_property_string(self.0, n.as_ptr(), v.as_ptr()),
                name,
            )
        }
    }
}

fn cstrings(a: &str, b: &str) -> Result<(CString, CString), String> {
    Ok((
        CString::new(a).map_err(|e| e.to_string())?,
        CString::new(b).map_err(|e| e.to_string())?,
    ))
}

fn check(rc: c_int, what: &str) -> Result<(), String> {
    if rc < 0 {
        let msg = unsafe { CStr::from_ptr(mpv_error_string(rc)) }.to_string_lossy();
        return Err(format!("mpv {what}: {msg} ({rc})"));
    }
    Ok(())
}

#[allow(non_upper_case_globals)] // bindgen constant names
fn event_thread(mpv: Handle, stop: Arc<AtomicBool>, tx: UnboundedSender<MpvEvent>) {
    let send = |event| {
        let _ = tx.unbounded_send(event);
    };
    while !stop.load(Ordering::SeqCst) {
        let event = unsafe { &*mpv_wait_event(mpv.0, -1.0) };
        match event.event_id {
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
                    b"time-pos" => send(MpvEvent::TimePos(unsafe { *(prop.data as *const f64) })),
                    b"duration" => send(MpvEvent::Duration(unsafe { *(prop.data as *const f64) })),
                    b"pause" => send(MpvEvent::Pause(
                        unsafe { *(prop.data as *const c_int) } != 0,
                    )),
                    b"hwdec-current" => {
                        let value = unsafe { CStr::from_ptr(*(prop.data as *const *const c_char)) };
                        eprintln!("mpv: hwdec-current={}", value.to_string_lossy());
                    }
                    _ => {}
                }
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
                eprintln!("mpv [{prefix}] {}", text.trim_end());
            }
            _ => {}
        }
    }
}

fn get_string(mpv: Handle, name: &str) -> Option<String> {
    let name = CString::new(name).ok()?;
    let value = unsafe { mpv_get_property_string(mpv.0, name.as_ptr()) };
    if value.is_null() {
        return None;
    }
    let text = unsafe { CStr::from_ptr(value) }
        .to_string_lossy()
        .into_owned();
    unsafe { mpv_free(value as *mut c_void) };
    Some(text)
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
            let mut parts: Vec<String> = [field("title"), field("lang"), field("codec")]
                .into_iter()
                .flatten()
                .collect();
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
                selected: field("selected").as_deref() == Some("yes"),
            })
        })
        .collect()
}

/// Wakes render thread when mpv has a new frame or the size changed.
#[derive(Default)]
struct Waker {
    ready: Mutex<bool>,
    cv: Condvar,
}

impl Waker {
    fn notify(&self) {
        *self.ready.lock().unwrap() = true;
        self.cv.notify_one();
    }

    fn wait(&self, timeout: Duration) {
        let guard = self.ready.lock().unwrap();
        let (mut guard, _) = self
            .cv
            .wait_timeout_while(guard, timeout, |ready| !*ready)
            .unwrap();
        *guard = false;
    }
}

unsafe extern "C" fn on_render_update(ctx: *mut c_void) {
    unsafe { &*(ctx as *const Waker) }.notify();
}

/// mpv render context for the OpenGL API, created with the platform's GL context current.
type GetProcAddress = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void;

fn create_gl_render_context(
    mpv: Handle,
    get_proc_address: GetProcAddress,
) -> Result<*mut mpv_render_context, String> {
    let mut init = mpv_opengl_init_params {
        get_proc_address: Some(get_proc_address),
        get_proc_address_ctx: std::ptr::null_mut(),
        extra_exts: std::ptr::null(),
    };
    let mut params = [
        param(
            mpv_render_param_type_MPV_RENDER_PARAM_API_TYPE,
            c"opengl".as_ptr() as *mut c_void,
        ),
        param(
            mpv_render_param_type_MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
            &mut init as *mut _ as *mut c_void,
        ),
        param(
            mpv_render_param_type_MPV_RENDER_PARAM_INVALID,
            std::ptr::null_mut(),
        ),
    ];
    let mut ctx = std::ptr::null_mut();
    unsafe {
        check(
            mpv_render_context_create(&mut ctx, mpv.0, params.as_mut_ptr()),
            "render context",
        )?
    };
    Ok(ctx)
}

fn param(type_: mpv_render_param_type, data: *mut c_void) -> mpv_render_param {
    mpv_render_param { type_, data }
}

/// Renders into an OpenGL framebuffer `fbo` of size `w`×`h`. Returns false if no frame was drawn.
fn render_gl(ctx: *mut mpv_render_context, fbo: c_int, w: i32, h: i32) -> bool {
    let mut target = mpv_opengl_fbo {
        fbo,
        w,
        h,
        internal_format: 0,
    };
    let mut params = [
        param(
            mpv_render_param_type_MPV_RENDER_PARAM_OPENGL_FBO,
            &mut target as *mut _ as *mut c_void,
        ),
        param(
            mpv_render_param_type_MPV_RENDER_PARAM_INVALID,
            std::ptr::null_mut(),
        ),
    ];
    unsafe { mpv_render_context_render(ctx, params.as_mut_ptr()) >= 0 }
}

fn skip_frame(ctx: *mut mpv_render_context) {
    let mut skip: c_int = 1;
    let mut params = [
        param(
            mpv_render_param_type_MPV_RENDER_PARAM_SKIP_RENDERING,
            &mut skip as *mut _ as *mut c_void,
        ),
        param(
            mpv_render_param_type_MPV_RENDER_PARAM_INVALID,
            std::ptr::null_mut(),
        ),
    ];
    unsafe { mpv_render_context_render(ctx, params.as_mut_ptr()) };
}

fn render_thread(
    mpv: Handle,
    mut renderer: platform::Renderer,
    size: Arc<Mutex<(i32, i32)>>,
    waker: Arc<Waker>,
    stop: Arc<AtomicBool>,
    ready: mpsc::Sender<Result<(), String>>,
) {
    let ctx = match renderer.init(mpv) {
        Ok(ctx) => ctx,
        Err(err) => {
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

    let mut drawn_size = (0, 0);
    while !stop.load(Ordering::SeqCst) {
        waker.wait(Duration::from_millis(250));
        let flags = unsafe { mpv_render_context_update(ctx) };
        let new_frame = flags & mpv_render_update_flag_MPV_RENDER_UPDATE_FRAME as u64 != 0;
        let (w, h) = *size.lock().unwrap();
        if w <= 0 || h <= 0 {
            if new_frame {
                skip_frame(ctx); // drain, else mpv stalls waiting for render
            }
            continue;
        }
        if (new_frame || drawn_size != (w, h)) && renderer.draw(ctx, w, h) {
            drawn_size = (w, h);
            unsafe { mpv_render_context_report_swap(ctx) };
        }
    }

    unsafe {
        mpv_render_context_set_update_callback(ctx, None, std::ptr::null_mut());
        mpv_render_context_free(ctx);
    }
    // renderer drops here, freeing its GL/GPU objects
}
