# 13 — Buffered range on the timeline

**What to build:** The timeline shows only the playhead. Show how far ahead the stream is buffered as a lighter bar behind the seek slider, so a user on a slow link sees why a seek is instant or not.

**Model:** sonnet

**Blocked by:** None.

**Status:** ready-for-agent

**Read first:** `src/components/video.rs` (`pub fn timeline`), `src/player/controls/chrome.rs` (`fn bottom_chrome`), `src/player/controls.rs` (`fn playback_info`, which reads `demuxer-cache-duration` through a `num(...)` helper), `src/player/playback.rs` (`fn update_time`, `fn on_mpv`), `src/design.rs` (`overlay`, `radius`, `sp`).

## Steps

1. Add `buffered_until: f64` to the playback state struct that holds `time` and `duration` (find it from `self.playback.time`). Reset it to `0.` wherever `time` is reset for a new load.
2. In `fn on_mpv`, in the `FileEvent::TimePos` arm, when `update_time` returns true (the shown time changed), read mpv's `demuxer-cache-duration` (seconds buffered ahead) the same way `playback_info` reads it, and set `buffered_until = time + ahead`. Reading on the shown-time step keeps it to a few reads per second.
3. In `src/components/video.rs`, give `timeline` a new parameter `buffered: f32` (fraction of the duration, 0 to 1). Inside the `relative()` wrapper around the slider, add as the first child:

   ```rust
   div()
       .absolute()
       .left_0()
       .top(relative(0.5))
       .h(px(1.))
       .w(relative(buffered.clamp(0., 1.)))
       .bg(white(0.35))
   ```

   Check how the slider track is positioned and use the same vertical centre; use `design` tokens for any size other than 0 or 1 px. Read the file's imports for `white` and `relative`.
4. In `fn bottom_chrome`, pass `(self.playback.buffered_until / self.playback.duration.max(1.)) as f32`.
5. Update the other callers of `timeline` (`rg -n "timeline\(" src`), such as PiP, passing their own value or `0.`.

## Done when

- [ ] A lighter bar extends from the left edge to the buffered position and grows during playback
- [ ] It never passes the right edge and resets on a new Episode
- [ ] Dragging the slider still works; the bar does not catch clicks

**Check:** the gate in `PLAN.md`, then play a remote or capped stream and watch the bar grow.
