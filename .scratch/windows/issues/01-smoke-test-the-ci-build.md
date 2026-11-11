# 01 — Smoke-test the CI build on a Windows PC

**What to build:** Nothing. Run the `tbis-windows-x86_64` artifact from the Windows workflow by hand and record what works before any Windows-specific code lands. Run `tbis.exe` from a terminal so panics stay on screen (a console window opens anyway until ticket 02).

**Blocked by:** None — can start immediately.

**Status:** done (run once by hand; playback failure fixed)

- [x] The window opens and the UI draws (gpui DirectX renderer, fonts, theme)
- [x] Login works (session survival across relaunch not reported)
- [x] Pages load far enough to start playback (poster/backdrop check not reported)
- [x] Starting playback fails without crashing. The "not supported yet" stub is gone since ticket 06 landed
- [x] Every failure is written down here with its first error line

## Failures

- Playback: `cannot start player. mpv osc: option not found (-5)`. The Windows libmpv is built with `lua=disabled` (`scripts/windows-deps.json`), and mpv registers the `osc` option only when Lua is built in. Fixed in `src/mpv/mod.rs`: `Handle::init` now skips `MPV_ERROR_OPTION_NOT_FOUND` for `osc` only. Re-test playback on the next CI artifact.
