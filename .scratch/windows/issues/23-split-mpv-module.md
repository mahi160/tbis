# 23: Split the mpv module below 1k lines

**What to build:** The mpv render thread and event thread move into their own modules so the main mpv module is back under 1,000 lines. Pure restructuring, no behaviour change.

**Blocked by:** 17: Make video drawing fallible on every OS; 19: Remove Windows branches from mpv event logging

**Status:** ready-for-agent

- [ ] mpv main module under 1,000 lines
- [ ] Render and event threads each in their own module
- [ ] No behaviour change; existing tests pass; `cargo check` on macOS and Windows CI pass

Work on the `feat/windows` branch. Source: thermo-nuclear code quality review of `feat/windows` against merge-base `60b2ef8`.
