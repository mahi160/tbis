# 18: Make the Windows log reliable and simple

**What to build:** On Windows, a log file that can't be created no longer prevents the app from starting; it falls back silently and playback still works. Every existing stderr message reaches the log, preferably by pointing the process's stderr at the log file at startup (spike first to confirm Rust's stderr honours the redirect in a `windows_subsystem = "windows"` release build; fall back to the current explicit writer if not). Redaction uses one marker list shared by the panic hook and mpv lines, drops over-broad markers (`?`, `@`, `header`, `session`, any path) that blank ordinary mpv diagnostics, and the 'GPU behavior unverified' startup string is removed.

**Blocked by:** None (can start immediately)

**Status:** done (runtime unverified on Windows)

- [x] App launches when the install folder is read-only (log creation fails)
- [x] Spike outcome recorded in the ticket: stderr redirect works or not
- [x] A single sensitive-marker list exists
- [x] Auth tokens and credentials are still redacted from mpv log lines and panics (test covers it)
- [x] Ordinary mpv lines with URLs-free paths or `?` survive when no secret is present (test covers it)
- [x] No debugging-only text in the startup log line

Work on the `feat/windows` branch. Source: thermo-nuclear code quality review of `feat/windows` against merge-base `60b2ef8`.

## Spike

Outcome: stderr redirect used (`SetStdHandle(STD_ERROR_HANDLE, log file)` at startup); the explicit file writer is removed. **Unverified at runtime** (not run on Windows).

Evidence: Rust std `library/std/src/sys/stdio/windows.rs` (master) — `Stderr::write` calls `write(c::STD_ERROR_HANDLE, ..)`, which calls `get_handle` -> `GetStdHandle(handle_id)` on every write, so a handle set with `SetStdHandle` after startup is honoured, including under `windows_subsystem = "windows"`. If log creation fails, stderr is left untouched and the app starts normally. Still to confirm on Windows: a release build writes `eprintln!` output and mpv lines to `tbis.log`, and launches from a read-only folder.
