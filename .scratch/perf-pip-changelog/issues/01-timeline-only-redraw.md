# 01 — Time updates redraw only the timeline

**What to build:** During playback the whole Player re-renders about 4×/s: the time-position update is throttled by the redraw step, but then falls through to a full notify, rebuilding the top and bottom chrome, the menus and the Now Playing sync. Make a time update refresh only the timeline (seek bar, elapsed time, "ends at"), for example by giving the timeline its own entity. The PiP window currently re-renders on every Player notify; make it re-render only when the state it mirrors (paused) actually changes.

**Blocked by:** None — can start immediately.

**Status:** done (runtime unverified). A separate timeline entity would not help: gpui marks every ancestor view dirty when a child notifies, so the Player would still re-render. Instead, with the controls hidden a time update re-renders only when the skip button or up-next countdown changes; with them shown the chrome is built only when visible.

- [ ] With the controls visible, the seek bar, elapsed time and "ends at" still advance smoothly
- [ ] A time update no longer re-renders the top/bottom chrome, menus or Now Playing sync (checked by counting Player renders during playback)
- [ ] The PiP window re-renders on pause/play changes, not on every Player update; its seek bar still advances
- [ ] Scrubbing, the scrub preview and the up-next card still behave as before
