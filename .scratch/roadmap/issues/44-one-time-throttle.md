# 44 — One time throttle, not two

**What to build:** mpv's position reaches the UI through two throttles. The event thread already lets through at most one `time-pos` every 250 ms (`TimeThrottle` in `src/mpv/events.rs`). The Player then drops any update that moved less than 0.1 s (`REDRAW_STEP`). At every speed the Player offers (0.5× and up) the second one never triggers during playback; it only swallows frame-steps while paused, so the seek bar and Now Playing miss them. Delete the second throttle.

**Model:** haiku

**Blocked by:** None.

**Status:** done (runtime unverified). Removed the Player's second position throttle; mpv's event-thread throttle remains unchanged. Formatting and warning-denying Clippy pass; the final test batch has 94 passing tests after correcting the new reset test's key-order assertion with owner approval. Cargo build passed; Windows CI and runtime checks were not run for these changes.

**Read first:** `src/player/playback.rs` (the constants at the top, `struct Playback`, `Playback::new`, `fn update_time`, the `FileEvent::TimePos` arm of `fn on_mpv`).

## Steps

All in `src/player/playback.rs`:

1. Delete the constant `REDRAW_STEP` and its doc comment.
2. In `struct Playback`, delete the field `shown_time` and its doc comment. In `Playback::new`, delete `shown_time: 0.,`.
3. Delete `fn update_time` and its doc comment.
4. In the `FileEvent::TimePos(time)` arm of `fn on_mpv`, replace:

   ```rust
   if !self.update_time(time) {
       return;
   }
   ```

   with:

   ```rust
   self.playback.time = time;
   ```

5. On `buffered_until` in `struct Playback`, change the doc comment's ending "read on each shown-time step" to "read on each time update".

Do not change `src/mpv/events.rs`.

## Done when

- [ ] `rg -n "REDRAW_STEP|shown_time|update_time" src` prints nothing
- [ ] While paused, pressing `.` (next frame) several times moves the time label once a second boundary is crossed, and the seek bar follows

**Check:** the gate in `PLAN.md`.
