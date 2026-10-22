# 04 — Playback Info reads mpv off the UI thread

**What to build:** While the Playback info panel is open, a 1-second ticker reads about 25 mpv properties synchronously on the UI thread, each taking mpv's core lock. Gather those values off the UI thread and hand the result back for rendering.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] With the panel open, no mpv property reads happen on the UI thread
- [ ] The panel shows the same values as before and still refreshes about once a second
- [ ] Closing the panel stops the refresh
