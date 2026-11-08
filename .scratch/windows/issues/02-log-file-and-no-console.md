# 02 — tbis.log next to the exe; no console window

**What to build:** Windows builds write a `tbis.log` next to `tbis.exe`: mpv log messages (`mpv_request_log_messages`, level warn or higher), video surface errors, panics (panic hook), and per-session frame stats (frames drawn, dropped, decoder in use). Release builds set `#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]`, so no console window opens; the log replaces it. Truncated on each launch so it can't grow without bound.

**Blocked by:** None — can start immediately.

**Status:** done

- [ ] Launching the release exe opens no console window
- [ ] A forced panic ends up in `tbis.log` with its message and location
- [ ] mpv warnings and errors appear in `tbis.log` during playback
- [ ] Closing the Player appends one frame-stats line
- [ ] macOS and Linux behaviour is unchanged
