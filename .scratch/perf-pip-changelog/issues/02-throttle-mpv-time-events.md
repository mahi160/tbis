# 02 — Thin out mpv time updates on the event thread

**What to build:** mpv reports the time position about once per video frame, and every report crosses to the main thread, where most are thrown away by the redraw step. Throttle on mpv's event thread so the main thread gets at most about 4 time updates a second, while seeks and end-of-file still show the exact position.

**Blocked by:** None — can start immediately.

**Status:** done (runtime unverified)

- [ ] During normal playback the main thread receives at most ~4 time updates per second
- [ ] After a seek (keys, scrub release, chapter jump) the timeline shows the new position without waiting a full interval
- [ ] Progress reports, the up-next card timing and skip-segment detection still work
