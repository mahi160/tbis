//! In-process libmpv (ADR-0001). This file holds the part shared by every platform
//! module: mpv handle, options and commands; `events` runs the event thread and `render`
//! the loop driving mpv's OpenGL render API. `platform::video` supplies the on-screen surface and the
//! GL/present side of a frame; it calls back into `create_gl_render_context` and
//! `render_gl` here.

mod events;
mod node;
mod render;

use crate::platform::video;
use events::event_thread;
pub use render::Phases;
use render::{Waker, frame_stats_enabled, render_thread};

use std::ffi::{CStr, CString, c_void};
use std::os::raw::{c_char, c_int};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::thread::JoinHandle;

use futures::channel::mpsc::UnboundedSender;
use libmpv_sys::*;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

/// Where the video goes: device pixels, plus the window's scale for platforms whose
/// compositor wants the logical size too.
#[derive(Clone, Copy, PartialEq, Default)]
pub struct Extent {
    pub width: i32,
    pub height: i32,
    pub scale: f32,
}

/// What mpv reports: player-wide, or tied to the playback load (`Mpv::load`'s id) it
/// came from, so a late event from a replaced file can be told apart.
pub enum MpvEvent {
    Player(PlayerEvent),
    File { load: i64, event: FileEvent },
}

impl MpvEvent {
    /// Whether the Player, whose current load is `load`, should act on it.
    pub fn is_current(&self, load: Option<i64>) -> bool {
        match self {
            Self::Player(_) => true,
            Self::File { load: from, .. } => load == Some(*from),
        }
    }
}

/// Not tied to any file.
pub enum PlayerEvent {
    /// 0–100.
    Volume(f64),
    Mute(bool),
    /// Video can no longer be drawn; the render thread has stopped.
    RenderFailed(String),
}

/// Belongs to one playback load.
pub enum FileEvent {
    TimePos(f64),
    Duration(f64),
    Pause(bool),
    /// mpv's `core-idle`: not advancing (paused, buffering, seeking, opening).
    CoreIdle(bool),
    FileLoaded,
    /// `Ok(true)` reached end of file, `Ok(false)` stopped otherwise.
    EndFile(Result<bool, String>),
    /// Audio and subtitle tracks of the current file.
    Tracks(Vec<Track>),
    SubtitleAdded(Result<(), String>),
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
    /// Primary selection (`aid`/`sid`).
    pub selected: bool,
    /// Selected as the secondary subtitle (`secondary-sid`); never also `selected`.
    pub secondary: bool,
    /// Raw language code (container tag, or the `sub-add` lang for an external
    /// subtitle); for matching a remembered language pref, separately from `label`.
    pub lang: Option<String>,
}

/// Sets the video's size without borrowing `Mpv`.
#[derive(Clone)]
pub struct VideoSize {
    size: Arc<Mutex<Extent>>,
    waker: Arc<Waker>,
}

impl VideoSize {
    /// Video size in device pixels, and the window scale it was derived with.
    /// Wakes the render thread only when the size actually changed.
    pub fn set(&self, width: i32, height: i32, scale: f32) {
        let extent = Extent {
            width,
            height,
            scale,
        };
        let mut size = self.size.lock().unwrap_or_else(PoisonError::into_inner);
        if *size == extent {
            return;
        }
        *size = extent;
        drop(size);
        self.waker.notify();
    }
}

pub struct Mpv {
    handle: Handle,
    surface: Option<video::Surface>,
    size: Arc<Mutex<Extent>>,
    waker: Arc<Waker>,
    stop: Arc<AtomicBool>,
    events: Option<JoinHandle<()>>,
    render: Option<JoinHandle<()>>,
}

