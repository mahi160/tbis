//! Keeps the screen awake while alive via `org.freedesktop.ScreenSaver.Inhibit` (GNOME,
//! KDE, Xfce, Cinnamon, MATE...). With `vo=libmpv` mpv has no window of its own, so its
//! `stop-screensaver` has nothing to inhibit.
//!
//! D-Bus runs on a worker thread, so a slow or missing session bus never stalls the UI.
//! The inhibit belongs to that thread's bus connection: it ends on drop, and on a crash.

use std::sync::mpsc;

use zbus::blocking::Connection;

const NAME: &str = "org.freedesktop.ScreenSaver";
const PATH: &str = "/org/freedesktop/ScreenSaver";

/// Dropping it releases the inhibit.
pub struct Awake(mpsc::Sender<()>);

impl Awake {
    pub fn new() -> Self {
        let (release, released) = mpsc::channel();
        std::thread::spawn(move || {
            let Ok(bus) = Connection::session() else {
                return;
            };
            let cookie = bus
                .call_method(
                    Some(NAME),
                    PATH,
                    Some(NAME),
                    "Inhibit",
                    &("tbis", "Playing video"),
                )
                .and_then(|reply| reply.body().deserialize::<u32>());
            let Ok(cookie) = cookie else {
                return;
            };
            let _ = released.recv(); // Err once `Awake` is dropped
            let _ = bus.call_method(Some(NAME), PATH, Some(NAME), "UnInhibit", &(cookie,));
        });
        Self(release)
    }
}
