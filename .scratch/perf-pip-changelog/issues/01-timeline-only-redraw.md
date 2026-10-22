# 01 — Time updates redraw only the timeline

**What to build:** During playback the whole Player re-renders about 4×/s: the time-position update is throttled by the redraw step, but then falls through to a full notify, rebuilding the top and bottom chrome, the menus and the Now Playing sync. Make a time update refresh only the timeline (seek bar, elapsed time, "ends at"), for example by giving the timeline its own entity. The PiP window currently re-renders on every Player notify; make it re-render only when the state it mirrors (paused) actually changes.

**Blocked by:** None — can start immediately.

**Status:** ready-for-agent

- [ ] With the controls visible, the seek bar, elapsed time and "ends at" still advance smoothly
- [ ] A time update no longer re-renders the top/bottom chrome, menus or Now Playing sync (checked by counting Player renders during playback)
- [ ] The PiP window re-renders on pause/play changes, not on every Player update; its seek bar still advances
- [ ] Scrubbing, the scrub preview and the up-next card still behave as before