impl Mpv {
    /// Must run on the main thread. `auth_header` goes on every HTTP request mpv makes.
    /// `defaults`: options from tbis Settings, set before the user's mpv.conf loads so
    /// mpv.conf overrides them.
    pub fn new(
        window: &(impl HasWindowHandle + HasDisplayHandle),
        auth_header: &str,
        defaults: &[(&str, String)],
        events: UnboundedSender<MpvEvent>,
    ) -> Result<Self, String> {
        let handle = Handle(unsafe { mpv_create() });
        if handle.0.is_null() {
            return Err("mpv_create failed".into());
        }
        if let Err(err) = handle.init(auth_header, defaults) {
            unsafe { mpv_terminate_destroy(handle.0) };
            return Err(err);
        }
        let (surface, renderer) = match video::Surface::attach(window) {
            Ok(parts) => parts,
            Err(err) => {
                crate::log::write(&format!("video surface attachment failed: {err}"));
                unsafe { mpv_terminate_destroy(handle.0) };
                return Err(err);
            }
        };

        let size = Arc::new(Mutex::new(Extent::default()));
        let waker = Arc::new(Waker::default());
        let stop = Arc::new(AtomicBool::new(false));

        let (ready_tx, ready_rx) = mpsc::channel();
        let render = {
            let (size, waker, stop, events) =
                (size.clone(), waker.clone(), stop.clone(), events.clone());
            std::thread::spawn(move || {
                render_thread(handle, renderer, size, waker, stop, ready_tx, events)
            })
        };
        let events = {
            let stop = stop.clone();
            let secrets = crate::log::header_secrets(auth_header);
            std::thread::spawn(move || event_thread(handle, stop, events, secrets))
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
    pub fn load(&self, url: &str, start_seconds: f64) -> Result<i64, String> {
        let start = if start_seconds > 0.0 {
            format!("{start_seconds}")
        } else {
            "none".into()
        };
        self.handle.set_property("start", &start)?;
        self.handle.command(&["loadfile", url, "replace"])?;
        get_string(self.handle, "playlist/0/id")
            .and_then(|id| id.parse().ok())
            .ok_or_else(|| "mpv: cannot identify loaded playlist entry".into())
    }

    /// Ends current file; mpv goes idle until next load.
    pub fn stop(&self) -> Result<(), String> {
        self.handle.command(&["stop"])
    }

    pub fn set_pause(&self, pause: bool) -> Result<(), String> {
        self.handle.set_property("pause", yes_no(pause))
    }

    pub fn seek(&self, seconds: f64) -> Result<(), String> {
        self.handle
            .command(&["seek", &format!("{seconds}"), "absolute"])
    }

    pub fn seek_by(&self, seconds: f64) -> Result<(), String> {
        self.handle
            .command(&["seek", &format!("{seconds}"), "relative"])
    }

    pub fn frame_step(&self, forward: bool) -> Result<(), String> {
        self.handle.command(&[if forward {
            "frame-step"
        } else {
            "frame-back-step"
        }])
    }

    /// 0–100.
    pub fn set_volume(&self, volume: f64) -> Result<(), String> {
        self.handle
            .set_property("volume", &format!("{}", volume.clamp(0., 100.)))
    }

    /// Playback rate; 1 is normal.
    pub fn set_speed(&self, speed: f64) -> Result<(), String> {
        self.handle.set_property("speed", &format!("{speed}"))
    }

    pub fn set_mute(&self, mute: bool) -> Result<(), String> {
        self.handle.set_property("mute", yes_no(mute))
    }

    /// `None` turns the track type off.
    pub fn select_track(&self, kind: TrackKind, id: Option<i64>) -> Result<(), String> {
        let value = id.map_or("no".to_string(), |id| id.to_string());
        self.handle.set_property(kind.property(), &value)
    }

    /// Second subtitle shown alongside the primary; `None` turns it off.
    pub fn select_secondary_subtitle(&self, id: Option<i64>) -> Result<(), String> {
        let value = id.map_or("no".to_string(), |id| id.to_string());
        self.handle.set_property("secondary-sid", &value)
    }

    /// Shows the video in `window` instead (Picture-in-Picture); playback carries on.
    /// Must run on the main thread.
    pub fn move_to(
        &mut self,
        window: &(impl HasWindowHandle + HasDisplayHandle),
    ) -> Result<(), String> {
        let surface = self.surface.as_mut().ok_or("no video surface")?;
        surface.move_to(window)?;
        self.waker.notify();
        Ok(())
    }

    /// Handle that sizes the video; whichever window shows it owns one.
    pub fn video_size(&self) -> VideoSize {
        VideoSize {
            size: self.size.clone(),
            waker: self.waker.clone(),
        }
    }

    /// Loads an external (sidecar) text subtitle not muxed into the file mpv opened.
    /// Deferred by the caller to `FileLoaded`: issued before the core opens a file,
    /// `sub-add` has no current file to attach the track to.
    pub fn add_subtitle(
        &self,
        url: &str,
        title: &str,
        lang: &str,
        select: bool,
        load: i64,
    ) -> Result<(), String> {
        let flag = if select { "select" } else { "auto" };
        // async: sub-add downloads url; sync call froze UI on slow server. The reply's
        // userdata is the load, so the event thread can tie it to its file unaided.
        self.handle
            .command_async(load as u64, &["sub-add", url, flag, title, lang])
    }

    /// Nudges subtitle/audio timing by `delta` seconds (mpv's own `add <prop>` idiom).
    pub fn adjust_sub_delay(&self, delta: f64) -> Result<(), String> {
        self.handle
            .command(&["add", "sub-delay", &format!("{delta}")])
    }

    pub fn adjust_audio_delay(&self, delta: f64) -> Result<(), String> {
        self.handle
            .command(&["add", "audio-delay", &format!("{delta}")])
    }
}

impl Mpv {
    /// Current frame with subtitles, at video resolution, to `path` (format from extension).
    pub fn screenshot(&self, path: &str) -> Result<(), String> {
        self.handle
            .command(&["screenshot-to-file", path, "subtitles"])
    }

    /// Sets any property by name from its string form, e.g. `sub-scale` = `1.25`.
    pub fn set_property(&self, name: &str, value: &str) -> Result<(), String> {
        self.handle.set_property(name, value)
    }

    /// Any property as mpv formats it (e.g. `video-codec`); `None` while unavailable.
    pub fn property(&self, name: &str) -> Option<String> {
        get_string(self.handle, name)
    }
}

/// mpv's yes/no spelling for a bool property.
fn yes_no(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

#[cfg(target_os = "windows")]
pub(crate) fn self_test() -> Result<(), String> {
    let handle = Handle(unsafe { mpv_create() });
    if handle.0.is_null() {
        return Err("self-test: mpv_create failed".into());
    }
    let result = (|| {
        unsafe {
            check(
                mpv_set_option_string(handle.0, c"config".as_ptr(), c"no".as_ptr()),
                "config",
            )?;
            check(
                mpv_set_option_string(handle.0, c"terminal".as_ptr(), c"no".as_ptr()),
                "terminal",
            )?;
            check(mpv_initialize(handle.0), "self-test mpv_initialize")?;
        }
        let version = get_string(handle, "mpv-version").ok_or("self-test: missing mpv version")?;
        crate::log::write(&format!(
            "self-test: {version}; client API={:#x}; no GPU test performed",
            unsafe { mpv_client_api_version() }
        ));
        Ok(())
    })();
    unsafe { mpv_terminate_destroy(handle.0) };
    result
}

impl Drop for Mpv {
    fn drop(&mut self) {
        let decoder = get_string(self.handle, "hwdec-current");
        let decoder = decoder.as_deref().unwrap_or("unknown");
        let dropped =
            get_string(self.handle, "frame-drop-count").and_then(|value| value.parse::<u64>().ok());
        let decoder_dropped = get_string(self.handle, "decoder-frame-drop-count")
            .and_then(|value| value.parse::<u64>().ok());
        crate::log::write(&format!(
            "player session closed: decoder={decoder}, VO frames dropped={dropped:?}, decoder frames dropped={decoder_dropped:?}"
        ));
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
        self.surface = None;
        unsafe { mpv_terminate_destroy(self.handle.0) };
    }
}

/// Read-ahead buffer for streaming from the server: rides out network hiccups and makes short back-seeks instant. Capped so a long
/// 4K remux can't grow memory unbounded. Read-ahead stops at 60 s, so low-bitrate
/// streams stay well under the cap.
const CACHE_OPTIONS: [(&str, &str); 4] = [
    ("cache", "yes"),
    ("demuxer-max-bytes", "400MiB"),
    ("demuxer-max-back-bytes", "100MiB"),
    // with cache on, mpv's huge default cache-secs overrides demuxer-readahead-secs
    ("cache-secs", "60"),
];

/// Options the in-window video path depends on (ADR-0001: libmpv render API, our
/// own controls and keys); a user mpv.conf may not change them.
const EMBEDDING: [(&str, &str); 4] = [
    ("vo", "libmpv"),
    ("terminal", "no"),
    ("input-default-bindings", "no"),
    ("input-vo-keyboard", "no"),
];

/// Part of `EMBEDDING` where it exists: libmpv built without Lua (Windows bundle) has no
/// OSC, hence no `osc` option.
const NO_OSC: (&str, &str) = ("osc", "no");

/// `<app support dir>/mpv` (`support_dir.rs`) when it holds an `mpv.conf`: the user's
/// own mpv config for the Player (scripts/shaders it names resolve there too).
/// Absent: mpv runs exactly as without it.
fn user_config_dir() -> Option<std::path::PathBuf> {
    let path = user_config_path();
    let dir = path.parent()?.to_path_buf();
    path.is_file().then_some(dir)
}

fn user_config_path() -> std::path::PathBuf {
    crate::support_dir::app_support_dir()
        .join("mpv")
        .join("mpv.conf")
}

/// The user's mpv.conf text (Settings), empty when there is none.
pub fn user_config() -> String {
    std::fs::read_to_string(user_config_path()).unwrap_or_default()
}

/// Saves the user's mpv.conf; blank text removes it, so mpv runs on its defaults.
pub fn save_user_config(text: &str) -> std::io::Result<()> {
    let path = user_config_path();
    if text.trim().is_empty() {
        return match std::fs::remove_file(&path) {
            Err(err) if err.kind() != std::io::ErrorKind::NotFound => Err(err),
            _ => Ok(()),
        };
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, text)
}

#[derive(Clone, Copy)]
pub(crate) struct Handle(*mut mpv_handle);

// mpv client API is thread-safe; Mpv::drop joins every thread before destroying handle
unsafe impl Send for Handle {}

impl Handle {
    fn init(self, auth_header: &str, defaults: &[(&str, String)]) -> Result<(), String> {
        let user_config = user_config_dir();
        let mut options: Vec<(&str, String)> = EMBEDDING
            .iter()
            .map(|(name, value)| (*name, value.to_string()))
            .collect();
        options.extend(defaults.iter().map(|(name, value)| (*name, value.clone())));
        options.extend([
            ("hwdec", video::HWDEC.to_string()),
            ("ytdl", "no".to_string()), // plain Jellyfin URLs; skip youtube-dl hook
            ("sub-font", crate::fonts::FAMILY.to_string()),
        ]);
        if let Some(dir) = &user_config {
            options.push(("config-dir", dir.to_string_lossy().into_owned()));
            options.push(("config", "yes".to_string()));
        }
        options.extend(CACHE_OPTIONS.map(|(name, value)| (name, value.to_string())));
        if let Some(dir) = crate::fonts::extract_dir() {
            options.push(("sub-fonts-dir", dir.to_string_lossy().into_owned()));
        }
        for (name, value) in &options {
            check(self.set_option(name, value)?, name)?;
        }
        let rc = self.set_option(NO_OSC.0, NO_OSC.1)?;
        let has_osc = rc != mpv_error_MPV_ERROR_OPTION_NOT_FOUND;
        if has_osc {
            check(rc, NO_OSC.0)?;
        }
        unsafe {
            // info: decoder, hwdec and audio-output choices, needed to diagnose a failed start
            mpv_request_log_messages(self.0, c"info".as_ptr());
            check(mpv_initialize(self.0), "mpv_initialize")?;
        }
        // mpv.conf loads during initialize, over the options above; take these back
        if user_config.is_some() {
            for (name, value) in EMBEDDING.into_iter().chain(has_osc.then_some(NO_OSC)) {
                if let Err(err) = self.set_property(name, value) {
                    crate::log::write(&format!(
                        "mpv: re-applying {name} over user mpv.conf failed: {err}"
                    ));
                }
            }
        }
        // change-list: one verbatim item; plain option would split header on its commas
        let header = format!("Authorization: {auth_header}");
        self.command(&["change-list", "http-header-fields", "append", &header])?;
        // no payload for track-list: tracks are re-read as one node on change.
        // hwdec-current is known only once the decoder is up; first thing any stutter
        // report needs.
        for (name, format) in [
            ("time-pos", mpv_format_MPV_FORMAT_DOUBLE),
            ("duration", mpv_format_MPV_FORMAT_DOUBLE),
            ("pause", mpv_format_MPV_FORMAT_FLAG),
            ("core-idle", mpv_format_MPV_FORMAT_FLAG),
            ("volume", mpv_format_MPV_FORMAT_DOUBLE),
            ("mute", mpv_format_MPV_FORMAT_FLAG),
            ("track-list", mpv_format_MPV_FORMAT_NONE),
            ("hwdec-current", mpv_format_MPV_FORMAT_STRING),
        ] {
            let name = CString::new(name).map_err(|e| e.to_string())?;
            unsafe { mpv_observe_property(self.0, 0, name.as_ptr(), format) };
        }
        Ok(())
    }

    fn command(self, args: &[&str]) -> Result<(), String> {
        let args = c_args(args)?;
        let mut ptrs = c_ptrs(&args);
        unsafe { check(mpv_command(self.0, ptrs.as_mut_ptr()), "mpv_command") }
    }

    /// Returns once queued; the result arrives as `MPV_EVENT_COMMAND_REPLY` with `userdata`
    /// on the event thread.
    fn command_async(self, userdata: u64, args: &[&str]) -> Result<(), String> {
        let args = c_args(args)?;
        let mut ptrs = c_ptrs(&args);
        unsafe {
            check(
                mpv_command_async(self.0, userdata, ptrs.as_mut_ptr()),
                "mpv_command_async",
            )
        }
    }

    /// mpv's status code, for the caller to `check`; before `mpv_initialize` only.
    fn set_option(self, name: &str, value: &str) -> Result<c_int, String> {
        let (n, v) = cstrings(name, value)?;
        Ok(unsafe { mpv_set_option_string(self.0, n.as_ptr(), v.as_ptr()) })
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

fn c_args(args: &[&str]) -> Result<Vec<CString>, String> {
    args.iter()
        .map(|a| CString::new(*a).map_err(|e| e.to_string()))
        .collect()
}

/// Null-terminated argv borrowing `args`.
fn c_ptrs(args: &[CString]) -> Vec<*const c_char> {
    let mut ptrs: Vec<*const c_char> = args.iter().map(|a| a.as_ptr()).collect();
    ptrs.push(std::ptr::null());
    ptrs
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

/// mpv render context for the OpenGL API, created with the platform's GL context current.
pub(crate) type GetProcAddress = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void;

/// Display connection hardware-decode interop needs on X11/Wayland: the render param
/// (`MPV_RENDER_PARAM_WL_DISPLAY` / `_X11_DISPLAY`) and the native pointer. `None` elsewhere.
pub(crate) type NativeDisplay = Option<(mpv_render_param_type, *mut c_void)>;

pub(crate) fn create_gl_render_context(
    mpv: Handle,
    get_proc_address: GetProcAddress,
    native_display: NativeDisplay,
) -> Result<*mut mpv_render_context, String> {
    let mut init = mpv_opengl_init_params {
        get_proc_address: Some(get_proc_address),
        get_proc_address_ctx: std::ptr::null_mut(),
        extra_exts: std::ptr::null(),
    };
    let mut params = vec![
        param(
            mpv_render_param_type_MPV_RENDER_PARAM_API_TYPE,
            c"opengl".as_ptr() as *mut c_void,
        ),
        param(
            mpv_render_param_type_MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
            &mut init as *mut _ as *mut c_void,
        ),
    ];
    params.extend(native_display.map(|(kind, display)| param(kind, display)));
    // vo-passes (render pass timings) is only filled with advanced control; kept to
    // TBIS_FRAME_STATS so normal playback's render loop contract is unchanged
    let mut advanced: c_int = 1;
    if frame_stats_enabled() {
        params.push(param(
            mpv_render_param_type_MPV_RENDER_PARAM_ADVANCED_CONTROL,
            &mut advanced as *mut _ as *mut c_void,
        ));
    }
    params.push(param(
        mpv_render_param_type_MPV_RENDER_PARAM_INVALID,
        std::ptr::null_mut(),
    ));
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

/// Renders into an OpenGL framebuffer `fbo` of size `w`×`h`; `flip_y` for the default
/// framebuffer (fbo 0), whose origin is bottom-left. Returns false if no frame was drawn.
pub(crate) fn render_gl(
    ctx: *mut mpv_render_context,
    fbo: c_int,
    w: i32,
    h: i32,
    flip_y: bool,
) -> bool {
    let mut target = mpv_opengl_fbo {
        fbo,
        w,
        h,
        internal_format: 0,
    };
    let mut flip: c_int = flip_y.into();
    let mut params = [
        param(
            mpv_render_param_type_MPV_RENDER_PARAM_OPENGL_FBO,
            &mut target as *mut _ as *mut c_void,
        ),
        param(
            mpv_render_param_type_MPV_RENDER_PARAM_FLIP_Y,
            &mut flip as *mut _ as *mut c_void,
        ),
        param(
            mpv_render_param_type_MPV_RENDER_PARAM_INVALID,
            std::ptr::null_mut(),
        ),
    ];
    unsafe { mpv_render_context_render(ctx, params.as_mut_ptr()) >= 0 }
}

pub(crate) fn skip_frame(ctx: *mut mpv_render_context) {
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
